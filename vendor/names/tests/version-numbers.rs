#[test]
fn test_readme_deps() {
    version_sync::assert_markdown_deps_updated!("README.md");

    // TODO(fnichol): Ideally a code block updating tool can keep this up to date
    // version_sync::assert_contains_regex!("README.md", r#"{name} --help$\n^{name} {version}$"#);
}

#[test]
fn test_html_root_url() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}
