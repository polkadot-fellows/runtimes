use crate::*;
use core::convert::TryFrom;
use core::marker::PhantomData;

impl<'a, T, const CLASS: u8, const TAG: u32> TryFrom<Any<'a>>
    for TaggedValue<T, Implicit, CLASS, TAG>
where
    T: TryFrom<Any<'a>, Error = Error>,
    T: Tagged,
{
    type Error = Error;

    fn try_from(any: Any<'a>) -> Result<Self> {
        any.tag().assert_eq(Tag(TAG))?;
        // XXX if input is empty, this function is not called

        if any.class() as u8 != CLASS {
            let class = Class::try_from(CLASS).ok();
            return Err(Error::unexpected_class(class, any.class()));
        }
        let any = Any {
            header: Header {
                tag: T::TAG,
                ..any.header.clone()
            },
            data: any.data,
        };
        match T::try_from(any) {
            Ok(inner) => Ok(TaggedValue::implicit(inner)),
            Err(e) => Err(e),
        }
    }
}

impl<'a, T, const CLASS: u8, const TAG: u32> CheckDerConstraints
    for TaggedValue<T, Implicit, CLASS, TAG>
where
    T: CheckDerConstraints,
    T: Tagged,
{
    fn check_constraints(any: &Any) -> Result<()> {
        any.header.length.assert_definite()?;
        let header = any.header.clone().with_tag(T::TAG);
        let inner = Any::new(header, any.data);
        T::check_constraints(&inner)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<T, const CLASS: u8, const TAG: u32> ToDer for TaggedValue<T, Implicit, CLASS, TAG>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        self.inner.to_der_len()
    }

    fn write_der(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let class =
            Class::try_from(CLASS).map_err(|_| SerializeError::InvalidClass { class: CLASS })?;
        let mut v = Vec::new();
        let inner_len = self.inner.write_der_content(&mut v)?;
        // XXX X.690 section 8.14.3: if implicing tagging was used [...]:
        // XXX a) the encoding shall be constructed if the base encoding is constructed, and shall be primitive otherwise
        let constructed = matches!(TAG, 16 | 17);
        let header = Header::new(class, constructed, self.tag(), Length::Definite(inner_len));
        let sz = header.write_der_header(writer)?;
        let sz = sz + writer.write(&v)?;
        Ok(sz)
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let mut sink = std::io::sink();
        let class =
            Class::try_from(CLASS).map_err(|_| SerializeError::InvalidClass { class: CLASS })?;
        let inner_len = self.inner.write_der_content(&mut sink)?;
        // XXX X.690 section 8.14.3: if implicing tagging was used [...]:
        // XXX a) the encoding shall be constructed if the base encoding is constructed, and shall be primitive otherwise
        let constructed = matches!(TAG, 16 | 17);
        let header = Header::new(class, constructed, self.tag(), Length::Definite(inner_len));
        header.write_der_header(writer).map_err(Into::into)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        self.inner.write_der(writer)
    }
}

/// A helper object to parse `[ n ] IMPLICIT T`
///
/// A helper object implementing [`FromBer`] and [`FromDer`], to parse tagged
/// optional values.
///
/// This helper expects context-specific tags.
/// See [`TaggedValue`] or [`TaggedParser`] for more generic implementations if needed.
///
/// # Examples
///
/// To parse a `[0] IMPLICIT INTEGER OPTIONAL` object:
///
/// ```rust
/// use asn1_rs::{FromBer, Integer, TaggedImplicit, TaggedValue};
///
/// let bytes = &[0xa0, 0x1, 0x2];
///
/// let (_, tagged) = TaggedImplicit::<Integer, 0>::from_ber(bytes).unwrap();
/// assert_eq!(tagged, TaggedValue::implicit(Integer::from(2)));
/// ```
pub type TaggedImplicit<T, const TAG: u32> = TaggedValue<T, Implicit, CONTEXT_SPECIFIC, TAG>;

impl<'a, T> FromBer<'a> for TaggedParser<'a, Implicit, T>
where
    T: TryFrom<Any<'a>, Error = Error>,
    T: Tagged,
{
    fn from_ber(bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, any) = Any::from_ber(bytes)?;
        let Any { header, data } = any;
        let any = Any {
            header: Header {
                tag: T::TAG,
                ..header.clone()
            },
            data,
        };
        match T::try_from(any) {
            Ok(t) => {
                let tagged_value = TaggedParser {
                    header,
                    inner: t,
                    tag_kind: PhantomData,
                };
                Ok((rem, tagged_value))
            }
            Err(e) => Err(e.into()),
        }
    }
}

// implementations for TaggedParser

impl<'a, T> FromDer<'a> for TaggedParser<'a, Implicit, T>
where
    T: TryFrom<Any<'a>, Error = Error>,
    T: CheckDerConstraints,
    T: Tagged,
{
    fn from_der(bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, any) = Any::from_der(bytes)?;
        let Any { header, data } = any;
        let any = Any {
            header: Header {
                tag: T::TAG,
                ..header.clone()
            },
            data,
        };
        T::check_constraints(&any)?;
        match T::try_from(any) {
            Ok(t) => {
                let tagged_value = TaggedParser {
                    header,
                    inner: t,
                    tag_kind: PhantomData,
                };
                Ok((rem, tagged_value))
            }
            Err(e) => Err(e.into()),
        }
    }
}

impl<'a, T> CheckDerConstraints for TaggedParser<'a, Implicit, T>
where
    T: CheckDerConstraints,
    T: Tagged,
{
    fn check_constraints(any: &Any) -> Result<()> {
        any.header.length.assert_definite()?;
        let any = Any {
            header: Header {
                tag: T::TAG,
                ..any.header.clone()
            },
            data: any.data,
        };
        T::check_constraints(&any)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl<'a, T> ToDer for TaggedParser<'a, Implicit, T>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        self.inner.to_der_len()
    }

    fn write_der(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let mut v = Vec::new();
        let inner_len = self.inner.write_der_content(&mut v)?;
        // XXX X.690 section 8.14.3: if implicing tagging was used [...]:
        // XXX a) the encoding shall be constructed if the base encoding is constructed, and shall be primitive otherwise
        let header = Header::new(self.class(), false, self.tag(), Length::Definite(inner_len));
        let sz = header.write_der_header(writer)?;
        let sz = sz + writer.write(&v)?;
        Ok(sz)
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let mut sink = std::io::sink();
        let inner_len = self.inner.write_der_content(&mut sink)?;
        // XXX X.690 section 8.14.3: if implicing tagging was used [...]:
        // XXX a) the encoding shall be constructed if the base encoding is constructed, and shall be primitive otherwise
        let header = Header::new(self.class(), false, self.tag(), Length::Definite(inner_len));
        header.write_der_header(writer).map_err(Into::into)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        self.inner.write_der_content(writer)
    }
}
