// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Integration-test harness for the AHM v2 migration.
//!
//! The Relay Chain and the Coretime chain are loaded from snapshots of live network state and
//! driven by hand: blocks are produced by calling the relevant hooks directly and DMP/UMP messages
//! are shuttled between the chains manually. There are no nodes and no networking involved, so the
//! whole suite runs in seconds, and the same tests run against Polkadot (default) or Kusama
//! (`--features kusama`).
//!
//! See this crate's `README.md` for how to obtain the snapshots.

#![cfg(test)]

pub mod mock;
pub mod tests;
