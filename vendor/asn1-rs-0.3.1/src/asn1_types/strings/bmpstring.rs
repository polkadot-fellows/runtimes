// do not use the `asn1_string` macro, since types are not the same
// X.680 section 37.15

use crate::*;
use alloc::borrow::Cow;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// ASN.1 `BMPSTRING` type
///
/// Note: parsing a `BmpString` allocates memory since the UTF-16 to UTF-8 conversion requires a memory allocation.
/// (see `String::from_utf16` method).
#[derive(Debug, PartialEq)]
pub struct BmpString<'a> {
    pub(crate) data: Cow<'a, str>,
}

impl<'a> BmpString<'a> {
    pub const fn new(s: &'a str) -> Self {
        BmpString {
            data: Cow::Borrowed(s),
        }
    }

    pub fn string(&self) -> String {
        self.data.to_string()
    }
}

impl<'a> AsRef<str> for BmpString<'a> {
    fn as_ref(&self) -> &str {
        &self.data
    }
}

impl<'a> From<&'a str> for BmpString<'a> {
    fn from(s: &'a str) -> Self {
        Self::new(s)
    }
}

impl From<String> for BmpString<'_> {
    fn from(s: String) -> Self {
        Self {
            data: alloc::borrow::Cow::Owned(s),
        }
    }
}

impl<'a> core::convert::TryFrom<Any<'a>> for BmpString<'a> {
    type Error = Error;

    fn try_from(any: Any<'a>) -> Result<BmpString<'a>> {
        any.tag().assert_eq(Self::TAG)?;

        // read slice as big-endian UTF-16 string
        let v = &any
            .data
            .chunks(2)
            .map(|s| match s {
                [a, b] => ((*a as u16) << 8) | (*b as u16),
                [a] => *a as u16,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();

        let s = String::from_utf16(v)?;
        let data = Cow::Owned(s);

        Ok(BmpString { data })
    }
}

impl<'a> CheckDerConstraints for BmpString<'a> {
    fn check_constraints(any: &Any) -> Result<()> {
        any.header.assert_primitive()?;
        Ok(())
    }
}

impl<'a> Tagged for BmpString<'a> {
    const TAG: Tag = Tag::BmpString;
}

impl<'a> TestValidCharset for BmpString<'a> {
    fn test_valid_charset(i: &[u8]) -> Result<()> {
        if i.len() % 2 != 0 {
            return Err(Error::StringInvalidCharset);
        }
        let iter = i.chunks(2).map(|s| ((s[0] as u16) << 8) | (s[1] as u16));
        for c in char::decode_utf16(iter) {
            if c.is_err() {
                return Err(Error::StringInvalidCharset);
            }
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
impl ToDer for BmpString<'_> {
    fn to_der_len(&self) -> Result<usize> {
        let sz = self.data.as_bytes().len();
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
        let header = Header::new(
            Class::Universal,
            false,
            Self::TAG,
            Length::Definite(self.data.as_bytes().len()),
        );
        header.write_der_header(writer).map_err(Into::into)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        writer.write(self.data.as_bytes()).map_err(Into::into)
    }
}
