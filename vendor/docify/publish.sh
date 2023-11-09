#!/bin/sh
set -ex
cargo build --workspace
cargo test --workspace
cargo doc --example integration
cargo fmt -- --check
cargo doc --workspace --all-features

cd macros
cargo publish
cd ..
cargo publish
