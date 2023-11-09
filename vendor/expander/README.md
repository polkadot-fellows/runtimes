
[![crates.io](https://img.shields.io/crates/v/expander.svg)](https://crates.io/crates/expander)
[![CI](https://ci.spearow.io/api/v1/teams/main/pipelines/expander/jobs/master-validate/badge)](https://ci.spearow.io/teams/main/pipelines/expander/jobs/master-validate)
![commits-since](https://img.shields.io/github/commits-since/drahnr/expander/latest.svg)
[![rust 1.51.0+ badge](https://img.shields.io/badge/rust-1.51.0+-93450a.svg)](https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html)

# expander

Expands a proc-macro into a file, and uses a `include!` directive in place.


## Advantages

* Only expands a particular proc-macro, not all of them. I.e. `tracing` is notorious for expanding into a significant amount of boilerplate with i.e. `cargo expand`
* Get good errors when _your_ generated code is not perfect yet


## Usage

In your `proc-macro`, use it like:

```rust

#[proc_macro_attribute]
pub fn baz(_attr: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // wrap as per usual for `proc-macro2::TokenStream`, here dropping `attr` for simplicity
    baz2(input.into()).into()
}


 // or any other macro type
fn baz2(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let modified = quote::quote!{
        #[derive(Debug, Clone, Copy)]
        #input
    };

    let expanded = Expander::new("baz")
        .add_comment("This is generated code!".to_owned())
        .fmt(Edition::_2021)
        .verbose(true)
        // common way of gating this, by making it part of the default feature set
        .dry(cfg!(feature="no-file-expansion"))
        .write_to_out_dir(modified.clone()).unwrap_or_else(|e| {
            eprintln!("Failed to write to file: {:?}", e);
            modified
        });
    expanded
}
```

will expand into

```rust
include!("/absolute/path/to/your/project/target/debug/build/expander-49db7ae3a501e9f4/out/baz-874698265c6c4afd1044a1ced12437c901a26034120b464626128281016424db.rs");
```

where the file content will be

```rust
#[derive(Debug, Clone, Copy)]
struct X {
    y: [u8:32],
}
```


## Exemplary output

An error in your proc-macro, i.e. an excess `;`, is shown as

---

<pre><font color="#26A269"><b>   Compiling</b></font> expander v0.0.4-alpha.0 (/somewhere/expander)
<font color="#F66151"><b>error</b></font><b>: macro expansion ignores token `;` and any following</b>
 <font color="#2A7BDE"><b>--&gt; </b></font>tests/multiple.rs:1:1
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>1</b></font> <font color="#2A7BDE"><b>| </b></font>#[baz::baz]
  <font color="#2A7BDE"><b>| </b></font><font color="#F66151"><b>^^^^^^^^^^^</b></font> <font color="#F66151"><b>caused by the macro expansion here</b></font>
  <font color="#2A7BDE"><b>|</b></font>
  <font color="#2A7BDE"><b>= </b></font><b>note</b>: the usage of `baz::baz!` is likely invalid in item context

<font color="#F66151"><b>error</b></font><b>: macro expansion ignores token `;` and any following</b>
 <font color="#2A7BDE"><b>--&gt; </b></font>tests/multiple.rs:4:1
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>4</b></font> <font color="#2A7BDE"><b>| </b></font>#[baz::baz]
  <font color="#2A7BDE"><b>| </b></font><font color="#F66151"><b>^^^^^^^^^^^</b></font> <font color="#F66151"><b>caused by the macro expansion here</b></font>
  <font color="#2A7BDE"><b>|</b></font>
  <font color="#2A7BDE"><b>= </b></font><b>note</b>: the usage of `baz::baz!` is likely invalid in item context

<font color="#C01C28"><b>error</b></font><b>:</b> could not compile `expander` due to 2 previous errors
<font color="#FFBF00"><b>warning</b></font><b>:</b> build failed, waiting for other jobs to finish...
<font color="#C01C28"><b>error</b></font><b>:</b> build failed
</pre>

---

becomes

---

<pre>
<font color="#26A269"><b>   Compiling</b></font> expander v0.0.4-alpha.0 (/somewhere/expander)
expander: writing /somewhere/expander/target/debug/build/expander-8cb9d7a52d4e83d1/out/baz-874698265c6c.rs
<font color="#F66151"><b>error</b></font><b>: expected item, found `;`</b>
 <font color="#2A7BDE"><b>--&gt; </b></font>/somewhere/expander/target/debug/build/expander-8cb9d7a52d4e83d1/out/baz-874698265c6c.rs:2:42
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>2</b></font> <font color="#2A7BDE"><b>| </b></font>#[derive(Debug, Clone, Copy)] struct A ; ;
  <font color="#2A7BDE"><b>| </b></font>                                         <font color="#F66151"><b>^</b></font>

expander: writing /somewhere/expander/target/debug/build/expander-8cb9d7a52d4e83d1/out/baz-73b3d5b9bc46.rs
<font color="#F66151"><b>error</b></font><b>: expected item, found `;`</b>
 <font color="#2A7BDE"><b>--&gt; </b></font>/somewhere/expander/target/debug/build/expander-8cb9d7a52d4e83d1/out/baz-73b3d5b9bc46.rs:2:42
  <font color="#2A7BDE"><b>|</b></font>
<font color="#2A7BDE"><b>2</b></font> <font color="#2A7BDE"><b>| </b></font>#[derive(Debug, Clone, Copy)] struct B ; ;
  <font color="#2A7BDE"><b>| </b></font>                                         <font color="#F66151"><b>^</b></font>

<font color="#C01C28"><b>error</b></font><b>:</b> could not compile `expander` due to 2 previous errors
<font color="#FFBF00"><b>warning</b></font><b>:</b> build failed, waiting for other jobs to finish...
<font color="#C01C28"><b>error</b></font><b>:</b> build failed
</pre>

---

which shows exactly where in the generated code, the produce of your proc-macro, rustc found an invalid token sequence.

Now this was a simple example, doing this with macros that would expand to multiple tens of thousand lines of
code when expanded with `cargo-expand`, and still in a few thousand that your particular one generates, it's a
life saver to know what caused the issue rather than having to use `eprintln!` to print a unformated
string to the terminal.

> Hint: You can quickly toggle this by using `.dry(true || false)`


# Special handling: `syn`

By default `expander` is built with feature `syndicate` which adds `fn maybe_write_*`
to `struct Expander`, which aids handling of `Result<TokenStream, syn::Error>` for the
commonly used rust parsing library `syn`.

### Reasoning

`syn::Error::new(Span::call_site(),"yikes!").into_token_stream(self)` becomes `compile_error!("yikes!")`
which provides better info to the user (that's you!) than when serializing it to file, since the provided
`span` for the `syn::Error` is printed differently - being pointed to the `compile_error!` invocation
in the generated file is not helpful, and `rustc` can point to the `span` instead.
