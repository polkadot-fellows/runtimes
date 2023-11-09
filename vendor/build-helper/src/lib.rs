/*
Copyright ⓒ 2017 contributors.
Licensed under the MIT license (see LICENSE or <http://opensource.org
/licenses/MIT>) or the Apache License, Version 2.0 (see LICENSE of
<http://www.apache.org/licenses/LICENSE-2.0>), at your option. All
files in the project carrying such notice may not be copied, modified,
or distributed except according to those terms.
*/
/*!
This crate contains convenience methods for build scripts.

It provides easy access to the information Cargo provides to build scripts, as well as functions for emitting information back to Cargo.

## Compatibility

* `0.1`: Rust 1.13.

## Features

The following [optional features](http://doc.crates.io/manifest.html#the-features-section) are available:

* `nightly`: requires a nightly compiler, and introduces functionality that is *not* subject to normal stability guarantees.

  * `target::features` - target processor features.
  * `target::has_atomic` - target platform atomic types.
*/
#![allow(dead_code, unused_variables)]
pub extern crate semver;

use std::env;
use std::error::Error;
use std::fmt::{self, Display};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/**
Instructs Cargo to display a warning.

`warning!(..)` is shorthand for `build_helper::warning(&format!(..))`.
*/
#[macro_export]
macro_rules! warning {
    ($($args:tt)*) => {
        $crate::warning(format!($($args)*))
    };
}

/**
Reads and unwraps an environment variable.

This should only be used on variables which are *guaranteed* to be defined by Cargo.
*/
macro_rules! env_var {
    ($name:expr) => {
        ::std::env::var($name)
            .expect(concat!($name, " environment variable is not set"))
    };
}

/**
Reads and unwraps an environment variable as an `OsString`.

This should only be used on variables which are *guaranteed* to be defined by Cargo.
*/
macro_rules! env_var_os {
    ($name:expr) => {
        ::std::env::var_os($name)
            .expect(concat!($name, " environment variable is not set"))
    };
}

/**
Reads, unwraps, and parses an environment variable.

This should only be used on variables which are *guaranteed* to be defined by Cargo.
*/
macro_rules! parse_env_var {
    (try: $name:expr, $ty_desc:expr) => {
        {
            ::std::env::var($name)
                .ok()
                .map(|v| v.parse()
                    .expect(&format!(concat!($name, " {:?} is not a valid ", $ty_desc), v))
                )
        }
    };

    ($name:expr, $ty_desc:expr) => {
        {
            let v = env_var!($name);
            v.parse()
                .expect(&format!(concat!($name, " {:?} is not a valid ", $ty_desc), v))
        }
    };
}

/**
Error type indicating a string parse failed due to invalid input.
*/
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct InvalidInput(String);

impl InvalidInput {
    /**
    Returns the input which caused the error.
    */
    pub fn input(&self) -> &str {
        &self.0
    }
}

impl Display for InvalidInput {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "invalid input: {:?}", self.0)
    }
}

impl Error for InvalidInput {
    fn description(&self) -> &str {
        "invalid input"
    }
}

/**
Represents an atomic type supported by a target.
*/
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Atomic {
    /// Integers with the given number of `bits` are atomic.
    Integer { bits: u8 },

    /// Pointers are atomic.
    Pointer,
}

impl Display for Atomic {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Atomic::Integer { bits } => bits.fmt(fmt),
            Atomic::Pointer => "ptr".fmt(fmt),
        }
    }
}

impl FromStr for Atomic {
    type Err = InvalidInput;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "ptr" {
            Ok(Atomic::Pointer)
        } else if let Ok(bits) = s.parse() {
            Ok(Atomic::Integer { bits: bits })
        } else {
            Err(InvalidInput(s.into()))
        }
    }
}

/**
Represents the target platform's endianness.
*/
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Endianness {
    Big,
    Little,
}

impl Display for Endianness {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Endianness::Big => "big".fmt(fmt),
            Endianness::Little => "little".fmt(fmt),
        }
    }
}

impl FromStr for Endianness {
    type Err = InvalidInput;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "big" => Ok(Endianness::Big),
            "little" => Ok(Endianness::Little),
            _ => Err(InvalidInput(s.into()))
        }
    }
}

/**
Library linkage kind.
*/
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum LibKind {
    /// Link a static library.
    Static,

    /// Link a dynamic library.
    DyLib,

    /// Link an Objective-C framework.
    Framework,
}

impl Display for LibKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LibKind::Static => "static".fmt(fmt),
            LibKind::DyLib => "dylib".fmt(fmt),
            LibKind::Framework => "framework".fmt(fmt),
        }
    }
}

impl FromStr for LibKind {
    type Err = InvalidInput;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "static" => Ok(LibKind::Static),
            "dylib" => Ok(LibKind::DyLib),
            "framework" => Ok(LibKind::Framework),
            _ => Err(InvalidInput(s.into()))
        }
    }
}

/**
A build profile.
*/
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Profile {
    Debug,
    Release,
}

impl Display for Profile {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Profile::Debug => "debug".fmt(fmt),
            Profile::Release => "release".fmt(fmt),
        }
    }
}

impl FromStr for Profile {
    type Err = InvalidInput;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "debug" => Ok(Profile::Debug),
            "release" => Ok(Profile::Release),
            _ => Err(InvalidInput(s.into()))
        }
    }
}

/**
Library search path kind.
*/
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SearchKind {
    Dependency,
    Crate,
    Native,
    Framework,
    All,
}

impl Display for SearchKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SearchKind::Dependency => "dependency".fmt(fmt),
            SearchKind::Crate => "crate".fmt(fmt),
            SearchKind::Native => "native".fmt(fmt),
            SearchKind::Framework => "framework".fmt(fmt),
            SearchKind::All => "all".fmt(fmt),
        }
    }
}

impl FromStr for SearchKind {
    type Err = InvalidInput;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dependency" => Ok(SearchKind::Dependency),
            "crate" => Ok(SearchKind::Crate),
            "native" => Ok(SearchKind::Native),
            "framework" => Ok(SearchKind::Framework),
            "all" => Ok(SearchKind::All),
            _ => Err(InvalidInput(s.into()))
        }
    }
}

/**
Platform triple.
*/
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Triple {
    triple: String,
    arch: Range<usize>,
    env: Option<Range<usize>>,
    family: Range<usize>,
    os: Range<usize>,
}

impl Triple {
    /// Create a `Triple` from its string representation.
    pub fn new(triple: String) -> Triple {
        let arch;
        let env;
        let family;
        let os;

        {
            let mut parts = triple.splitn(4, '-')
                .map(|s| {
                    let off = subslice_offset(&triple, s);
                    off..(off + s.len())
                });

            arch = parts.next().expect(&format!("could not find architecture in triple {:?}", triple));
            family = parts.next().expect(&format!("could not find family in triple {:?}", triple));
            os = parts.next().expect(&format!("could not find os in triple {:?}", triple));
            env = parts.next();
        }

        Triple {
            triple: triple,
            arch: arch,
            env: env,
            family: family,
            os: os,
        }
    }

    /// Get triple as a string.
    pub fn as_str(&self) -> &str {
        &self.triple
    }

    /**
    Platform processor architecture.

    Values include `"i686"`, `"x86_64"`, `"arm"`, *etc.*
    */
    pub fn arch(&self) -> &str {
        &self.triple[self.arch.clone()]
    }

    /**
    Platform toolchain environment.

    Values include `"gnu"`, `"msvc"`, `"musl"`, `"android"` *etc.*  Value is `None` if the platform doesn't specify an environment.
    */
    pub fn env(&self) -> Option<&str> {
        self.env.as_ref()
            .map(|s| &self.triple[s.clone()])
    }

    /**
    Platform machine family.

    Values include `"apple"`, `"pc"`, `"unknown"`, *etc.*

    <!-- Definitive proof that Apples aren't PCs.  *mic drop* -->
    */
    pub fn family(&self) -> &str {
        &self.triple[self.family.clone()]
    }

    /**
    Platform operating system.

    Values include `"linux"`, `"windows"`, `"ios"`, *etc.*
    */
    pub fn os(&self) -> &str {
        &self.triple[self.os.clone()]
    }
}

impl Display for Triple {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        self.triple.fmt(fmt)
    }
}

impl FromStr for Triple {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Triple::new(s.into()))
    }
}

/**
Functions for locating toolchain binaries.
*/
pub mod bin {
    use std::env;
    use std::path::PathBuf;

    /// Path to Cargo binary.
    pub fn cargo() -> PathBuf {
        env::var_os("CARGO")
            .unwrap_or_else(|| "cargo".into())
            .into()
    }

    /// Path to `rustc` as selected by Cargo.
    pub fn rustc() -> PathBuf {
        env_var_os!("RUSTC").into()
    }

    /// Path to `rustdoc` as selected by Cargo.
    pub fn rustdoc() -> PathBuf {
        env_var_os!("RUSTDOC").into()
    }
}

/**
Information related to the Cargo package environment.
*/
pub mod cargo {
    /**
    Package features.
    */
    pub mod features {
        use std::ascii::AsciiExt;
        use std::env;

        /**
        Iterator over enabled features.
        */
        pub struct Iter {
            iter: env::Vars,
        }

        impl Iterator for Iter {
            type Item = String;

            fn next(&mut self) -> Option<Self::Item> {
                while let Some((key, _)) = self.iter.next() {
                    if key.starts_with("CARGO_FEATURE_") {
                        return Some(canon_feature_name(&key["CARGO_FEATURE_".len()..]));
                    }
                }
                None
            }
        }

        /**
        Return an iterator over all enabled Cargo features.

        # Note

        Due to the name mangling used by Cargo, features are returned as all lower case, with hypens instead of underscores.
        */
        pub fn all() -> Iter {
            Iter {
                iter: env::vars(),
            }
        }

        /// Determine if a specific feature is enabled.
        pub fn enabled(name: &str) -> bool {
            let key = format!("CARGO_FEATURE_{}", env_feature_name(name));
            env::var(&key).is_ok()
        }

        fn canon_feature_name(name: &str) -> String {
            name.chars()
                .map(|c| match c {
                    'A'...'Z' => c.to_ascii_lowercase(),
                    '_' => '-',
                    c => c
                })
                .collect()
        }

        fn env_feature_name(name: &str) -> String {
            name.chars()
                .map(|c| match c {
                    'a'...'z' => c.to_ascii_uppercase(),
                    '-' => '_',
                    c => c
                })
                .collect()
        }
    }

    /**
    Information related to the package manifest.
    */
    pub mod manifest {
        use std::env;
        use std::path::PathBuf;

        /// Path to the directory in which the manifest is stored.
        pub fn dir() -> PathBuf {
            env_var!("CARGO_MANIFEST_DIR").into()
        }

        /// The `package.links` field, if set.
        pub fn links() -> Option<String> {
            env::var("CARGO_MANIFEST_LINKS").ok()
        }
    }

    /**
    Information related to the package.
    */
    pub mod pkg {
        use semver::Version;

        /// A list of authors.
        pub fn authors() -> Vec<String> {
            env_var!("CARGO_PKG_AUTHORS")
                .split(':')
                .map(Into::into)
                .collect()
        }

        /// The package description.
        pub fn description() -> Option<String> {
            let v = env_var!("CARGO_PKG_DESCRIPTION");
            if v == "" {
                None
            } else {
                Some(v)
            }
        }

        /// URL to the package's homepage.
        pub fn homepage() -> Option<String> {
            let v = env_var!("CARGO_PKG_HOMEPAGE");
            if v == "" {
                None
            } else {
                Some(v)
            }
        }

        /// Name of the package.
        pub fn name() -> String {
            env_var!("CARGO_PKG_NAME")
        }

        /// Version of the package.
        pub fn version() -> Version {
            parse_env_var!("CARGO_PKG_VERSION", "version")
        }
    }
}

/**
Inter-dependency metadata.
*/
pub mod metadata {
    use std::env;

    /// Emit a metadata field for dependents of this package.
    pub fn emit_raw(key: &str, value: &str) {
        println!("cargo:{}={}", key, value);
    }

    /// Read a metadata field from the specified dependency of this package.
    pub fn get_raw(dep: &str, key: &str) -> Option<String> {
        let name = format!("DEP_{}_{}", dep.to_uppercase(), key.to_uppercase());
        env::var(&name).ok()
    }
}

/**
Functions for communicating with `rustc`.
*/
pub mod rustc {
    use std::path::Path;
    use ::{LibKind, SearchKind};

    /// Link a library into the output.
    pub fn link_lib<P: AsRef<Path>>(link_kind: Option<LibKind>, name: P) {
        println!("cargo:rustc-link-lib={}{}",
            link_kind.map(|v| format!("{}=", v))
                .unwrap_or_else(|| "".into()),
            name.as_ref().display());
    }

    /// Add a search directory.
    pub fn link_search<P: AsRef<Path>>(link_kind: Option<SearchKind>, path: P) {
        println!("cargo:rustc-link-search={}{}",
            link_kind.map(|v| format!("{}=", v))
                .unwrap_or_else(|| "".into()),
            path.as_ref().display());
    }

    /**
    Pass a flag to `rustc`.

    # Note

    Cargo restricts the set of permissable flags.  See the [Cargo documentation on build script outputs](http://doc.crates.io/build-script.html#outputs-of-the-build-script) for more details.
    */
    pub fn flags(flags: &str) {
        println!("cargo:rustc-flags={}", flags);
    }

    /// Define a conditional compilation flag.
    pub fn cfg(cfg: &str) {
        println!("cargo:rustc-cfg={}", cfg);
    }
}

/**
Target platform information.
*/
pub mod target {
    use super::*;

    /**
    Platform endianness.

    **Requires**: Rust 1.14.
    */
    pub fn endian() -> Option<Endianness> {
        parse_env_var!(try: "CARGO_CFG_TARGET_ENDIAN", "endianness")
    }

    /**
    Platform processor features.

    A list of features can be obtained using `rustc --print target-features`.

    **Requires**: Rust nightly.
    */
    #[cfg(feature = "nightly")]
    pub fn features() -> Option<Vec<String>> {
        env::var("CARGO_CFG_TARGET_FEATURE")
            .ok()
            .map(|v| v.split(',').map(Into::into).collect())
    }

    /**
    List of types which are atomic on this platform.

    **Requires**: Rust nightly.
    */
    #[cfg(feature = "nightly")]
    pub fn has_atomic() -> Option<Vec<Atomic>> {
        env::var("CARGO_CFG_TARGET_HAS_ATOMIC")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.parse()
                        .expect(&format!("CARGO_CFG_TARGET_HAS_ATOMIC \
                            contained invalid atomic type {:?}", s)))
                    .collect()
            })
    }

    /**
    Width, in bits, of a pointer on this platform.

    **Requires**: Rust 1.14.
    */
    pub fn pointer_width() -> Option<u8> {
        parse_env_var!(try: "CARGO_CFG_TARGET_POINTER_WIDTH", "integer")
    }

    /**
    Platform triple.

    A list of target triples can be obtained using `rustc --print target-list`.
    */
    pub fn triple() -> Triple {
        parse_env_var!("TARGET", "triple")
    }
}

/// Is this a debug build?
pub fn debug() -> bool {
    parse_env_var!("DEBUG", "bool")
}

/// Host platform triple.
pub fn host() -> Triple {
    parse_env_var!("HOST", "triple")
}

/// Number of top-level parallel jobs.
pub fn num_jobs() -> u32 {
    parse_env_var!("NUM_JOBS", "integer")
}

/// Optimisation level.
pub fn opt_level() -> u32 {
    parse_env_var!("OPT_LEVEL", "integer")
}

/// Output directory for build script outputs.
pub fn out_dir() -> PathBuf {
    env_var_os!("OUT_DIR").into()
}

/// Build profile.
pub fn profile() -> Profile {
    let s = env_var!("PROFILE");
    match &*s {
        "debug" => Profile::Debug,
        "release" => Profile::Release,
        _ => panic!("PROFILE {:?} is not a valid profile", s),
    }
}

/// Specify a file or directory which, if changed, should trigger a rebuild.
pub fn rerun_if_changed<P: AsRef<Path>>(path: P) {
    println!("cargo:rerun-if-changed={}", path.as_ref().display());
}

/**
Instructs Cargo to display a warning.

`warning!(..)` is shorthand for `build_helper::warning(&format!(..))`.
*/
pub fn warning<S: AsRef<str>>(msg: S) {
    println!("cargo:warning={}", msg.as_ref());
}

/// Is this build targeting a UNIX platform?
pub fn unix() -> bool {
    env::var("CARGO_CFG_UNIX").is_ok()
}

/// Is this build targetting Microsoft Windows?
pub fn windows() -> bool {
    env::var("CARGO_CFG_WINDOWS").is_ok()
}

/// Offset of slice within a base string.
fn subslice_offset(base: &str, inner: &str) -> usize {
    let base_beg = base.as_ptr() as usize;
    let inner = inner.as_ptr() as usize;
    if inner < base_beg || inner > base_beg.wrapping_add(base.len()) {
        panic!("cannot compute subslice offset of disjoint strings")
    } else {
        inner.wrapping_sub(base_beg)
    }
}
