#![no_std]

use macro_magic_core::*;
use proc_macro::TokenStream;

/// Can be applied to any item. Doing so will make the tokens for this item available for
/// import by the other macros in this crate.
///
/// An optional argument can be provided specifying an override export name to use instead of
/// the regular name of the item, such as `#[export_tokens(MyCoolName)]` or
/// `#[export_tokens(some_name)]`. Syntactically this name is parsed as a `syn::Ident` and is
/// then normalized by converting to snake_case. Note that because of this, `MyCoolName` would
/// collide with `my_cool_name`, resulting in a compiler error if these items are being
/// exported from the same module.
///
/// Note that some types of items, namely `syn::ItemForeignMod`, `syn::ItemUse`,
/// `syn::ItemImpl`, and `syn::Item::Verbatim`, do not have an inherent concept of a naming
/// ident, and so for these items specifying an override name is required or you will get a
/// compiler error. This also applies to `macro_rules!` definitions that do not specify a name.
///
/// Note also that while you can presently _attach_ `#[export_tokens]` to anything attributes
/// can be attached to, some of these items do not exist at the module path level, and
/// therefore cannot be accessed. You should only attach `#[export_tokens]` to items that are
/// accessible by path from the location where you wish to use their tokens.
///
/// ## Examples
///
/// Applied to a regular function definition:
/// ```ignore
/// #[export_tokens]
/// fn my_function() {
///     println!("hey");
/// }
/// ```
///
/// Applied to a module:
/// ```ignore
/// #[export_tokens]
/// mod my_module() {
///     fn some_fn() {
///         stuff();
///     }
/// }
/// ```
///
/// Applied to an `impl` requiring an override name:
/// ```ignore
/// #[export_tokens(impl_my_trait_for_my_item)]
/// impl MyTrait for MyItem {
///     fn something() {
///         do_stuff();
///     }
/// }
/// ```
///
/// Applied to a struct, but specifying an override name:
/// ```ignore
/// #[export_tokens(SomeOtherName)]
/// struct MyStruct {
///     field: u32,
/// }
/// ```
///
/// Previously it was possible to access `#[export_tokens]` items defined in
/// private/inaccessible contexts, however this was removed in 0.4.x.
#[proc_macro_attribute]
pub fn export_tokens(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match export_tokens_internal(attr, tokens, true) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Like [`#[export_tokens]`](`macro@export_tokens`) but does not emit the tokens of the
/// attached item locally.
///
/// This is useful for scenarios where the local tokens would not compile anyway locally,
/// and/or do not need to be used locally.
#[proc_macro_attribute]
pub fn export_tokens_no_emit(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match export_tokens_internal(attr, tokens, false) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Creates an attribute proc macro that is an alias for
/// [`#[export_tokens]`](`macro@export_tokens`).
///
/// Simply pass an ident to this proc macro, and an alias for
/// [`#[export_tokens]`](`macro@export_tokens`) will be created with the specified name.
///
/// Can only be used within a proc macro crate.
#[proc_macro]
pub fn export_tokens_alias(tokens: TokenStream) -> TokenStream {
    match export_tokens_alias_internal(tokens, true) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Like [`#[export_tokens]`](`macro@export_tokens`) but intead creates an alias for
/// [`#[export_tokens_no_emit]`](`macro@export_tokens_no_emit`)
///
/// Can only be used within a proc macro crate.
#[proc_macro]
pub fn export_tokens_alias_no_emit(tokens: TokenStream) -> TokenStream {
    match export_tokens_alias_internal(tokens, false) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// "Forwards" the tokens of the specified exported item (specified by path as the first arg)
/// to the specified proc or `macro_rules!` macro (specified by path as the second arg).
///
/// This is used internally as the basis for many of the other macros in this crate, but can
/// also be useful in its own right in certain situations.
///
/// Note that the referenced item _must_ have the [`#[export_tokens]`][`macro@export_tokens`]
/// attribute attached to it, or this will not work.
///
/// There is also an optional third argument called "extra" which allows you to forward
/// arbitrary data to the target macro. This is used by
/// [`#[import_tokens_attr]`](`macro@import_tokens_proc`) to pass the tokens for the attached
/// item in addition to the tokens for the external item.
///
/// ## Example
///
/// ```ignore
/// #[macro_export]
/// macro_rules! receiver {
///     ($tokens:item) => {
///         stringify!($tokens)
///     };
/// }
///
/// let result = forward_tokens!(LionStruct, receiver);
/// assert_eq!(result, "struct LionStruct {}");
/// ```
#[proc_macro]
pub fn forward_tokens(tokens: TokenStream) -> TokenStream {
    match forward_tokens_internal(tokens) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Allows you to import the tokens of an external item marked with
/// [`#[export_tokens]`][`macro@export_tokens`] whose path is already known at compile-time
/// without having to do any additional parsing.
///
/// If the path of the item is defined by the downstream programmer and is not "hard-coded",
/// then you should instead use [`#[import_tokens_attr]`](`macro@import_tokens_attr`) /
/// [`#[import_tokens_proc]`](`macro@import_tokens_proc`).
///
/// The macro lets you define as its argument a let variable declaration that will expand to
/// that variable being set to the tokens of the specified external item at compile-time.
///
/// For example:
///
/// ```ignore
/// import_tokens!(let tokens = external_crate::SomeItem);
/// ```
///
/// will expand such that a `tokens` variable will be created containing the tokens for the
/// `SomeItem` item that exists in an external crate. For this to work,
/// `external_crate::SomeItem` must be the path of an item that has
/// [`#[export_tokens]`][`macro@export_tokens`] attached to it. The imported tokens wil be of
/// type `TokenStream2`.
///
/// Unfortunately this macro isn't very useful, because it is quite rare that you already know
/// the path of the item you want to import _inside_ your proc macro. Note that having the
/// _tokens_ for the path you want isn't the same as having those tokens already expanded in
/// the current context.
///
/// That said, this can be quite useful for scenarios where for whatever reason you have an
/// item with a set-in-stone path whose tokens you need to access at compile time.
#[proc_macro]
pub fn import_tokens(tokens: TokenStream) -> TokenStream {
    match import_tokens_internal(tokens) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// An attribute macro that can be attached to a proc macro function definition that will cause
/// it to receive the tokens of the external item referred to by its argument as input to your
/// proc macro.
///
/// For example:
///
/// ```ignore
/// #[import_tokens_proc]
/// #[proc_macro]
/// pub fn my_macro(tokens: TokenStream) -> TokenStream {
///     // `tokens` will contain the tokens of
///     let item = parse_macro_input!(tokens as Item);
///     // you can now do stuff with `item`
///     // ...
/// }
/// ```
///
/// Which you could use like this:
///
/// ```ignore
/// my_macro!(some_crate::some_item);
/// ```
///
/// In this case the `tokens` variable will contain the tokens for the `some_crate::some_item`
/// item, as long as it has been marked with [`#[export_tokens]`][`macro@export_tokens`].
///
/// Note that this attribute can only be used within a proc macro crate.
///
/// ## Overriding [`MACRO_MAGIC_ROOT`]:
///
/// You can also provide a module path as an optional argument to this attribute macro and that
/// path will be used as the override for [`MACRO_MAGIC_ROOT`] within the context of code
/// generated by this attribute. Instead of a `Path`, you are also free to provide any `Expr`
/// that evaluates to something compatible with [`Into<String>`] so you can dynamically
/// generate this path based on `format!` and other string manipulation machinery, if
/// necessary.
///
/// Here is an example of providing a `Path` as the override for [`MACRO_MAGIC_ROOT`]:
///
/// ```ignore
/// #[import_tokens_proc(my_crate::__private::macro_magic)]
/// pub fn my_macro(tokens: TokenStream) -> TokenStream {
///     // ..
/// }
/// ```
///
/// and here is an example of providing an [`Into<String>`]-compatible `Expr` as the override
/// for [`MACRO_MAGIC_ROOT`]:
///
/// ```ignore
/// #[import_tokens_proc(format!("{}::__private::macro_magic", generate_crate_access_2018("my_crate")))]
/// pub fn my_macro(tokens: TokenStream) -> TokenStream {
///     // ..
/// }
/// ```
#[proc_macro_attribute]
pub fn import_tokens_proc(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match import_tokens_proc_internal(attr, tokens) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Can be attached to an attribute proc macro function, causing it to receive the tokens for
/// the external item referred to by the path provided as the `attr` / first argument to the
/// attribute macro.
///
/// The item whose path is provided as the `attr` / first argument _must_ have the
/// [`#[export_tokens]`][`macro@export_tokens`] attribute attached to it, or this will not
/// work.
///
/// For example:
///
/// ```ignore
/// #[import_tokens_attr]
/// #[proc_macro_attribute]
/// pub fn my_attribute(attr: TokenStream, tokens: TokenStream) -> TokenStream {
///     let external_item = parse_macro_input!(attr as Item);
///     let attached_item = parse_macro_input!(tokens as Item);
///     // ...
/// }
/// ```
///
/// Which could then be used like:
///
/// ```ignore
/// #[my_attribute(path::to::AnItem)]
/// mod my_mod {
///     // ...
/// }
/// ```
///
/// This would result in the `external_item` variable having the parsed tokens of the external
/// `path::to::AnItem` item, and the `attached_item` variable having the parsed tokens of the
/// item the attribute is attached to (`my_mod`) as usual.
///
/// This allows for the creation of extremely powerful attribute macros that take in an export
/// tokens path as their `attr` and internally receive the tokens for that external item. For
/// example you could write an attribute macro that combines two modules or two structs
/// together, among many other things. Custom parsing, covered below, makes these capabilities
/// even more powerful.
///
/// ## Overriding [`MACRO_MAGIC_ROOT`]
///
/// You can also provide a module path as an optional argument to this attribute macro and that
/// path will be used as the override for [`MACRO_MAGIC_ROOT`] within the context of code
/// generated by this attribute. Instead of a `Path`, you are also free to provide any `Expr`
/// that evaluates to something compatible with [`Into<String>`] so you can dynamically
/// generate this path based on `format!` and other string manipulation machinery, if
/// necessary.
///
/// Here is an example of providing a `Path` as the override for [`MACRO_MAGIC_ROOT`]:
///
/// ```ignore
/// #[import_tokens_attr(my_crate::__private::macro_magic)]
/// pub fn my_macro(attr: TokenStream, tokens: TokenStream) -> TokenStream {
///     // ..
/// }
/// ```
///
/// and here is an example of providing an [`Into<String>`]-compatible `Expr` as the override
/// for [`MACRO_MAGIC_ROOT`]:
///
/// ```ignore
/// #[import_tokens_proc(format!("{}::__private::macro_magic", generate_crate_access_2018("my_crate")))]
/// pub fn my_macro(attr: TokenStream, tokens: TokenStream) -> TokenStream {
///     // ..
/// }
/// ```
///
///
/// ## Optional Feature: `#[with_custom_parsing(..)]`
///
/// By default, [`#[import_tokens_attr]`](`macro@import_tokens_attr`)-based attribute macros
/// expect the foreign item path to be passed directly as the only argument to the resulting
/// macro. Sometimes, however, it is desirable to support multiple arguments, or otherwise
/// implement some kind of custom parsing that determines how the foreign path is obtained. You
/// can do this by attaching the optional attribute
/// [`#[with_custom_parsing(..)]`](`macro@with_custom_parsing`) to the same proc macro
/// attribute definition that you attached `#[import_tokens_attr]` to.
///
/// This optional attribute takes one argument, which should be the path to a struct that
/// implements `syn::parse::Parse`, `quote::ToTokens`, and [`ForeignPath`]. To access the
/// tokens for your custom parsed input, you can use the magic variable `__custom_tokens:
/// TokenStream` anywhere in your attribute proc macro.
///
/// Here is a full example:
///
/// ```ignore
/// #[derive(Parse)]
/// struct MyCustomParsing {
///     foreign_path: syn::Path,
///     _comma: syn::token::Comma,
///     custom_path: syn::Path,
/// }
///
/// impl ToTokens for MyCustomParsing {
///     fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
///         tokens.extend(self.foreign_path.to_token_stream());
///         tokens.extend(self._comma.to_token_stream());
///         tokens.extend(self.custom_path.to_token_stream());
///     }
/// }
///
/// impl ForeignPath for MyCustomParsing {
///     fn foreign_path(&self) -> &syn::Path {
///         &self.foreign_path
///     }
/// }
/// #[import_tokens_attr]
/// #[with_custom_parsing(MyCustomParsing)]
/// #[proc_macro_attribute]
/// pub fn my_attribute(attr: TokenStream, tokens: TokenStream) -> TokenStream {
///     let external_item = parse_macro_input!(attr as Item);
///     let attached_item = parse_macro_input!(tokens as Item);
///     let custom_parsed_item = parse_macro_input!(__custom_tokens as MyCustomParsing);
///     // ...
/// }
/// ```
///
/// Usage would look like:
/// ```ignore
/// #[my_attribute(foreign::path, some_other::path)]
/// struct SomeItem {}
/// ```
///
/// This is just an example, you could implement the parsing any way you want, maybe even using
/// something that isn't initially a `syn::Path` but is transformed into one. The possibilities
/// are endless.
///
/// ## Notes
///
/// * See `tests.rs` for more examples.
/// * Can only be used within a proc macro crate.
/// * A handy `__source_path: TokenStream` variable is also injected into your proc macro
///   function definition which provides access to the original `syn::Path` that was provided
///   as the path for the foreign item before its tokens were imported. You can access this
///   directly simply by referring to `__source_path`. This should parse to a `syn::Path`.
/// * When using the custom parsing feature, you can also access the original tokens for the
///   input attribute within your proc macro body using the magic variable `__custom_tokens`.
///   For more information and an example see [`macro@with_custom_parsing`].
#[proc_macro_attribute]
pub fn import_tokens_attr(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match import_tokens_attr_internal(attr, tokens) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// To be used in tandem with [`#[import_tokens_attr]`](`macro@import_tokens_attr`)
///
/// Example:
/// ```ignore
/// #[import_tokens_attr]
/// #[with_custom_parsing(MyCustomParsing)]
/// #[proc_macro_attribute]
/// pub fn my_attribute(attr: TokenStream, tokens: TokenStream) -> TokenStream {
///     let external_item = parse_macro_input!(attr as Item);
///     let attached_item = parse_macro_input!(tokens as Item);
///     let custom_parsed_item = parse_macro_input!(__custom_tokens as MyCustomParsing);
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn with_custom_parsing(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    match with_custom_parsing_internal(attr, tokens, "import_tokens_attr") {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Deprecated: No-op
#[deprecated(
    note = "`use_attr` is no longer needed for importing or re-exporting, implementation is no-op, it can be removed safely"
)]
#[proc_macro_attribute]
pub fn use_attr(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    tokens
}

/// Deprecated: No-op
#[deprecated(
    note = "`use_proc` is no longer needed for importing or re-exporting, implementation is no-op, it can be removed safely"
)]
#[proc_macro_attribute]
pub fn use_proc(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    tokens
}

/// A helper macro used by [`macro@import_tokens`]. Hidden from docs.
#[doc(hidden)]
#[proc_macro]
pub fn import_tokens_inner(tokens: TokenStream) -> TokenStream {
    match import_tokens_inner_internal(tokens) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// A helper macro used by [`macro@forward_tokens`]. Hidden from docs.
#[doc(hidden)]
#[proc_macro]
pub fn forward_tokens_inner(tokens: TokenStream) -> TokenStream {
    match forward_tokens_inner_internal(tokens) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
