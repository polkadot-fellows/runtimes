#!/bin/bash
set -ex
cargo doc --all-features
cargo test --all-features --workspace
cd core_macros
cargo publish
cd ..
cd core
cargo publish
cd ..
cd macros
cargo publish
cd ..
cargo publish
echo "published successfully."
