#!/usr/bin/env sh
# shellcheck shell=sh disable=SC3043

print_usage() {
  local program version author default_dest default_platform
  program="$1"
  version="$2"
  author="$3"
  bin="$4"
  default_dest="$5"
  default_platform="$6"

  need_cmd sed

  echo "$program $version

    Installs a binary release of $bin for supported platforms

    USAGE:
        $program [OPTIONS] [--]

    OPTIONS:
        -h, --help                Prints help information
        -d, --destination=<DEST>  Destination directory for installation
                                  [default: $default_dest]
        -p, --platform=<PLATFORM> Platform type to install
                                  [examples: linux-x86_64, darwin-x86_64]
                                  [default: $default_platform]
        -r, --release=<RELEASE>   Release version
                                  [examples: latest, 1.2.3, nightly]
                                  [default: latest]
        -V, --version             Prints version information

    EXAMPLES:
        # Installs the latest release into \`\$$HOME/bin\`
        $program

    AUTHOR:
        $author
    " | sed 's/^ \{1,4\}//g'
}

main() {
  set -eu
  if [ -n "${DEBUG:-}" ]; then set -v; fi
  if [ -n "${TRACE:-}" ]; then set -xv; fi

  local program version author
  program="install.sh"
  version="0.2.0"
  author="Fletcher Nichol <fnichol@nichol.ca>"

  local gh_repo bin
  gh_repo="fnichol/names"
  bin="names"

  parse_cli_args "$program" "$version" "$author" "$bin" "$@"
  local dest platform release
  dest="$DEST"
  platform="$PLATFORM"
  release="$RELEASE"
  unset DEST PLATFORM RELEASE

  need_cmd basename

  setup_cleanups
  setup_traps trap_cleanups

  local initial_dir
  initial_dir="$PWD"

  section "Downloading, verifying, and installing '$bin'"

  if [ "$release" = "latest" ]; then
    info_start "Determining latest release for '$bin'"
    release="$(latest_release "$gh_repo")" \
      || die "Could not find latest release for '$bin' in repo '$gh_repo'"
    info_end
  fi

  local asset_url
  info_start \
    "Determining asset URL for '$bin' release '$release' on '$platform'"
  asset_url="$(asset_url "$gh_repo" "$bin" "$release" "$platform")" \
    || die "Unsupported platform '$platform' for '$bin' release '$release'"
  info_end

  local tmpdir
  tmpdir="$(mktemp_directory)"
  cleanup_directory "$tmpdir"

  local asset
  asset="$(basename "$asset_url")"
  section "Downloading assets for '$asset'"
  download "$asset_url" "$tmpdir/$asset"
  download "$asset_url.md5" "$tmpdir/$asset.md5"
  download "$asset_url.sha256" "$tmpdir/$asset.sha256"

  section "Verifying '$asset'"
  cd "$tmpdir"
  verify_asset_md5 "$asset" || die "Failed to verify MD5 checksum"
  verify_asset_sha256 "$asset" || die "Failed to verify SHA256 checksum"

  section "Installing '$asset'"
  extract_asset "$asset" || die "Failed to extract asset"
  cd "$initial_dir"
  local asset_bin
  asset_bin="${asset%%.tar.gz}"
  asset_bin="${asset_bin%%.zip}"
  install_bin "$tmpdir/$asset_bin" "$dest/$bin"

  section "Installation of '$bin' release '$release' complete"
}

parse_cli_args() {
  local program version author bin
  program="$1"
  shift
  version="$1"
  shift
  author="$1"
  shift
  bin="$1"
  shift

  need_cmd id
  need_cmd uname
  need_cmd tr

  local os_type cpu_type plat dest
  os_type="$(uname -s | tr '[:upper:]' '[:lower:]')"
  cpu_type="$(uname -m | tr '[:upper:]' '[:lower:]')"
  plat="$os_type-$cpu_type"
  if [ "$(id -u)" -eq 0 ]; then
    dest="/usr/local/bin"
  else
    dest="$HOME/bin"
  fi

  DEST="$dest"
  PLATFORM="$plat"
  RELEASE="latest"

  OPTIND=1
  while getopts "d:hp:r:V-:" arg; do
    case "$arg" in
      d)
        DEST="$OPTARG"
        ;;
      h)
        print_usage "$program" "$version" "$author" "$bin" "$dest" "$plat"
        exit 0
        ;;
      p)
        PLATFORM="$OPTARG"
        ;;
      r)
        RELEASE="$OPTARG"
        ;;
      V)
        print_version "$program" "$version" "$plat"
        exit 0
        ;;
      -)
        long_optarg="${OPTARG#*=}"
        case "$OPTARG" in
          destination=?*)
            DEST="$long_optarg"
            ;;
          destination*)
            print_usage "$program" "$version" "$author" "$bin" "$dest" "$plat" >&2
            die "missing required argument for --$OPTARG option"
            ;;
          help)
            print_usage "$program" "$version" "$author" "$bin" "$dest" "$plat"
            exit 0
            ;;
          platform=?*)
            PLATFORM="$long_optarg"
            ;;
          platform*)
            print_usage "$program" "$version" "$author" "$bin" "$dest" "$plat" >&2
            die "missing required argument for --$OPTARG option"
            ;;
          release=?*)
            RELEASE="$long_optarg"
            ;;
          release*)
            print_usage "$program" "$version" "$author" "$bin" "$dest" "$plat" >&2
            die "missing required argument for --$OPTARG option"
            ;;
          version)
            print_version "$program" "$version" "true"
            exit 0
            ;;
          '')
            # "--" terminates argument processing
            break
            ;;
          *)
            print_usage "$program" "$version" "$author" "$bin" "$dest" "$plat" >&2
            die "invalid argument --$OPTARG"
            ;;
        esac
        ;;
      \?)
        print_usage "$program" "$version" "$author" "$bin" "$dest" "$plat" >&2
        die "invalid argument; arg=-$OPTARG"
        ;;
    esac
  done
  shift "$((OPTIND - 1))"
}

latest_release() {
  local gh_repo
  gh_repo="$1"

  need_cmd awk

  local tmpfile
  tmpfile="$(mktemp_file)"
  cleanup_file "$tmpfile"

  download \
    "https://api.github.com/repos/$gh_repo/releases/latest" \
    "$tmpfile" \
    >/dev/null
  awk '
    BEGIN { FS="\""; RS="," }
    $2 == "tag_name" { sub(/^v/, "", $4); print $4 }
  ' "$tmpfile"
}

asset_url() {
  local repo bin release platform
  repo="$1"
  bin="$2"
  release="$3"
  platform="$4"
  if [ "$release" != "nightly" ]; then
    release="v$release"
  fi

  need_cmd awk

  local base_url manifest_url
  base_url="https://github.com/$repo/releases/download/$release"
  manifest_url="$base_url/$bin.manifest.txt"

  local tmpfile
  tmpfile="$(mktemp_file)"
  cleanup_file "$tmpfile"

  download "$manifest_url" "$tmpfile" >/dev/null
  awk -v platform="$platform" -v base_url="$base_url" '
    $1 == platform { print base_url "/" $2; found = 1; exit }
    END { if (!found) { exit 1 } }
  ' "$tmpfile" \
    || {
      echo >&2
      warn "Cannot find platform entry for '$platform' in $manifest_url" >&2
      return 1
    }
}

verify_asset_sha256() {
  local asset
  asset="$1"

  need_cmd uname

  info "Verifying SHA256 checksum"
  case "$(uname -s)" in
    FreeBSD)
      if check_cmd sha256; then
        need_cmd awk
        indent sha256 -c "$(awk '{print $1}' "$asset.sha256")" "$asset"
      fi
      ;;
    Linux)
      if check_cmd sha256sum; then
        indent sha256sum -c "$asset.sha256"
      fi
      ;;
    Darwin)
      if check_cmd shasum; then
        indent shasum -c "$asset.sha256"
      fi
      ;;
  esac
}

verify_asset_md5() {
  local asset
  asset="$1"

  need_cmd uname

  info "Verifying MD5 checksum"
  case "$(uname -s)" in
    FreeBSD)
      if check_cmd md5; then
        need_cmd awk
        indent md5 -c "$(awk '{print $1}' "$asset.md5")" "$asset"
      fi
      ;;
    Linux)
      if check_cmd md5sum; then
        indent md5sum -c "$asset.md5"
      fi
      ;;
    Darwin)
      if check_cmd md5; then
        need_cmd awk
        local expected actual
        expected="$(awk '{ print $1 }' "$asset.md5")"
        actual="$(md5 "$asset" | awk '{ print $NF }')"
        if [ "$expected" = "$actual" ]; then
          indent echo "$asset: OK"
        else
          indent echo "$asset: FAILED"
          indent echo "md5: WARNING: 1 computed checksum did NOT match"
          return 1
        fi
      fi
      ;;
  esac
}

extract_asset() {
  local asset
  asset="$1"

  info "Extracting $asset"
  case "$asset" in
    *.tar.gz)
      need_cmd tar
      need_cmd zcat
      zcat "$asset" | indent tar xvf -
      ;;
    *.zip)
      need_cmd unzip
      indent unzip "$asset"
      ;;
  esac
}

install_bin() {
  local src dest
  src="$1"
  dest="$2"

  need_cmd dirname
  need_cmd install
  need_cmd mkdir

  info_start "Installing '$dest'"
  mkdir -p "$(dirname "$dest")"
  install -p -m 755 "$src" "$dest"
  info_end
}

# BEGIN: libsh.sh

#
# Copyright 2019 Fletcher Nichol and/or applicable contributors.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license (see
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option. This
# file may not be copied, modified, or distributed except according to those
# terms.
#
# libsh.sh
# --------
# project: https://github.com/fnichol/libsh
# author: Fletcher Nichol <fnichol@nichol.ca>
# version: 0.9.0
# distribution: libsh.full-minified.sh
# commit-hash: e155f96bc281060342da4a19c26bf896f47de09c
# commit-date: 2021-04-14
# artifact: https://github.com/fnichol/libsh/releases/download/v0.9.0/libsh.full.sh
# source: https://github.com/fnichol/libsh/tree/v0.9.0
# archive: https://github.com/fnichol/libsh/archive/v0.9.0.tar.gz
#
if [ -n "${KSH_VERSION:-}" ]; then
  eval "local() { return 0; }"
fi
# shellcheck disable=SC2120
mktemp_directory() {
  need_cmd mktemp
  if [ -n "${1:-}" ]; then
    mktemp -d "$1/tmp.XXXXXX"
  else
    mktemp -d 2>/dev/null || mktemp -d -t tmp
  fi
}
# shellcheck disable=SC2120
mktemp_file() {
  need_cmd mktemp
  if [ -n "${1:-}" ]; then
    mktemp "$1/tmp.XXXXXX"
  else
    mktemp 2>/dev/null || mktemp -t tmp
  fi
}
trap_cleanup_files() {
  set +e
  if [ -n "${__CLEANUP_FILES__:-}" ] && [ -f "$__CLEANUP_FILES__" ]; then
    local _file
    while read -r _file; do
      rm -f "$_file"
    done <"$__CLEANUP_FILES__"
    unset _file
    rm -f "$__CLEANUP_FILES__"
  fi
}
need_cmd() {
  if ! check_cmd "$1"; then
    die "Required command '$1' not found on PATH"
  fi
}
trap_cleanups() {
  set +e
  trap_cleanup_directories
  trap_cleanup_files
}
print_version() {
  local _program _version _verbose _sha _long_sha _date
  _program="$1"
  _version="$2"
  _verbose="${3:-false}"
  _sha="${4:-}"
  _long_sha="${5:-}"
  _date="${6:-}"
  if [ -z "$_sha" ] || [ -z "$_long_sha" ] || [ -z "$_date" ]; then
    if check_cmd git \
      && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
      if [ -z "$_sha" ]; then
        _sha="$(git show -s --format=%h)"
        if ! git diff-index --quiet HEAD --; then
          _sha="${_sha}-dirty"
        fi
      fi
      if [ -z "$_long_sha" ]; then
        _long_sha="$(git show -s --format=%H)"
        case "$_sha" in
          *-dirty) _long_sha="${_long_sha}-dirty" ;;
        esac
      fi
      if [ -z "$_date" ]; then
        _date="$(git show -s --format=%ad --date=short)"
      fi
    fi
  fi
  if [ -n "$_sha" ] && [ -n "$_date" ]; then
    echo "$_program $_version ($_sha $_date)"
  else
    echo "$_program $_version"
  fi
  if [ "$_verbose" = "true" ]; then
    echo "release: $_version"
    if [ -n "$_long_sha" ]; then
      echo "commit-hash: $_long_sha"
    fi
    if [ -n "$_date" ]; then
      echo "commit-date: $_date"
    fi
  fi
  unset _program _version _verbose _sha _long_sha _date
}
warn() {
  case "${TERM:-}" in
    *term | alacritty | rxvt | screen | screen-* | tmux | tmux-* | xterm-*)
      printf -- "\033[1;31;40m!!! \033[1;37;40m%s\033[0m\n" "$1"
      ;;
    *)
      printf -- "!!! %s\n" "$1"
      ;;
  esac
}
section() {
  case "${TERM:-}" in
    *term | alacritty | rxvt | screen | screen-* | tmux | tmux-* | xterm-*)
      printf -- "\033[1;36;40m--- \033[1;37;40m%s\033[0m\n" "$1"
      ;;
    *)
      printf -- "--- %s\n" "$1"
      ;;
  esac
}
setup_cleanup_directories() {
  if [ -z "${__CLEANUP_DIRECTORIES__:-}" ]; then
    __CLEANUP_DIRECTORIES__="$(mktemp_file)"
    if [ -z "$__CLEANUP_DIRECTORIES__" ]; then
      return 1
    fi
    export __CLEANUP_DIRECTORIES__
  fi
}
setup_cleanup_files() {
  if [ -z "${__CLEANUP_FILES__:-}" ]; then
    __CLEANUP_FILES__="$(mktemp_file)"
    if [ -z "$__CLEANUP_FILES__" ]; then
      return 1
    fi
    export __CLEANUP_FILES__
  fi
}
setup_cleanups() {
  setup_cleanup_directories
  setup_cleanup_files
}
setup_traps() {
  local _sig
  for _sig in HUP INT QUIT ALRM TERM; do
    trap "
      $1
      trap - $_sig EXIT
      kill -s $_sig "'"$$"' "$_sig"
  done
  if [ -n "${ZSH_VERSION:-}" ]; then
    eval "zshexit() { eval '$1'; }"
  else
    # shellcheck disable=SC2064
    trap "$1" EXIT
  fi
  unset _sig
}
trap_cleanup_directories() {
  set +e
  if [ -n "${__CLEANUP_DIRECTORIES__:-}" ] \
    && [ -f "$__CLEANUP_DIRECTORIES__" ]; then
    local _dir
    while read -r _dir; do
      rm -rf "$_dir"
    done <"$__CLEANUP_DIRECTORIES__"
    unset _dir
    rm -f "$__CLEANUP_DIRECTORIES__"
  fi
}
check_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    return 1
  fi
}
cleanup_directory() {
  setup_cleanup_directories
  echo "$1" >>"$__CLEANUP_DIRECTORIES__"
}
cleanup_file() {
  setup_cleanup_files
  echo "$1" >>"$__CLEANUP_FILES__"
}
die() {
  case "${TERM:-}" in
    *term | alacritty | rxvt | screen | screen-* | tmux | tmux-* | xterm-*)
      printf -- "\n\033[1;31;40mxxx \033[1;37;40m%s\033[0m\n\n" "$1" >&2
      ;;
    *)
      printf -- "\nxxx %s\n\n" "$1" >&2
      ;;
  esac
  exit 1
}
download() {
  local _url _dst _code _orig_flags
  _url="$1"
  _dst="$2"
  need_cmd sed
  if check_cmd curl; then
    info "Downloading $_url to $_dst (curl)"
    _orig_flags="$-"
    set +e
    curl -sSfL "$_url" -o "$_dst"
    _code="$?"
    set "-$(echo "$_orig_flags" | sed s/s//g)"
    if [ $_code -eq 0 ]; then
      unset _url _dst _code _orig_flags
      return 0
    else
      local _e
      _e="curl failed to download file, perhaps curl doesn't have"
      _e="$_e SSL support and/or no CA certificates are present?"
      warn "$_e"
      unset _e
    fi
  fi
  if check_cmd wget; then
    info "Downloading $_url to $_dst (wget)"
    _orig_flags="$-"
    set +e
    wget -q -O "$_dst" "$_url"
    _code="$?"
    set "-$(echo "$_orig_flags" | sed s/s//g)"
    if [ $_code -eq 0 ]; then
      unset _url _dst _code _orig_flags
      return 0
    else
      local _e
      _e="wget failed to download file, perhaps wget doesn't have"
      _e="$_e SSL support and/or no CA certificates are present?"
      warn "$_e"
      unset _e
    fi
  fi
  if check_cmd ftp; then
    info "Downloading $_url to $_dst (ftp)"
    _orig_flags="$-"
    set +e
    ftp -o "$_dst" "$_url"
    _code="$?"
    set "-$(echo "$_orig_flags" | sed s/s//g)"
    if [ $_code -eq 0 ]; then
      unset _url _dst _code _orig_flags
      return 0
    else
      local _e
      _e="ftp failed to download file, perhaps ftp doesn't have"
      _e="$_e SSL support and/or no CA certificates are present?"
      warn "$_e"
      unset _e
    fi
  fi
  unset _url _dst _code _orig_flags
  warn "Downloading requires SSL-enabled 'curl', 'wget', or 'ftp' on PATH"
  return 1
}
indent() {
  local _ecfile _ec _orig_flags
  need_cmd cat
  need_cmd rm
  need_cmd sed
  _ecfile="$(mktemp_file)"
  _orig_flags="$-"
  set +e
  {
    "$@" 2>&1
    echo "$?" >"$_ecfile"
  } | sed 's/^/       /'
  set "-$(echo "$_orig_flags" | sed s/s//g)"
  _ec="$(cat "$_ecfile")"
  rm -f "$_ecfile"
  unset _ecfile _orig_flags
  return "${_ec:-5}"
}
info() {
  case "${TERM:-}" in
    *term | alacritty | rxvt | screen | screen-* | tmux | tmux-* | xterm-*)
      printf -- "\033[1;36;40m  - \033[1;37;40m%s\033[0m\n" "$1"
      ;;
    *)
      printf -- "  - %s\n" "$1"
      ;;
  esac
}
info_end() {
  case "${TERM:-}" in
    *term | alacritty | rxvt | screen | screen-* | tmux | tmux-* | xterm-*)
      printf -- "\033[1;37;40m%s\033[0m\n" "done."
      ;;
    *)
      printf -- "%s\n" "done."
      ;;
  esac
}
info_start() {
  case "${TERM:-}" in
    *term | alacritty | rxvt | screen | screen-* | tmux | tmux-* | xterm-*)
      printf -- "\033[1;36;40m  - \033[1;37;40m%s ... \033[0m" "$1"
      ;;
    *)
      printf -- "  - %s ... " "$1"
      ;;
  esac
}

# END: libsh.sh

main "$@"
