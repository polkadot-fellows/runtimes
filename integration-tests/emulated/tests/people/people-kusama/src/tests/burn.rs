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

use crate::*;

#[test]
fn accumulated_funds_are_burnt_on_asset_hub() {
	use people_kusama_runtime::System;
	integration_tests_helpers::burn::test_accumulated_funds_are_burnt_on_asset_hub::<
		PeopleKusama,
		AssetHubKusama,
	>(AssetHubKusamaSender::get(), System::set_block_number);
}
