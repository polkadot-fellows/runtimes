use crate::*;
use core::marker::PhantomData;

#[derive(Debug, PartialEq)]
pub struct TaggedParser<'a, TagKind, T> {
    pub header: Header<'a>,
    pub inner: T,

    pub(crate) tag_kind: PhantomData<TagKind>,
}

impl<'a, TagKind, T> TaggedParser<'a, TagKind, T> {
    pub const fn new(header: Header<'a>, inner: T) -> Self {
        TaggedParser {
            header,
            inner,
            tag_kind: PhantomData,
        }
    }

    pub const fn assert_class(&self, class: Class) -> Result<()> {
        self.header.assert_class(class)
    }

    pub const fn assert_tag(&self, tag: Tag) -> Result<()> {
        self.header.assert_tag(tag)
    }

    #[inline]
    pub const fn class(&self) -> Class {
        self.header.class
    }

    #[inline]
    pub const fn tag(&self) -> Tag {
        self.header.tag
    }
}

impl<'a, TagKind, T> AsRef<T> for TaggedParser<'a, TagKind, T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<'a, TagKind, T> TaggedParser<'a, TagKind, T>
where
    Self: FromBer<'a>,
{
    pub fn parse_ber(class: Class, tag: Tag, bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, t) = TaggedParser::<TagKind, T>::from_ber(bytes)?;
        t.assert_class(class)?;
        t.assert_tag(tag)?;
        Ok((rem, t))
    }
}

impl<'a, TagKind, T> TaggedParser<'a, TagKind, T>
where
    Self: FromDer<'a>,
{
    pub fn parse_der(class: Class, tag: Tag, bytes: &'a [u8]) -> ParseResult<'a, Self> {
        let (rem, t) = TaggedParser::<TagKind, T>::from_der(bytes)?;
        t.assert_class(class)?;
        t.assert_tag(tag)?;
        Ok((rem, t))
    }
}

impl<'a, TagKind, T> DynTagged for TaggedParser<'a, TagKind, T> {
    fn tag(&self) -> Tag {
        self.tag()
    }
}
