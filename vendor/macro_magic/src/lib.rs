//! # Macro Magic 🪄
//!
//! ![Build Status](https://img.shields.io/github/actions/workflow/status/sam0x17/macro_magic/ci.yaml)
//! ![GitHub](https://img.shields.io/github/license/sam0x17/macro_magic)
//! ![Crates.io](https://img.shields.io/crates/d/macro_magic)
//! ![docs.rs](https://img.shields.io/docsrs/macro_magic?label=docs)
//!
//! ## Overview
//!
//! This crate provides an [`#[export_tokens]`](`export_tokens`) attribute macro, and a number
//! of companion macros, most prominently [`#[import_tokens_proc]`](`import_tokens_proc`) and
//! [`#[import_tokens_attr]`](`import_tokens_attr`), which, when used in tandem with
//! [`#[export_tokens]`](`export_tokens`), allow you to create regular and attribute proc
//! macros in which you can import and make use of the tokens of external/foreign items marked
//! with [`#[export_tokens]`](`export_tokens`) in other modules, files, and even in other
//! crates merely by referring to them by name/path.
//!
//! Among other things, the patterns introduced by `macro_magic` can be used to implement safe
//! and efficient exportation and importation of item tokens within the same file, and even
//! across file and crate boundaries.
//!
//! ## no_std
//!
//! `macro_magic` is designed to work with stable Rust, and is fully `no_std` compatible (in
//! fact, there is a unit test to ensure everything is `no_std` safe).
//!
//! ## Features
//!
//! ### proc_support
//!
//! The `proc_support` feature _must_ be enabled in proc macro crates that make use of any
//! import tokens functionality, including [`#[import_tokens_attr]`](`import_tokens_attr`),
//! [`#[import_tokens_proc]`](`import_tokens_proc`) and [`import_tokens!`]. Otherwise these
//! macros will not function correctly and will issue compiler errors complaining about items
//! not existing under [`mm_core`]. The [`#[export_tokens]`](`export_tokens`) macro does not
//! require this feature to function correctly, so you can safely use it without enabling this
//! feature.
//!
//! The reason for this feature gating is that things like `syn`, `quote`, `proc_macro2`, etc.,
//! are not 100% `no_std` compatible and should only be enabled in proc macro crates.
//!
//! ## Limitations
//!
//! One thing that `macro_magic` _doesn't_ provide is the ability to build up state information
//! across multiple macro invocations, however this problem can be tackled effectively using
//! the [outer macro pattern](https://www.youtube.com/watch?v=aEWbZxNCH0A) or in some cases
//! using static atomics and mutexes in your proc macro crate (which we actually do in this
//! crate to keep track of unique identifiers).
//!
//! ## Breaking Changes
//!
//! - **0.4x** removed `#[use_attr]` and `#[use_proc]` (they are no longer needed with the new
//!   self-calling macro style that has been adopted in 0.4x) and also removed the ability to
//!   access `#[export_tokens]` invocations in inaccessible locations like inside of functions
//!   and across module permission boundaries like in an inaccessible private module. This
//!   feature may be re-added in the future if there is interest, however removing it allowed
//!   us to consolidate naming of our `macro_rules!` declarations and remove the need for
//!  `#[use_attr]` / `#[use_proc]`.
//! - **0.2x** removed and/or re-wrote a number of features that relied on a non-future-proof
//!   behavior of writing/reading files in the `OUT_DIR`. Versions >= 0.2.0 are completely safe
//!   and no longer contain this behavior, however features that provided the ability to
//!   enumerate all the `#[export_tokens]` calls in a namespace have been removed. The proper
//!   way to do this is with the outer macro pattern or with global state mutexes/atomics in
//!   your proc macro crate, as mentioned above.
//!
//! More detailed historical change information can be found in
//! [releases](https://github.com/sam0x17/docify/releases).

#![no_std]

/// Contains the internal code behind the `macro_magic` macros in a re-usable form, in case you
/// need to design new macros that utilize some of the internal functionality of `macro_magic`.
pub mod mm_core {
    #[cfg(feature = "proc_support")]
    pub use macro_magic_core::*;
}

pub use macro_magic_macros::{
    export_tokens, export_tokens_alias, export_tokens_no_emit, forward_tokens, use_attr, use_proc,
};

#[cfg(feature = "proc_support")]
pub use macro_magic_macros::{
    import_tokens, import_tokens_attr, import_tokens_proc, with_custom_parsing,
};

/// Contains re-exports required at compile-time by the macro_magic macros and support
/// functions.
#[doc(hidden)]
pub mod __private {
    pub use macro_magic_macros::*;

    #[cfg(feature = "proc_support")]
    pub use quote;

    #[cfg(feature = "proc_support")]
    pub use syn;

    #[cfg(feature = "proc_support")]
    pub use syn::__private::TokenStream2;
}
