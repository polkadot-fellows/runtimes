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

// TODO: move implementations to the polkadot-sdk.
pub mod tx_payment {
	use super::*;
	use core::marker::PhantomData;
	use frame_support::{
		ensure,
		pallet_prelude::{InvalidTransaction, TransactionValidityError},
		traits::{
			tokens::{Fortitude, Precision, Preservation},
			Defensive, OnUnbalanced, SameOrOther,
		},
	};
	use pallet_transaction_payment::OnChargeTransaction;
	use sp_core::Get;
	use sp_runtime::{
		traits::{DispatchInfoOf, PostDispatchInfoOf, Zero},
		Saturating,
	};

	/// Implements [`OnChargeTransaction`] for [`pallet_transaction_payment`], where the asset class
	/// used to pay the fee is defined with the `A` type parameter (eg. KSM location) and accessed
	/// via the type implementing the [`frame_support::traits::fungibles`] trait.
	///
	/// This implementation with the `fungibles` trait is necessary to set up
	/// [`pallet_asset_conversion_tx_payment`] with the [`SwapCreditAdapter`] type. For both types,
	/// the credit types they handle must be the same, therefore they must be credits of
	/// `fungibles`.
	pub struct FungiblesAdapter<F, A, OU>(PhantomData<(F, A, OU)>);

	impl<T, F, A, OU> OnChargeTransaction<T> for FungiblesAdapter<F, A, OU>
	where
		T: pallet_transaction_payment::Config,
		F: fungibles::Balanced<T::AccountId>,
		A: Get<F::AssetId>,
		OU: OnUnbalanced<fungibles::Credit<T::AccountId, F>>,
	{
		type LiquidityInfo = Option<fungibles::Credit<T::AccountId, F>>;
		type Balance = F::Balance;

		fn withdraw_fee(
			who: &<T>::AccountId,
			_call: &<T>::RuntimeCall,
			_dispatch_info: &DispatchInfoOf<<T>::RuntimeCall>,
			fee: Self::Balance,
			_tip: Self::Balance,
		) -> Result<Self::LiquidityInfo, TransactionValidityError> {
			if fee.is_zero() {
				return Ok(None)
			}

			match F::withdraw(
				A::get(),
				who,
				fee,
				Precision::Exact,
				Preservation::Preserve,
				Fortitude::Polite,
			) {
				Ok(imbalance) => Ok(Some(imbalance)),
				Err(_) => Err(InvalidTransaction::Payment.into()),
			}
		}

		fn correct_and_deposit_fee(
			who: &<T>::AccountId,
			_dispatch_info: &DispatchInfoOf<<T>::RuntimeCall>,
			_post_info: &PostDispatchInfoOf<<T>::RuntimeCall>,
			corrected_fee: Self::Balance,
			_tip: Self::Balance,
			already_withdrawn: Self::LiquidityInfo,
		) -> Result<(), TransactionValidityError> {
			let Some(paid) = already_withdrawn else {
				return Ok(());
			};
			// Make sure the credit is in desired asset id.
			ensure!(paid.asset() == A::get(), InvalidTransaction::Payment);
			// Calculate how much refund we should return.
			let refund_amount = paid.peek().saturating_sub(corrected_fee);
			// Refund to the the account that paid the fees if it was not removed by the
			// dispatched function. If fails for any reason (eg. ED requirement is not met) no
			// refund given.
			let refund_debt =
				if F::total_balance(A::get(), who).is_zero() || refund_amount.is_zero() {
					fungibles::Debt::<T::AccountId, F>::zero(A::get())
				} else {
					F::deposit(A::get(), who, refund_amount, Precision::BestEffort)
						.unwrap_or_else(|_| fungibles::Debt::<T::AccountId, F>::zero(A::get()))
				};
			// Merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid: fungibles::Credit<T::AccountId, F> =
				match paid.offset(refund_debt).defensive_proof("credits should be identical") {
					Ok(SameOrOther::Same(credit)) => credit,
					// Paid amount is fully refunded.
					Ok(SameOrOther::None) => fungibles::Credit::<T::AccountId, F>::zero(A::get()),
					// Should never fail as at this point the asset id is always valid and the
					// refund amount is not greater than paid amount.
					_ => return Err(InvalidTransaction::Payment.into()),
				};
			// No separation for simplicity.
			// In our case the fees and the tips are deposited to the same pot.
			// We cannot call [`OnUnbalanced::on_unbalanceds`] since fungibles credit does not
			// implement `Imbalanced` trait.
			OU::on_unbalanced(adjusted_paid);
			Ok(())
		}
	}
}
