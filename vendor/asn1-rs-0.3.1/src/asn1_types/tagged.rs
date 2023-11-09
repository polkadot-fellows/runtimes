use crate::{Class, Tag, Tagged};
use core::marker::PhantomData;

mod builder;
mod explicit;
mod helpers;
mod implicit;
mod optional;
mod parser;

pub use builder::*;
pub use explicit::*;
pub use helpers::*;
pub use implicit::*;
pub use optional::*;
pub use parser::*;

pub(crate) const CONTEXT_SPECIFIC: u8 = Class::ContextSpecific as u8;

/// A type parameter for `IMPLICIT` tagged values.
#[derive(Debug, PartialEq, Eq)]
pub enum Implicit {}

/// A type parameter for `EXPLICIT` tagged values.
#[derive(Debug, PartialEq, Eq)]
pub enum Explicit {}

/// A type parameter for tagged values either [`Explicit`] or [`Implicit`].
pub trait TagKind {}

impl TagKind for Implicit {}
impl TagKind for Explicit {}

/// Helper object for creating `FromBer`/`FromDer` types for TAGGED OPTIONAL types
///
/// When parsing `ContextSpecific` (the most common class), see [`TaggedExplicit`] and
/// [`TaggedImplicit`] alias types.
///
/// # Notes
///
/// `CLASS` must be between 0 and 4. See [`Class`] for possible values for the `CLASS` parameter.
///
/// # Examples
///
/// To parse a `[APPLICATION 0] EXPLICIT INTEGER` object:
///
/// ```rust
/// use asn1_rs::{Explicit, FromBer, Integer, TaggedValue};
///
/// let bytes = &[0x60, 0x03, 0x2, 0x1, 0x2];
///
/// // If tagged object is present (and has expected tag), parsing succeeds:
/// let (_, tagged) = TaggedValue::<Integer, Explicit, 0b01, 0>::from_ber(bytes).unwrap();
/// assert_eq!(tagged, TaggedValue::explicit(Integer::from(2)));
/// ```
#[derive(Debug, PartialEq)]
pub struct TaggedValue<T, TagKind, const CLASS: u8, const TAG: u32> {
    pub(crate) inner: T,

    tag_kind: PhantomData<TagKind>,
}

impl<T, TagKind, const CLASS: u8, const TAG: u32> TaggedValue<T, TagKind, CLASS, TAG> {
    /// Consumes the `TaggedParser`, returning the wrapped value.
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T, const CLASS: u8, const TAG: u32> TaggedValue<T, Explicit, CLASS, TAG> {
    /// Constructs a new `EXPLICIT TaggedParser` with the provided value
    #[inline]
    pub const fn explicit(inner: T) -> Self {
        TaggedValue {
            inner,
            tag_kind: PhantomData,
        }
    }
}

impl<T, const CLASS: u8, const TAG: u32> TaggedValue<T, Implicit, CLASS, TAG> {
    /// Constructs a new `IMPLICIT TaggedParser` with the provided value
    #[inline]
    pub const fn implicit(inner: T) -> Self {
        TaggedValue {
            inner,
            tag_kind: PhantomData,
        }
    }
}

impl<T, TagKind, const CLASS: u8, const TAG: u32> AsRef<T> for TaggedValue<T, TagKind, CLASS, TAG> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T, TagKind, const CLASS: u8, const TAG: u32> Tagged for TaggedValue<T, TagKind, CLASS, TAG> {
    const TAG: Tag = Tag(TAG);
}
