<h1 align="center">
  <br/>
  {{crate}}
  <br/>
</h1>

<h4 align="center">
  Random name generator for Rust
</h4>

|                  |                                                                                          |
| ---------------: | ---------------------------------------------------------------------------------------- |
|               CI | [![CI Status][badge-ci-overall]][ci]<br /> [![Bors enabled][badge-bors]][bors-dashboard] |
|   Latest Version | [![Latest version][badge-version]][crate]                                                |
|    Documentation | [![Documentation][badge-docs]][docs]                                                     |
|  Crate Downloads | [![Crate downloads][badge-crate-dl]][crate]                                              |
| GitHub Downloads | [![Github downloads][badge-github-dl]][github-releases]                                  |
|     Docker Pulls | [![Docker pulls][badge-docker-pulls]][docker]                                            |
|          License | [![Crate license][badge-license]][github]                                                |

<details>
<summary><strong>Table of Contents</strong></summary>

<!-- toc -->

</details>

## CLI

### Usage

Simple! Run without any parameters, you get a name:

```console
> names
selfish-change
```

Need more? Tell it how many:

```console
> names 10
rustic-flag
nondescript-crayon
picayune-map
elderly-cough
skinny-jeans
neat-rock
aware-sponge
psychotic-coast
brawny-event
tender-oatmeal
```

Not random enough? How about adding a 4-number pad:

```console
> names --number 5
imported-rod-9680
thin-position-2344
hysterical-women-5647
volatile-pen-9210
diligent-grip-4520
```

If you're ever confused, at least there's help:

```console
> names --help
names 0.11.0
Fletcher Nichol <fnichol@nichol.ca>

A random name generator with results like "delirious-pail"

USAGE:
    names [FLAGS] [AMOUNT]

ARGS:
    <AMOUNT>    Number of names to generate [default: 1]

FLAGS:
    -h, --help       Prints help information
    -n, --number     Adds a random number to the name(s)
    -V, --version    Prints version information
```

### Installation

#### install.sh (Pre-Built Binaries)

An installer is provided at <https://fnichol.github.io/{{crate}}/install.sh>
which installs a suitable pre-built binary for common systems such as Linux,
macOS, Windows, and FreeBSD. It can be downloaded and run locally or piped into
a shell interpreter in the "curl-bash" style as shown below. Note that if you're
opposed to this idea, feel free to check some of the alternatives below.

To install the latest release for your system into `$HOME/bin`:

```sh
curl -sSf https://fnichol.github.io/{{crate}}/install.sh | sh
```

When the installer is run as `root` the installation directory defaults to
`/usr/local/bin`:

```sh
curl -sSf https://fnichol.github.io/{{crate}}/install.sh | sudo sh
```

A [nightly] release built from `HEAD` of the main branch is available which can
also be installed:

```sh
curl -sSf https://fnichol.github.io/{{crate}}/install.sh \
    | sh -s -- --release=nightly
```

For a full set of options, check out the help usage with:

```sh
curl -sSf https://fnichol.github.io/{{crate}}/install.sh | sh -s -- --help
```

#### GitHub Releasees (Pre-Built Binaries)

Each release comes with binary artifacts published in [GitHub
Releases][github-releases]. The `install.sh` program downloads its artifacts
from this location so this serves as a manual alternative. Each artifact ships
with MD5 and SHA256 checksums to help verify the artifact on a target system.

#### Docker Image

A minimal image ships with each release (including a [nightly] built version
from `HEAD` of the main branch) published to [Docker Hub][docker]. The
entrypoint invokes the binary directly, so any arguments to `docker run` will be
passed to the program. For example, to display the full help usage:

```sh
docker run fnichol/names --help
```

#### Cargo Install

If [Rust](https://rustup.rs/) is installed on your system, then installing with
Cargo is straight forward with:

```sh
cargo install {{crate}}
```

#### From Source

To install from source, you can clone the Git repository, build with Cargo and
copy the binary into a destination directory. This will build the project from
the latest commit on the main branch, which may not correspond to the latest
stable release:

```console
> git clone https://github.com/fnichol/{{crate}}.git
> cd {{crate}}
> cargo build --release
> cp ./target/release/{{crate}} /dest/path/
```

---

## Library

{{readme}}

## CI Status

### Build (main branch)

| Operating System | Target                        | Stable Rust                                                                     |
| ---------------: | ----------------------------- | ------------------------------------------------------------------------------- |
|          FreeBSD | `x86_64-unknown-freebsd`      | [![FreeBSD Build Status][badge-ci-build-x86_64-unknown-freebsd]][ci-staging]    |
|            Linux | `arm-unknown-linux-gnueabihf` | [![Linux Build Status][badge-ci-build-arm-unknown-linux-gnueabihf]][ci-staging] |
|            Linux | `aarch64-unknown-linux-gnu`   | [![Linux Build Status][badge-ci-build-aarch64-unknown-linux-gnu]][ci-staging]   |
|            Linux | `i686-unknown-linux-gnu`      | [![Linux Build Status][badge-ci-build-i686-unknown-linux-gnu]][ci-staging]      |
|            Linux | `i686-unknown-linux-musl`     | [![Linux Build Status][badge-ci-build-i686-unknown-linux-musl]][ci-staging]     |
|            Linux | `x86_64-unknown-linux-gnu`    | [![Linux Build Status][badge-ci-build-x86_64-unknown-linux-gnu]][ci-staging]    |
|            Linux | `x86_64-unknown-linux-musl`   | [![Linux Build Status][badge-ci-build-x86_64-unknown-linux-musl]][ci-staging]   |
|            macOS | `x86_64-apple-darwin`         | [![macOS Build Status][badge-ci-build-x86_64-apple-darwin]][ci-staging]         |
|          Windows | `x86_64-pc-windows-msvc`      | [![Windows Build Status][badge-ci-build-x86_64-pc-windows-msvc]][ci-staging]    |

### Test (main branch)

| Operating System | Stable Rust                                                               | Nightly Rust                                                                |
| ---------------: | ------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
|          FreeBSD | [![FreeBSD Stable Test Status][badge-ci-test-stable-freebsd]][ci-staging] | [![FreeBSD Nightly Test Status][badge-ci-test-nightly-freebsd]][ci-staging] |
|            Linux | [![Linux Stable Test Status][badge-ci-test-stable-linux]][ci-staging]     | [![Linux Nightly Test Status][badge-ci-test-nightly-linux]][ci-staging]     |
|            macOS | [![macOS Stable Test Status][badge-ci-test-stable-macos]][ci-staging]     | [![macOS Nightly Test Status][badge-ci-test-nightly-macos]][ci-staging]     |
|          Windows | [![Windows Stable Test Status][badge-ci-test-stable-windows]][ci-staging] | [![Windows Nightly Test Status][badge-ci-test-nightly-windows]][ci-staging] |

**Note**: The
[Minimum Supported Rust Version (MSRV)](https://github.com/rust-lang/rfcs/pull/2495)
is also tested and can be viewed in the [CI dashboard][ci-staging].

### Check (main branch)

|        | Status                                                |
| ------ | ----------------------------------------------------- |
| Lint   | [![Lint Status][badge-ci-check-lint]][ci-staging]     |
| Format | [![Format Status][badge-ci-check-format]][ci-staging] |

## Code of Conduct

This project adheres to the Contributor Covenant [code of
conduct][code-of-conduct]. By participating, you are expected to uphold this
code. Please report unacceptable behavior to fnichol@nichol.ca.

## Issues

If you have any problems with or questions about this project, please contact us
through a [GitHub issue][issues].

## Contributing

You are invited to contribute to new features, fixes, or updates, large or
small; we are always thrilled to receive pull requests, and do our best to
process them as fast as we can.

Before you start to code, we recommend discussing your plans through a [GitHub
issue][issues], especially for more ambitious contributions. This gives other
contributors a chance to point you in the right direction, give you feedback on
your design, and help you find out if someone else is working on the same thing.

## Release History

See the [changelog] for a full release history.

## Authors

Created and maintained by [Fletcher Nichol][fnichol] (<fnichol@nichol.ca>).

## License

Licensed under the MIT license ([LICENSE.txt][license]).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the MIT license, shall be
licensed as above, without any additional terms or conditions.

[badge-bors]: https://bors.tech/images/badge_small.svg
[badge-ci-build-x86_64-unknown-freebsd]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-x86_64-unknown-freebsd.tar.gz
[badge-ci-build-arm-unknown-linux-gnueabihf]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-arm-unknown-linux-gnueabihf.tar.gz
[badge-ci-build-aarch64-unknown-linux-gnu]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-aarch64-unknown-linux-gnu.tar.gz
[badge-ci-build-i686-unknown-linux-gnu]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-i686-unknown-linux-gnu.tar.gz
[badge-ci-build-i686-unknown-linux-musl]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-i686-unknown-linux-musl.tar.gz
[badge-ci-build-x86_64-unknown-linux-gnu]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-x86_64-unknown-linux-gnu.tar.gz
[badge-ci-build-x86_64-unknown-linux-musl]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-x86_64-unknown-linux-musl.tar.gz
[badge-ci-build-x86_64-apple-darwin]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-x86_64-apple-darwin.zip
[badge-ci-build-x86_64-pc-windows-msvc]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=build-bin-{{crate}}-x86_64-pc-windows-msvc.zip
[badge-ci-check-format]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=check&script=format
[badge-ci-check-lint]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=check&script=lint
[badge-ci-overall]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/main?style=flat-square
[badge-ci-test-nightly-freebsd]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-nightly-x86_64-unknown-freebsd
[badge-ci-test-nightly-linux]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-nightly-x86_64-unknown-linux-gnu
[badge-ci-test-nightly-macos]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-nightly-x86_64-apple-darwin
[badge-ci-test-nightly-windows]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-nightly-x86_64-pc-windows-msvc
[badge-ci-test-stable-freebsd]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-stable-x86_64-unknown-freebsd
[badge-ci-test-stable-linux]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-stable-x86_64-unknown-linux-gnu
[badge-ci-test-stable-macos]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-stable-x86_64-apple-darwin
[badge-ci-test-stable-windows]:
  https://img.shields.io/cirrus/github/fnichol/{{crate}}/staging?style=flat-square&task=test-stable-x86_64-pc-windows-msvc
[badge-crate-dl]:
  https://img.shields.io/crates/d/{{crate}}.svg?style=flat-square
[badge-docker-pulls]:
  https://img.shields.io/docker/pulls/fnichol/{{crate}}.svg?style=flat-square
[badge-docs]: https://docs.rs/{{crate}}/badge.svg?style=flat-square
[badge-github-dl]:
  https://img.shields.io/github/downloads/fnichol/{{crate}}/total.svg
[badge-license]: https://img.shields.io/crates/l/{{crate}}.svg?style=flat-square
[badge-version]: https://img.shields.io/crates/v/{{crate}}.svg?style=flat-square
[bors-dashboard]: https://app.bors.tech/repositories/37173
[changelog]: https://github.com/fnichol/{{crate}}/blob/main/CHANGELOG.md
[ci]: https://cirrus-ci.com/github/fnichol/{{crate}}
[ci-staging]: https://cirrus-ci.com/github/fnichol/{{crate}}/staging
[code-of-conduct]:
  https://github.com/fnichol/{{crate}}/blob/main/CODE_OF_CONDUCT.md
[commonmark]: https://commonmark.org/
[crate]: https://crates.io/crates/{{crate}}
[docker]: https://hub.docker.com/r/fnichol/{{crate}}
[docs]: https://docs.rs/{{crate}}
[fnichol]: https://github.com/fnichol
[github]: https://github.com/fnichol/{{crate}}
[github-releases]: https://github.com/fnichol/{{crate}}/releases
[issues]: https://github.com/fnichol/{{crate}}/issues
[license]: https://github.com/fnichol/{{crate}}/blob/main/LICENSE.txt
[nightly]: https://github.com/fnichol/{{crate}}/releases/tag/nightly
