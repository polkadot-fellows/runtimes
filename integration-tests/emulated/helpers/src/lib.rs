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

//! Emulated integration test helpers that have no upstream equivalent.
//!
//! `emulated-integration-tests-common` is the first place to look; test crates import from it
//! directly rather than through this crate. What is kept here has no upstream counterpart yet
//! and is expected to move there eventually.

pub mod burn;
pub mod snowbridge;
