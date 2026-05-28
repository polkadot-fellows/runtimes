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

frame_benchmarking::define_benchmarks!(
	[frame_system, SystemBench::<Runtime>]
	[cumulus_pallet_parachain_system, ParachainSystem]
	[pallet_timestamp, Timestamp]
	[pallet_balances, Balances]
	[pallet_collator_selection, CollatorSelection]
	[pallet_session, SessionBench::<Runtime>]
	[cumulus_pallet_xcmp_queue, XcmpQueue]
	[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
	[pallet_message_queue, MessageQueue]
	// NOTE: Make sure you point to the individual modules below.
	[pallet_xcm_benchmarks::fungible, XcmBalances]
	[pallet_xcm_benchmarks::generic, XcmGeneric]
	[cumulus_pallet_weight_reclaim, WeightReclaim]
	[pallet_utility, Utility]
);
