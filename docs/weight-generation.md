# Weight Generation

To generate weights for a runtime

1. Build `chain-spec-generator` with `--profile production --features runtime-benchmarks`
2. Use it to build a chain spec for your runtime, e.g. `./target/production/chain-spec-generator --raw polkadot-local > polkadot-chain-spec.json`
3. Create `file_header.txt`

```text
// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
```

4. `rsync` chain spec/s and the file header to a benchmark machine

5. Build `polkadot` binary from the latest release of `polkadot-sdk` with `--profile production --features runtime-benchmarks --bin polkadot` on the benchmark machine

6. Run on the benchmark machine:

```bash
#!/bin/bash

# Default value is 'polkadot', but you can override it by passing a different value as an argument
CHAIN=${1:-polkadot}

pallets=($(
  ./target/production/polkadot benchmark pallet --list \
    --chain=./$CHAIN-chain-spec.json |
    tail -n+2 |
    cut -d',' -f1 |
    sort |
    uniq
));

mkdir -p ./$CHAIN-weights
for pallet in "${pallets[@]}"; do
  output_file=./$CHAIN-weights/
  # a little hack for pallet_xcm_benchmarks - we want to output them to a nested directory
  if [[ "$pallet" == "pallet_xcm_benchmarks::generic" ]] || [[ "$pallet" == "pallet_xcm_benchmarks::fungible" ]]; then
    mkdir -p ./$CHAIN-weights/xcm
    output_file="${output_file}xcm/${pallet//::/_}.rs"
  fi
  echo "Running benchmarks for $pallet to $output_file"
  ./target/production/polkadot benchmark pallet \
    --chain=./$CHAIN-chain-spec.json \
    --steps=50 \
    --repeat=20 \
    --pallet=$pallet \
    --extrinsic=* \
    --wasm-execution=compiled \
    --heap-pages=4096 \
    --output="$output_file" \
    --header=./file_header.txt
done
```

You probably want to do this inside a `tmux` session or something similar (e.g., `nohup <bench-cmd> &`), as it will take a while (several hours).

7. `rsync` the weights back to your local machine, replacing the existing weights.

8. Manually fix XCM weights by
- Replacing `impl<T: frame_system::Config> xxx::yyy::WeightInfo<T> for WeightInfo<T> {` with `impl<T: frame_system::Config> WeightInfo<T> {`
- Marking all functions `pub(crate)`
- Removing any unused functions

9. Commit the weight changes.

10. Ensure the changes are reasonable. If not installed, `cargo install subweight`, check the weight changes:
   ```
   subweight compare commits \
      --path-pattern "./**/weights/**/*.rs" \
      --method asymptotic \
      --ignore-errors \
      <LATEST-RELEASE-BRANCH> \
      <ACTUAL_BRANCH_WITH_COMMITED_WEIGHTS>`
   ```
   _Hint1: Add `--format markdown --no-color` for markdown-compatible results._

   _Hint2: Change `--path-pattern "./**/weights/**/*.rs"` to e.g. `--path-pattern "./relay/polkadot/weights/**/*.rs"` for a specific runtime._

   _Hint3: Add `--change added changed` to include only relevant changes._

## FAQ

### What benchmark machine spec should I use?

See the [Polkadot Wiki Reference Hardware](https://wiki.polkadot.network/docs/maintain-guides-how-to-validate-polkadot#standard-hardware).

