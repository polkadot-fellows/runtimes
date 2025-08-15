// This file is part of Substrate.

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
use pallet_rc_migrator::types::TranslateAccounts;

impl<T: Config> Pallet<T> {
	pub fn do_receive_recovery_messages(messages: Vec<PortableRecoveryMessage>) -> Result<(), Error<T>> {
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Recovery,
			count: messages.len() as u32,
		});

		for message in &messages {
			Self::do_receive_recovery_message(message.clone());
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Recovery,
			count_good: messages.len() as u32,
			count_bad: 0,
		});

		Ok(())
	}

	pub fn do_receive_recovery_message(message: PortableRecoveryMessage) {
		let message = message.translate_accounts(Self::translate_account_rc_to_ah);

		match message {
			PortableRecoveryMessage::Recoverable((who, config)) => {
				let config: pallet_recovery::RecoveryConfig<_, _, _> = config.into();
				pallet_recovery::Recoverable::<T::KusamaConfig>::insert(who, config);
			},
			PortableRecoveryMessage::ActiveRecoveries((w1, w2, config)) => {
				let config: pallet_recovery::ActiveRecovery<_, _, _> = config.into();
				pallet_recovery::ActiveRecoveries::<T::KusamaConfig>::insert(w1, w2, config);
			},
			PortableRecoveryMessage::Proxy((w1, w2)) => {
				pallet_recovery::Proxy::<T::KusamaConfig>::insert(w1, w2);
			},
		}
	}
}
