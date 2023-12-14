# Runtimes

This repository houses the code required to build the runtimes for Polkadot, Kusama, and their System-Parachains. Its maintenance is overseen by the Fellowship, as decreed by the Polkadot and Kusama Governance. The primary objective is to provide excellent code, which can subsequently be enacted on-chain through a decentralized referendum.

## Structure

Each leaf folder contains one runtime crate:

<!-- Run "tree -I 'target' -d -L 3" and then delete some folders from Polkadot and Kusama. -->

```pre
├── relay
│   ├── kusama
│   └── polkadot
└── system-parachains
    ├── asset-hubs
    │   ├── asset-hub-kusama
    │   └── asset-hub-polkadot
    ├── bridge-hubs
    │   ├── bridge-hub-kusama
    │   └── bridge-hub-polkadot
    ├── collectives
    │   └── collectives-polkadot
    └── gluttons
        └── glutton-kusama
```

## Approval rights

The approval rights are configured in [`review-bot.yml`](.github/review-bot.yml). The rights are configured as:

- All files in `.github` require two approvals from Fellowship members of rank 4 or higher.
- `CHANGELOG.md`, `relay/*` or `system-parachains/*` require four approvals from Fellowship members of rank 3 or higher.
- All other files require the approval from one Fellowship member of rank 2 or higher.

The review-bot uses the on-chain identity to map from a GitHub account to a Fellowship member. This requires that each Fellowship member add their GitHub handle to their on-chain identity. Check [here](docs/on-chain-identity.md) for instructions.

# Working on Pull Requests

To merge a pull request, we use [Auto Merge Bot](https://github.com/paritytech/auto-merge-bot).

To use it, write a comment in a PR that says:

> `/merge`

This will enable [`auto-merge`](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/automatically-merging-a-pull-request) in the Pull Request (or merge it if it is ready to merge).

The automation can be triggered by the author of the PR or any fellow whose GitHub handle is part of their identity.

# Release process

Releases are automatically pushed on commits merged to master that fulfill the following requirements:

- The [`CHANGELOG.md`](CHANGELOG.md) file was modified.
- The latest version (the version at the top of the file) in [`CHANGELOG.md`](CHANGELOG.md) has no tag in the repository.

The release process is building all runtimes and then puts them into a release in this github repository.

The format of [`CHANGELOG.md`](CHANGELOG.md) is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

# Weight Generation

To generate weights for a runtime

1. Build `chain-spec-generator` with `--features runtime-benchmarks`
2. Use it to build a chain spec for your runtime, e.g. `./target/release/chain-spec-generator --raw polkadot-local > polkadot-chain-spec.json`
3. Create `file_header.txt`

```text
// Copyright (C) Parity Technologies (UK) Ltd.
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

5. Build `polkadot-sdk` with `--features runtime-benchmarks` on the benchmark machine

6. Create output directories for the weights on the benchmark machine

7. Run on the benchmark machine:

```bash
for pallet in \
  frame_system \
  # other pallets you want to benchmark
  pallet_proxy; do
  echo "Running benchmark for $pallet"
  ./target/release/polkadot benchmark pallet \
    --chain=/path/to/chain-spec.json \
    --steps 50 \
    --repeat 20 \
    --pallet=$pallet \
    --extrinsic=* \
    --wasm-execution=compiled \
    --heap-pages=4096 \
    --output /path/to/runtime/weights/directory \
    --header /path/to/file_header.txt
done
```

You probably want to do this inside a `tmux` session or similar, as it will take a while.

7a. If benchmarking `pallet_alliance`

Rename `fn add_scrupulous_items` to `fn add_unscrupulous_items` (see `https://github.com/paritytech/polkadot-sdk/pull/2173`).

8. `rsync` the weights back to your local machine

## FAQ

### What benchmark machine spec should I use?

Google Cloud `n2-standard-8` or equivalent.

### Why not use `--pallet=*` when generating benchmarks?

XCM benchmarks are broken until runtimes repo gets <https://github.com/paritytech/polkadot-sdk/pull/2288>. Once this is fixed, we can use `--pallet=*` instead of a list of pallets.

### Why is this such a manual task?

It shouldn't be. Now that we have a process to follow, it should be automated by a script that takes as input:

1. List of runtimes & pallets to bench
2. SSH credentials for a benchmark machine
3. Output dir

and writes the weights to the local output dir.
