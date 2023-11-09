/// These are some docs
/// These are some more docs
#[doc = docify::embed!("examples/samples.rs", MyCoolStruct)]
/// even more here
#[allow(unused)]
pub struct SomeItem;

/// These are some docs
/// These are some more docs
#[doc = docify::embed!("examples/samples.rs", SomeImpl)]
/// even more here
#[allow(unused)]
pub struct TestExplicitName;

/// Some more docs
#[doc = docify::embed!("examples/samples.rs", some_random_test)]
#[doc = docify::embed!("examples/samples.rs", test_with_custom_name)]
pub struct MultipleEmbeds;

#[doc = docify::embed!("examples/samples.rs", Duplicate)]
pub struct Duplicates;

/// This example will actually run in rust docs
#[doc = docify::embed_run!("examples/samples.rs", runnable)]
pub struct Runnable;

/// This example runs a test
#[doc = docify::embed_run!("examples/samples.rs", test_with_custom_name)]
pub struct RunnableTest;

/// This example has comments
#[doc = docify::embed!("examples/samples.rs", some_complex_example)]
pub struct LineComments;

/// This example has wonky formatting that would normally get auto-correct by rustfmt but that
/// has been manually allowed using a flag
#[doc = docify::embed!("examples/samples.rs", wonky_comment_example)]
pub struct WonkyComments;

/// This example caused an error regression
#[doc = docify::embed!("macros/fixtures/complicated.rs", successful_unstake)]
pub struct KianRegression;

/// This has always worked
#[doc = docify::embed!("examples/samples.rs", test_with_normal_ordering)]
pub struct TestWithNormalOrdering;

/// This example originally failed with <https://github.com/sam0x17/docify/issues/1>
#[doc = docify::embed!("examples/samples.rs", test_with_weird_ordering)]
pub struct TestWithWeirdOrdering;

/// This example has docify-related export keywords directly in the doc comments. They should
/// not get stripped:
#[doc = docify::embed!("examples/samples.rs", docify_keyword_in_comments)]
pub struct TestWithWeirdOrderingAndKeywords;

/// This example pertains to <https://github.com/sam0x17/docify/issues/2>
#[doc = docify::embed!("examples/samples.rs", oliver_substrate_example_2)]
pub struct OliverExample2;

/// This will compile all markdown files in the `markdown_source` directory to `markdown_bin`
/// when `cargo doc` is run, handling any doc embed calls as it goes
#[cfg(doc)]
docify::compile_markdown!("examples/markdown_source", "examples/markdown_bin");

fn main() {}
