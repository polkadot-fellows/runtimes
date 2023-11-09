use crate::*;
use core::convert::TryFrom;
use core::marker::PhantomData;

impl<'a, T, const CLASS: u8, const TAG: u32> TryFrom<Any<'a>>
    for TaggedValue<T, Explicit, CLASS, TAG>
where
    T: FromBer<'a>,
{
    type Error = Error;

    fn try_from(any: Any<'a>) -> Result<Self> {
        any.tag().assert_eq(Tag(TAG))?;
        any.header.assert_constructed()?;
        if any.class() as u8 != CLASS {
            let class = Class::try_from(CLASS).ok();
            return Err(Error::unexpected_class(class, any.class()));
        }
        let (_, inner) = T::from_ber(any.data)?;
        Ok(TaggedValue::explicit(inner))
    }
}

impl<'a, T, const CLASS: u8, const TAG: u32> CheckDerConstraints
    for TaggedValue<T, Explicit, CLASS, TAG>
where
    T: CheckDerConstraints,
{
    fn check_constraints(any: &Any) -> Result<()> {
        any.header.length.assert_definite()?;
        let (_, inner) = Any::from_ber(any.data)?;
        T::check_constraints(&inner)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<T, const CLASS: u8, const TAG: u32> ToDer for TaggedValue<T, Explicit, CLASS, TAG>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        let sz = self.inner.to_der_len()?;
        if sz < 127 {
            // 1 (class+tag) + 1 (length) + len
            Ok(2 + sz)
        } else {
            // 1 (class+tag) + n (length) + len
            let n = Length::Definite(sz).to_der_len()?;
            Ok(1 + n + sz)
        }
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let inner_len = self.inner.to_der_len()?;
        let class =
            Class::try_from(CLASS).map_err(|_| SerializeError::InvalidClass { class: CLASS })?;
        let header = Header::new(class, true, self.tag(), Length::Definite(inner_len));
        header.write_der_header(writer).map_err(Into::into)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        self.inner.write_der(writer)
    }
}

/// A helper object to parse `[ n ] EXPLICIT T`
///
/// A helper object implementing [`FromBer`] and [`FromDer`], to parse tagged
/// optional values.
///
/// This helper expects context-specific tags.
/// See [`TaggedValue`] or [`TaggedParser`] for more generic implementations if needed.
///
/// # Examples
///
/// To parse a `[0] EXPLICIT INTEGER` object:
///
/// ```rust
/// use asn1_rs::{FromBer, Integer, TaggedExplicit, TaggedValue};
///
/// let bytes = &[0xa0, 0x03, 0x2, 0x1, 0x2];
///
/// // If tagged object is present (and has expected tag), parsing succeeds:
/// let (_, tagged) = TaggedExplicit::<Integer, 0>::from_ber(bytes).unwrap();
/// assert_eq!(tagged, TaggedValue::explicit(Integer::from(2)));
/// ```
pub type TaggedExplicit<T, const TAG: u32> = TaggedValue<T, Explicit, CONTEXT_SPECIFIC, TAG>;

// implementations for TaggedParser

impl<'a, T> TaggedParser<'a, Implicit, T> {
    pub const fn new_implicit(class: Class, constructed: bool, tag: u32, inner: T) -> Self {
        Self {
            header: Header::new(class, constructed, Tag(tag), Length::Definite(0)),
            inner,
            tag_kind: PhantomData,
        }
    }
}

impl<'a, T> TaggedParser<'a, Explicit, T> {
    pub const fn new_explicit(class: Class, tag: u32, inner: T) -> Self {
        Self {
            header: Header::new(class, true, Tag(tag), Length::Definite(0)),
            inner,
            tag_kind: PhantomData,
        }
    }
}

impl<'a, T> TaggedParser<'a, Explicit, T> {
    pub fn from_ber_and_then<F>(
        class: Class,
        tag: u32,
        bytes: &'a [u8],
        op: F,
    ) -> ParseResult<'a, T>
    where
        F: FnOnce(&'a [u8]) -> ParseResult<T>,
    {
        let (rem, any) = Any::from_ber(bytes)?;
        any.tag().assert_eq(Tag(tag))?;
        if any.class() != class {
            return Err(any.tag().invalid_value("Invalid class").into());
        }
        let (_, res) = op(any.data)?;
        Ok((rem, res))
    }

    pub fn from_der_and_then<F>(
        class: Class,
        tag: u32,
        bytes: &'a [u8],
        op: F,
    ) -> ParseResult<'a, T>
    where
        F: FnOnce(&'a [u8]) -> ParseResult<T>,
    {
        let (rem, any) = Any::from_der(bytes)?;
        any.tag().assert_eq(Tag(tag))?;
        if any.class() != class {
            return Err(any.tag().invalid_value("Invalid class").into());
        }
        let (_, res) = op(any.data)?;
        Ok((rem, res))
    }
}

impl<'a, T> FromBer<'a> for TaggedParser<'a, Explicit, T>
where
    T: FromBer<'a>,
{
    fn from_ber(bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, any) = Any::from_ber(bytes)?;
        let header = any.header;
        let (_, inner) = T::from_ber(any.data)?;
        let tagged = TaggedParser {
            header,
            inner,
            tag_kind: PhantomData,
        };
        Ok((rem, tagged))
    }
}

impl<'a, T> FromDer<'a> for TaggedParser<'a, Explicit, T>
where
    T: FromDer<'a>,
{
    fn from_der(bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, any) = Any::from_der(bytes)?;
        let header = any.header;
        let (_, inner) = T::from_der(any.data)?;
        let tagged = TaggedParser {
            header,
            inner,
            tag_kind: PhantomData,
        };
        Ok((rem, tagged))
    }
}

impl<'a, T> CheckDerConstraints for TaggedParser<'a, Explicit, T>
where
    T: CheckDerConstraints,
{
    fn check_constraints(any: &Any) -> Result<()> {
        any.header.length.assert_definite()?;
        let (_, inner_any) = Any::from_der(any.data)?;
        T::check_constraints(&inner_any)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<'a, T> ToDer for TaggedParser<'a, Explicit, T>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        let sz = self.inner.to_der_len()?;
        if sz < 127 {
            // 1 (class+tag) + 1 (length) + len
            Ok(2 + sz)
        } else {
            // 1 (class+tag) + n (length) + len
            let n = Length::Definite(sz).to_der_len()?;
            Ok(1 + n + sz)
        }
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let inner_len = self.inner.to_der_len()?;
        let header = Header::new(self.class(), true, self.tag(), Length::Definite(inner_len));
        header.write_der_header(writer).map_err(Into::into)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        self.inner.write_der(writer)
    }
}
