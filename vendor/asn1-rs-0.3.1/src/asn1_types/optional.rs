use crate::*;

impl<'a, T> FromBer<'a> for Option<T>
where
    T: FromBer<'a>,
{
    fn from_ber(bytes: &'a [u8]) -> ParseResult<Self> {
        if bytes.is_empty() {
            return Ok((bytes, None));
        }
        match T::from_ber(bytes) {
            Ok((rem, t)) => Ok((rem, Some(t))),
            Err(nom::Err::Error(Error::UnexpectedTag { .. })) => Ok((bytes, None)),
            Err(e) => Err(e),
        }
    }
}

impl<'a, T> FromDer<'a> for Option<T>
where
    T: FromDer<'a>,
{
    fn from_der(bytes: &'a [u8]) -> ParseResult<Self> {
        if bytes.is_empty() {
            return Ok((bytes, None));
        }
        match T::from_der(bytes) {
            Ok((rem, t)) => Ok((rem, Some(t))),
            Err(nom::Err::Error(Error::UnexpectedTag { .. })) => Ok((bytes, None)),
            Err(e) => Err(e),
        }
    }
}

impl<T> CheckDerConstraints for Option<T>
where
    T: CheckDerConstraints,
{
    fn check_constraints(any: &Any) -> Result<()> {
        T::check_constraints(any)
    }
}

impl<T> DynTagged for Option<T>
where
    T: DynTagged,
{
    fn tag(&self) -> Tag {
        if self.is_some() {
            self.tag()
        } else {
            Tag(0)
        }
    }
}

#[cfg(feature = "std")]
impl<T> ToDer for Option<T>
where
    T: ToDer,
{
    fn to_der_len(&self) -> Result<usize> {
        match self {
            None => Ok(0),
            Some(t) => t.to_der_len(),
        }
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        match self {
            None => Ok(0),
            Some(t) => t.write_der_header(writer),
        }
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        match self {
            None => Ok(0),
            Some(t) => t.write_der_content(writer),
        }
    }
}
