use crate::ber::*;
use crate::*;
use alloc::borrow::Cow;
use alloc::string::String;
use core::convert::TryInto;

/// The `Any` object is not strictly an ASN.1 type, but holds a generic description of any object
/// that could be encoded.
///
/// It contains a header, and either a reference to or owned data for the object content.
///
/// Note: this type is only provided in **borrowed** version (*i.e.* it can own the inner data).
#[derive(Clone, Debug, PartialEq)]
pub struct Any<'a> {
    /// The object header
    pub header: Header<'a>,
    /// The object contents
    pub data: &'a [u8],
}

impl<'a> Any<'a> {
    /// Create a new `Any` from BER/DER header and content
    #[inline]
    pub const fn new(header: Header<'a>, data: &'a [u8]) -> Self {
        Any { header, data }
    }

    /// Create a new `Any` from a tag, and BER/DER content
    #[inline]
    pub const fn from_tag_and_data(tag: Tag, data: &'a [u8]) -> Self {
        let constructed = matches!(tag, Tag::Sequence | Tag::Set);
        Any {
            header: Header {
                tag,
                constructed,
                class: Class::Universal,
                length: Length::Definite(data.len()),
                raw_tag: None,
            },
            data,
        }
    }

    /// Return the `Class` of this object
    #[inline]
    pub const fn class(&self) -> Class {
        self.header.class
    }

    /// Update the class of the current object
    #[inline]
    pub fn with_class(self, class: Class) -> Self {
        Any {
            header: self.header.with_class(class),
            ..self
        }
    }

    /// Return the `Tag` of this object
    #[inline]
    pub const fn tag(&self) -> Tag {
        self.header.tag
    }

    /// Update the tag of the current object
    #[inline]
    pub fn with_tag(self, tag: Tag) -> Self {
        Any {
            header: self.header.with_tag(tag),
            data: self.data,
        }
    }

    /// Get the bytes representation of the *content*
    #[inline]
    pub fn as_bytes(&'a self) -> &'a [u8] {
        self.data
    }

    #[inline]
    pub fn parse_ber<T>(&'a self) -> ParseResult<'a, T>
    where
        T: FromBer<'a>,
    {
        T::from_ber(self.data)
    }

    #[inline]
    pub fn parse_der<T>(&'a self) -> ParseResult<'a, T>
    where
        T: FromDer<'a>,
    {
        T::from_der(self.data)
    }
}

macro_rules! impl_any_into {
    (IMPL $sname:expr, $fn_name:ident => $ty:ty, $asn1:expr) => {
        #[doc = "Attempt to convert object to `"]
        #[doc = $sname]
        #[doc = "` (ASN.1 type: `"]
        #[doc = $asn1]
        #[doc = "`)."]
        pub fn $fn_name(self) -> Result<$ty> {
            self.try_into()
        }
    };
    ($fn_name:ident => $ty:ty, $asn1:expr) => {
        impl_any_into! {
            IMPL stringify!($ty), $fn_name => $ty, $asn1
        }
    };
}

impl<'a> Any<'a> {
    impl_any_into!(bitstring => BitString<'a>, "BIT STRING");
    impl_any_into!(bmpstring => BmpString<'a>, "BmpString");
    impl_any_into!(bool => bool, "BOOLEAN");
    impl_any_into!(boolean => Boolean, "BOOLEAN");
    impl_any_into!(embedded_pdv => EmbeddedPdv<'a>, "EMBEDDED PDV");
    impl_any_into!(enumerated => Enumerated, "ENUMERATED");
    impl_any_into!(generalizedtime => GeneralizedTime, "GeneralizedTime");
    impl_any_into!(generalstring => GeneralString<'a>, "GeneralString");
    impl_any_into!(graphicstring => GraphicString<'a>, "GraphicString");
    impl_any_into!(ia5string => Ia5String<'a>, "IA5String");
    impl_any_into!(integer => Integer<'a>, "INTEGER");
    impl_any_into!(null => Null, "NULL");
    impl_any_into!(numericstring => NumericString<'a>, "NumericString");
    impl_any_into!(objectdescriptor => ObjectDescriptor<'a>, "ObjectDescriptor");
    impl_any_into!(octetstring => OctetString<'a>, "OCTET STRING");
    impl_any_into!(oid => Oid<'a>, "OBJECT IDENTIFIER");
    /// Attempt to convert object to `Oid` (ASN.1 type: `RELATIVE-OID`).
    pub fn relative_oid(self) -> Result<Oid<'a>> {
        self.header.assert_tag(Tag::RelativeOid)?;
        let asn1 = Cow::Borrowed(self.data);
        Ok(Oid::new_relative(asn1))
    }
    impl_any_into!(printablestring => PrintableString<'a>, "PrintableString");
    impl_any_into!(sequence => Sequence<'a>, "SEQUENCE");
    impl_any_into!(set => Set<'a>, "SET");
    impl_any_into!(string => String, "UTF8String");
    impl_any_into!(teletexstring => TeletexString<'a>, "TeletexString");
    impl_any_into!(u8 => u8, "INTEGER");
    impl_any_into!(u16 => u16, "INTEGER");
    impl_any_into!(u32 => u32, "INTEGER");
    impl_any_into!(u64 => u64, "INTEGER");
    impl_any_into!(universalstring => UniversalString<'a>, "UniversalString");
    impl_any_into!(utctime => UtcTime, "UTCTime");
    impl_any_into!(utf8string => Utf8String<'a>, "UTF8String");
    impl_any_into!(videotexstring => VideotexString<'a>, "VideotexString");
    impl_any_into!(visiblestring => VisibleString<'a>, "VisibleString");
}

impl<'a> FromBer<'a> for Any<'a> {
    fn from_ber(bytes: &'a [u8]) -> ParseResult<Self> {
        let (i, header) = Header::from_ber(bytes)?;
        let (i, data) = ber_get_object_content(i, &header, MAX_RECURSION)?;
        Ok((i, Any { header, data }))
    }
}

impl<'a> FromDer<'a> for Any<'a> {
    fn from_der(bytes: &'a [u8]) -> ParseResult<Self> {
        let (i, header) = Header::from_der(bytes)?;
        // X.690 section 10.1: The definite form of length encoding shall be used
        header.length.assert_definite()?;
        let (i, data) = ber_get_object_content(i, &header, MAX_RECURSION)?;
        Ok((i, Any { header, data }))
    }
}

impl CheckDerConstraints for Any<'_> {
    fn check_constraints(any: &Any) -> Result<()> {
        any.header.length().assert_definite()?;
        // if len < 128, must use short form (10.1: minimum number of octets)
        Ok(())
    }
}

impl DynTagged for Any<'_> {
    fn tag(&self) -> Tag {
        self.tag()
    }
}

// impl<'a> ToStatic for Any<'a> {
//     type Owned = Any<'static>;

//     fn to_static(&self) -> Self::Owned {
//         Any {
//             header: self.header.to_static(),
//             data: Cow::Owned(self.data.to_vec()),
//         }
//     }
// }

#[cfg(feature = "std")]
impl ToDer for Any<'_> {
    fn to_der_len(&self) -> Result<usize> {
        let hdr_len = self.header.to_der_len()?;
        Ok(hdr_len + self.data.len())
    }

    fn write_der_header(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        // create fake header to have correct length
        let header = Header::new(
            self.header.class,
            self.header.constructed,
            self.header.tag,
            Length::Definite(self.data.len()),
        );
        let sz = header.write_der_header(writer)?;
        Ok(sz)
    }

    fn write_der_content(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        writer.write(self.data).map_err(Into::into)
    }

    /// Similar to using `to_der`, but uses header without computing length value
    fn write_der_raw(&self, writer: &mut dyn std::io::Write) -> SerializeResult<usize> {
        let sz = self.header.write_der_header(writer)?;
        let sz = sz + writer.write(self.data)?;
        Ok(sz)
    }
}
