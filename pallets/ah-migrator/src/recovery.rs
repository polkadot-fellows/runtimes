// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

use crate::*;
use pallet_rc_migrator::types::TranslateAccounts;

impl<T: Config> Pallet<T> {
	pub fn do_receive_recovery_messages(
		messages: Vec<PortableRecoveryMessage>,
	) -> Result<(), Error<T>> {
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
