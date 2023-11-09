#![recursion_limit = "4096"]

use syn::Type;
use tt_call::{parse_type, tt_call};

macro_rules! assert_type {
    ($($tokens:tt)*) => {
        tt_call! {
            macro = [{ parse_type }]
            input = [{ $($tokens)* @ }]
            ~~> assert_type_return! {
                expected = [{ $($tokens)* }]
            }
        }
    };
}

macro_rules! assert_type_return {
    {
        expected = [{ $($expected:tt)* }]
        type = [{ $($actual:tt)* }]
        rest = [{ @ }]
    } => {
        check(stringify!($($expected)*), stringify!($($actual)*));
    };
}

fn check(expected: &str, actual: &str) {
    assert_eq!(
        syn::parse_str::<Type>(expected).unwrap(),
        syn::parse_str::<Type>(actual).unwrap(),
    );
}

#[test]
fn test_parse_type() {
    // Paths
    assert_type!(u8);
    assert_type!(std::collections::HashMap);
    assert_type!(::std::collections::HashMap);

    // Angle brackets
    assert_type!(Vec<u8>);
    assert_type!(<u8>::Associated);
    assert_type!(<u8 as Trait>::Associated);
    assert_type!(<u8 as Trait<T>>::Associated);
    assert_type!(<Vec<u8> as Trait>::Associated);
    assert_type!(Iterator<Item = u8>);
    assert_type!(RefMut<'a, u8>);

    // Square brackets
    assert_type!([u8]);
    assert_type!([u8; 64]);

    // Pointers
    assert_type!(*const u8);
    assert_type!(*mut u8);

    // References
    assert_type!(&u8);
    assert_type!(&mut u8);
    assert_type!(&'a u8);
    assert_type!(&'a mut u8);

    // Functions
    assert_type!(fn());
    assert_type!(fn(u8));
    assert_type!(fn(u8));
    assert_type!(fn(u8, u8));
    assert_type!(fn(a: u8, b: u8));
    assert_type!(fn() -> u8);

    // Tuples
    assert_type!(());
    assert_type!((u8));
    assert_type!((u8,));
    assert_type!((u8, u8));

    // Traits
    assert_type!(dyn Display);
    assert_type!(dyn Display + Send);
    assert_type!(impl Display);
    assert_type!(impl Display + Send);
    assert_type!(impl Fn() -> Box<Display + Send> + 'static);

    // Type macros
    assert_type!(m!());
    assert_type!(m![]);
    assert_type!(m! {});
    assert_type!(::m!());
    assert_type!(::m!(u8));
    assert_type!(crate::m!(u8));

    // Special punctuation
    assert_type!(!);
    assert_type!(_);
}

#[test]
fn test_futures() {
    assert_type!(
        futures::MapErr<
            futures::Map<
                futures::sink::SendAll<
                    futures::stream::SplitSink<
                        futures::stream::AndThen<
                            tokio_core::io::Framed<
                                Client,
                                ipc::LenDelimited<protobuf::Message::Message>,
                            >,
                            fn(
                                tokio_core::io::EasyBuf,
                            )
                                -> std::result::Result<protobuf::Message::Message, std::io::Error>,
                            std::result::Result<protobuf::Message::Message, std::io::Error>,
                        >,
                    >,
                    futures::stream::Map<
                        futures::stream::SplitStream<
                            futures::stream::AndThen<
                                tokio_core::io::Framed<
                                    Client,
                                    ipc::LenDelimited<protobuf::Message::Message>,
                                >,
                                fn(
                                    tokio_core::io::EasyBuf,
                                ) -> std::result::Result<
                                    protobuf::Message::Message,
                                    std::io::Error,
                                >,
                                std::result::Result<protobuf::Message::Message, std::io::Error>,
                            >,
                        >,
                        H,
                    >,
                >,
                fn(
                    (
                        futures::stream::SplitSink<
                            futures::stream::AndThen<
                                tokio_core::io::Framed<
                                    Client,
                                    ipc::LenDelimited<protobuf::Message::Message>,
                                >,
                                fn(
                                    tokio_core::io::EasyBuf,
                                ) -> std::result::Result<
                                    protobuf::Message::Message,
                                    std::io::Error,
                                >,
                                std::result::Result<protobuf::Message::Message, std::io::Error>,
                            >,
                        >,
                        futures::stream::Map<
                            futures::stream::SplitStream<
                                futures::stream::AndThen<
                                    tokio_core::io::Framed<
                                        Client,
                                        ipc::LenDelimited<protobuf::Message::Message>,
                                    >,
                                    fn(
                                        tokio_core::io::EasyBuf,
                                    ) -> std::result::Result<
                                        protobuf::Message::Message,
                                        std::io::Error,
                                    >,
                                    std::result::Result<protobuf::Message::Message, std::io::Error>,
                                >,
                            >,
                            H,
                        >
                    ),
                ),
            >,
            EH,
        >
    );
}
