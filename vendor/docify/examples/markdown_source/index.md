# This is a book

Markdown is really cool. It lets you write files that begin with a `.md` extension. With
`docify` you can enhance these files by adding dynamic embed tags such as the following:

```markdown
# Some Markdown
<!-- docify::embed!("examples/samples.rs", some_random_test) -->
This is some more markdown
```

The following is the same embed from above, but live (this should show the embedded item!):

<!-- docify::embed!("examples/samples.rs", some_random_test) -->

If you see code above, then it worked!
