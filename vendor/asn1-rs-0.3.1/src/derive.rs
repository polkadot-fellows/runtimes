/// # BerSequence custom derive
///
/// `BerSequence` is a custom derive attribute, to derive a BER [`Sequence`](super::Sequence) parser automatically from the structure definition.
/// This attribute will automatically derive implementations for the following traits:
///   - [`TryFrom<Any>`](super::Any), also providing [`FromBer`](super::FromBer)
///   - [`Tagged`](super::Tagged)
///
/// `DerSequence` implies `BerSequence`, and will conflict with this custom derive. Use `BerSequence` when you only want the
/// above traits derived.
///
/// Parsers will be automatically derived from struct fields. Every field type must implement the [`FromBer`](super::FromBer) trait.
///
/// ## Examples
///
/// To parse the following ASN.1 structure:
/// <pre>
/// S ::= SEQUENCE {
///     a INTEGER(0..2^32),
///     b INTEGER(0..2^16),
///     c INTEGER(0..2^16),
/// }
/// </pre>
///
/// Define a structure and add the `BerSequence` derive:
///
/// ```rust
/// use asn1_rs::*;
///
/// #[derive(BerSequence)]
/// struct S {
///   a: u32,
///   b: u16,
///   c: u16
/// }
/// ```
///
/// ## Debugging
///
/// To help debugging the generated code, the `#[debug_derive]` attribute has been added.
///
/// When this attribute is specified, the generated code will be printed to `stderr` during compilation.
///
/// Example:
/// ```rust
/// use asn1_rs::*;
///
/// #[derive(BerSequence)]
/// #[debug_derive]
/// struct S {
///   a: u32,
/// }
/// ```
pub use asn1_rs_derive::BerSequence;

/// # DerSequence custom derive
///
/// `DerSequence` is a custom derive attribute, to derive both BER and DER [`Sequence`](super::Sequence) parsers automatically from the structure definition.
/// This attribute will automatically derive implementations for the following traits:
///   - [`TryFrom<Any>`](super::Any), also providing [`FromBer`](super::FromBer)
///   - [`Tagged`](super::Tagged)
///   - [`FromDer`](super::FromDer)
///
/// `DerSequence` implies `BerSequence`, and will conflict with this custom derive.
///
/// Parsers will be automatically derived from struct fields. Every field type must implement the [`FromDer`](super::FromDer) trait.
///
/// ## Examples
///
/// To parse the following ASN.1 structure:
/// <pre>
/// S ::= SEQUENCE {
///     a INTEGER(0..2^32),
///     b INTEGER(0..2^16),
///     c INTEGER(0..2^16),
/// }
/// </pre>
///
/// Define a structure and add the `DerSequence` derive:
///
/// ```rust
/// use asn1_rs::*;
///
/// #[derive(DerSequence)]
/// struct S {
///   a: u32,
///   b: u16,
///   c: u16
/// }
/// ```
///
/// ## Debugging
///
/// To help debugging the generated code, the `#[debug_derive]` attribute has been added.
///
/// When this attribute is specified, the generated code will be printed to `stderr` during compilation.
///
/// Example:
/// ```rust
/// use asn1_rs::*;
///
/// #[derive(DerSequence)]
/// #[debug_derive]
/// struct S {
///   a: u32,
/// }
/// ```
pub use asn1_rs_derive::DerSequence;
