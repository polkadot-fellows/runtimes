
[![crates.io](https://img.shields.io/crates/v/fatality.svg)](https://crates.io/crates/fatality)
[![CI](https://ci.spearow.io/api/v1/teams/main/pipelines/fatality/jobs/master-validate/badge)](https://ci.spearow.io/teams/main/pipelines/fatality/jobs/master-validate)
![commits-since](https://img.shields.io/github/commits-since/drahnr/fatality/latest.svg)
[![rust 1.51.0+ badge](https://img.shields.io/badge/rust-1.51.0+-93450a.svg)](https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html)

# fatality

A generative approach to creating _fatal_ and _non-fatal_ errors.

The generated source utilizes `thiserror::Error` derived attributes heavily,
and any unknown annotations will be passed to that.

## Motivation

For large scale mono-repos, with subsystems it eventually becomes very tedious to `match`
against nested error variants defined with `thiserror`. Using `anyhow` or `eyre` - while it being an application - also comes with an unmanagable amount of pain for medium-large scale code bases.

`fatality` is a solution to this, by extending `thiserror::Error` with annotations to declare certain variants as `fatal`, or `forward` the fatality extraction to an inner error type.

Read on!

## Usage

`#[fatality]` currently provides a `trait Fatality` with a single `fn is_fatal(&self) -> bool` by default.

Annotations with `forward` require the _inner_ error type to also implement `trait Fatality`.

Annotating with `#[fatality(splitable)]`, allows to split the type into two sub-types, a `Jfyi*` and a `Fatal*` one via `fn split(self) -> Result<Self::Jfyi, Self::Fatal>`. If `splitable` is annotated.

The derive macro implements them, and can defer calls, based on `thiserror` annotations, specifically
`#[source]` and `#[transparent]` on `enum` variants and their members.

```rust
/// Fatality only works with `enum` for now.
/// It will automatically add `#[derive(Debug, thiserror::Error)]`
/// annotations.
#[fatality]
enum OhMy {
    #[error("An apple a day")]
    Itsgonnabefine,

    /// Forwards the `is_fatal` to the `InnerError`, which has to implement `trait Fatality` as well.
    #[fatal(forward)]
    #[error("Dropped dead")]
    ReallyReallyBad(#[source] InnerError),

    /// Also works on `#[error(transparent)]
    #[fatal(forward)]
    #[error(transparent)]
    Translucent(InnerError),


    /// Will always return `is_fatal` as `true`,
    /// irrespective of `#[error(transparent)]` or
    /// `#[source]` annotations.
    #[fatal]
    #[error("So dead")]
    SoDead(#[source] InnerError),
}
```

```rust
#[fatality(splitable)]
enum Yikes {
    #[error("An apple a day")]
    Orange,

    #[fatal]
    #[error("So dead")]
    Dead,
}

fn foo() -> Result<[u8;32], Yikes> {
    Err(Yikes::Dead)
}

fn i_call_foo() -> Result<(), FatalYikes> {
    // availble via a convenience trait `Nested` that is implemented
    // for any `Result` whose error type implements `Split`.
    let x: Result<[u8;32], Jfyi> = foo().into_nested()?;
}

fn i_call_foo_too() -> Result<(), FatalYikes> {
    if let Err(jfyi_and_fatal_ones) = foo() {
        // bail if bad, otherwise just log it
        log::warn!("Jfyi: {:?}", jfyi_and_fatal_ones.split()?);
    }
}
```

## Roadmap

* [ ] Optionally reduce the marco overhead, replace `#[fatal($args)]#[error(..` with `#[fatal($args;..)]` and generate the correct `#[error]` annotations for `thiserror`.
* [x] Add an optional arg to `finality`: `splitable` determines if a this is the root error that shall be handled, and hence should be splitable into two enums `Fatal` and `Jfyi` errors, with `trait Split` and `fn split() -> Result<Jfyi, Fatal> {..}`.
* [ ] Allow annotations for `struct`s as well, to be all fatal or informational.
