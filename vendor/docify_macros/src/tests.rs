use super::*;

#[test]
fn test_export_basic_parsing_valid() {
    export_internal(
        quote!(),
        quote!(
            struct SomeStruct;
        ),
    )
    .unwrap();
    export_internal(
        quote!(some_ident),
        quote!(
            struct SomeStruct;
        ),
    )
    .unwrap();
    export_internal(
        quote!(SomethingSomething),
        quote!(
            struct SomeStruct;
        ),
    )
    .unwrap();
}

#[test]
fn test_export_basic_parsing_invalid() {
    assert!(export_internal(
        quote!(),
        quote!(
            struct SomeStruct
        ),
    )
    .is_err());
    assert!(export_internal(
        quote!(something as something),
        quote!(
            struct SomeStruct;
        ),
    )
    .is_err());
    assert!(export_internal(
        quote!(something something),
        quote!(
            struct SomeStruct;
        ),
    )
    .is_err());
}

#[test]
fn test_compile_markdown_dir() {
    compile_markdown_dir("fixtures", "test_bin").unwrap();
}

#[test]
fn test_compile_markdown_valid() {
    compile_markdown_internal(quote!("fixtures", "test_bin")).unwrap();
    compile_markdown_internal(quote!("fixtures/file_1.md", "test_bin/alternate_output.md"))
        .unwrap();
    assert_eq!(
        compile_markdown_internal(quote!("fixtures/file_1.md"))
            .unwrap()
            .to_string(),
        "\"# This is a markdown file\\n\\n```rust\\nstruct \
        Something;\\n```\\n<!-- this is a comment -->\\n\\n`\
        ``rust\\nfn some_fn() {\\n    println!(\\\"foo\\\");\
        \\n}\\n```\\n\\nSome text this is some text\\n\""
    );
}

#[test]
fn test_compile_markdown_invalid() {
    assert!(compile_markdown_internal(quote!("&97298", "79*&(")).is_err());
    assert!(compile_markdown_internal(quote!("&97298", "test_bin")).is_err());
    assert!(compile_markdown_internal(quote!("fixtures")).is_err());
    assert!(compile_markdown_internal(quote!("fixtures/file_1.md", "test_bin")).is_err());
    assert!(compile_markdown_internal(quote!("something", "")).is_err());
    assert!(compile_markdown_internal(quote!("", "something")).is_err());
    assert!(compile_markdown_internal(quote!("", "")).is_err());
}

#[test]
fn test_compile_markdown_source_valid() {
    assert_eq!(
        compile_markdown_source(
            "this is some markdown\n\
            this is some more markdown\n\
            # this is a title\n\
            <!-- docify::embed!(\"fixtures/file.rs\", some_fn) -->\n\
            this is some more text\n",
        )
        .unwrap(),
        "this is some markdown\n\
        this is some more markdown\n\
        # this is a title\n\
        ```rust\n\
        fn some_fn() {\n    \
            println!(\"foo\");\n\
        }\n\
        ```\n\
        this is some more text\n"
    );
    assert!(compile_markdown_source(
        "this is some markdown\n\
        this is some more markdown\n\
        # this is a title\n\
        <!-- docify::embed!(\"fixtures/file.rs\", some_other_fn) -->\n\
        this is some more text\n",
    )
    .unwrap()
    .contains("bar"));
    assert!(compile_markdown_source(
        "this is some markdown\n\
        this is some more markdown\n\
        # this is a title\n\
        <!--docify::embed!(\"fixtures/file.rs\", some_other_fn) -->\n\
        this is some more text\n",
    )
    .unwrap()
    .contains("bar"));
    assert!(compile_markdown_source(
        "this is some markdown\n\
        this is some more markdown\n\
        # this is a title\n\
        <!-- docify::embed!(\"fixtures/file.rs\", some_fn)-->\n\
        this is some more text\n",
    )
    .unwrap()
    .contains("foo"));
    assert!(compile_markdown_source(
        "this is some markdown\n\
        this is some more markdown\n\
        # this is a title\n\
        <!--docify::embed!(\"fixtures/file.rs\", some_fn)-->\n\
        this is some more text\n",
    )
    .unwrap()
    .contains("foo"));
    assert!(compile_markdown_source(
        "<!-- docify::embed!(\"fixtures/file.rs\", some_fn) --> this is some more text\n",
    )
    .unwrap()
    .ends_with("more text\n"));
    assert!(compile_markdown_source(
        "prefix<!-- docify::embed!(\"fixtures/file.rs\", some_fn) -->",
    )
    .unwrap()
    .starts_with("prefix"));
}

#[test]
fn test_compile_markdown_source_invalid() {
    assert!(compile_markdown_source(
        "# this is a title\n\
        <!-- docify:embed!(\"fixtures/file.rs\", some_fn) -->\n\
        this is some more text\n",
    )
    .is_err());
    assert!(compile_markdown_source(
        "# this is a title\n\
        <!-- docify::em!(\"fixtures/file.rs\", some_fn) -->\n\
        this is some more text\n",
    )
    .is_err());
    assert!(compile_markdown_source(
        "# this is a title\n\
        <!-- docify -->\n\
        this is some more text\n",
    )
    .is_err());
}
