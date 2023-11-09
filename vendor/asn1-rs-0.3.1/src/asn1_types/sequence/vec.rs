use crate::*;
use alloc::vec::Vec;

impl<T> Tagged for SequenceOf<T> {
    const TAG: Tag = Tag::Sequence;
}

impl<T> Tagged for Vec<T> {
    const TAG: Tag = Tag::Sequence;
}

impl<'a, T> FromBer<'a> for Vec<T>
where
    T: FromBer<'a>,
{
    fn from_ber(bytes: &'a [u8]) -> ParseResult<Self> {
        let (rem, any) = Any::from_ber(bytes)?;
        any.header.assert_tag(Self::TAG)?;
        let v = SequenceIterator::<T, BerParser>::new(any.data).collect::<Result<Vec<T>>>()?;
        Ok((rem, v))
    }
}

impl<'a, T> FromDer<'a> for Vec<T>
where
    T: FromDer<'a>,
{
    fn from_der(bytes: &'a [u8]) -> ParseResult<Self> {
        let (rem, any) = Any::from_der(bytes)?;
        any.header.assert_tag(Self::TAG)?;
        let v = SequenceIterator::<T, DerParser>::new(any.data).collect::<Result<Vec<T>>>()?;
        Ok((rem, v))
    }
}

#[cfg(feature = "std")]
impl<T> ToDer for Vec<T>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        let mut len = 0;
        for t in self.iter() {
            len += t.to_der_len()?;
        }
        let header = Header::new(Class::Universal, true, Self::TAG, Length::Definite(len));
        Ok(header.to_der_len()? + len)
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let mut len = 0;
        for t in self.iter() {
            len += t.to_der_len().map_err(|_| SerializeError::InvalidLength)?;
        }
        let header = Header::new(Class::Universal, true, Self::TAG, Length::Definite(len));
        header.write_der_header(writer).map_err(Into::into)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let mut sz = 0;
        for t in self.iter() {
            sz += t.write_der(writer)?;
        }
        Ok(sz)
    }
}
