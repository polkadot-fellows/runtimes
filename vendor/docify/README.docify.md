# Docify

[![Crates.io](https://img.shields.io/crates/v/docify)](https://crates.io/crates/docify)
[![docs.rs](https://img.shields.io/docsrs/docify?label=docs)](https://docs.rs/docify/latest/docify/)
[![Build Status](https://img.shields.io/github/actions/workflow/status/sam0x17/docify/ci.yaml)](https://github.com/sam0x17/docify/actions/workflows/ci.yaml?query=branch%3Amain)
[![MIT License](https://img.shields.io/github/license/sam0x17/docify)](https://github.com/sam0x17/docify/blob/main/LICENSE)

This crate provides a simple set of rust macros, namely
[`#[docify::export]`](https://docs.rs/docify/latest/docify/attr.export.html) and
[`docify::embed!`](https://docs.rs/docify/latest/docify/macro.embed.html), that allow you to
dynamically embed tests and examples from the current crate or sub-crates of the current crate
directly within rust docs comments, with the option to make these examples runnable.

The intent behind docify is to allow you to showcase your best examples and tests directly in
your docs, without having to update them in two places every time there is a change. It also
encourages a methodology where crate authors better document their tests, since they can now
showcase these directly in their doc comments.

All-in-all this is a much better workflow than having doc examples isolated within your docs,
since you can avoid boilerplate from the surrounding code and just focus on showcasing the item
you want to highlight.

## General Usage

Using `docify` is simple. First mark the tests/examples/items that you wish to embed with
[`#[docify::export]`](https://docs.rs/docify/latest/docify/attr.export.html), such as the
following:

```rust
#[docify::export]
fn some_example() {
  assert_eq!(2 + 2, 4);
  assert_eq!(2 + 3, 5);
  assert_eq!(3 + 3, 6);
}
```

You can then embed this item directly in doc comments using the `docify::embed` macro:

```rust
/// These are some docs about an item. You can embed examples, tests, and
/// other items directly into docs using the following macro:
#[doc = docify::embed!("source/file/path.rs", some_example)]
/// More docs can go here, the example will embed itself inline exactly
/// where you reference it.
pub struct SomeItem;
```

This will result in the following expanded doc comments:

```rust
/// These are some docs about an item. You can embed examples,
/// tests, and other items directly into docs using the
/// following macro:
/// ```ignore
/// fn some_example() {
///   assert_eq!(2 + 2, 4);
///   assert_eq!(2 + 3, 5);
///   assert_eq!(3 + 3, 6);
/// }
/// ```
/// More docs can go here, the example will embed itself inline
/// exactly where you reference it.
pub struct SomeItem;
```

You can embed any item capable of having an attribute macro attached to it.

## Runnable Examples

Note that you can also use the
[`embed_run!`](https://docs.rs/docify/latest/docify/macro.embed_run.html) version of the
macro to make the embedded example compile/run as part of doc tests, which is desirable in
certain situations even though typically the example will already be running/compiling
somewhere else in your project.

## Markdown

A newly added feature allows compiling markdown files with HTML comments
that contain regular `docify::embed!(..)` calls, with the option to compile entire directories
of files or individual files.

In fact, this `README.md` file is automatically compiled whenever `cargo doc` is run on this
crate, resulting in the following codeblock to populate dynamically:

<!-- docify::embed!("examples/samples.rs", some_example) -->

If you look at the [source
code](https://raw.githubusercontent.com/sam0x17/docify/main/.README.docify.md) for
`.README.docify.md`, you'll notice we use the following HTML comment to perform the above
embedding:

```markdown
<!-- docify::embed!("examples/samples.rs", some_example) -->
```

See [`compile_markdown!`](https://docs.rs/docify/latest/docify/macro.compile_markdown.html) for more info.

## More Info

For more documentation, features, and examples, check out [the docs](https://docs.rs/docify)!
