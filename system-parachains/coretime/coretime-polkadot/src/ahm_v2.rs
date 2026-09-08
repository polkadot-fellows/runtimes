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

//! AHM v2 migration wiring: the Coretime-chain side of receiving the relay chain's remaining
//! state.
//!
//! Behind the `ahm-v2` feature until the migration's storage layout stops changing — the relay
//! chain's stage machine grows a variant per data stage, and the receiving calls grow with it, so
//! neither has any business in a shipped runtime yet. The feature is what lets the integration
//! tests drive the real runtime meanwhile.

use crate::{xcm_config::XcmRouter, Runtime};

impl pallet_ct_migrator::Config for Runtime {
	type RuntimeEvent = crate::RuntimeEvent;
	type SendXcm = XcmRouter;
}
