# Contributing to bounded-vec

Thanks for wanting to contribute! There are many ways to contribute and we
appreciate any level you're willing to do.

## Process
As a heads up, we'll be running your PR through the following CI jobs:
- warnings turned to compile errors
- `cargo test`
- `rustfmt` (we recommend to setup running `rustfmt` on file save)
- `clippy`

## Releasing
When we're ready to release, a project owner should do the following:

- Create(checkout) a release branch (naming convention `release/vX-Y-Z`);
- install or update "cargo release" (via `cargo install cargo-release`);
- `cargo release minor --no-dev-version -vv` if you intend to bump a minor version 
  or change `minor` to `major` to bump a major version, `patch` to bump a patch/hotfix version 
- check the output and run it with `--execute` to actually execute it(make sure that you have api token from `crates.io`, otherwise run `cargo login` first);
- Merge release branch into develop
- Merge release branch into master
- Make a github release

