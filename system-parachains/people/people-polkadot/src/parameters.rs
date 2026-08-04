// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Governance-mutable Individuality runtime parameters.

use crate::*;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_support::{
	dynamic_params::{dynamic_pallet_params, dynamic_params},
	traits::{EnsureOrigin, EnsureOriginWithArg, Get},
};
use indiv_pallet_resources::types::LongTermStorageAllocation;
use polkadot_runtime_constants::system_parachain::BULLETIN_ID;
use scale_info::TypeInfo;
use sp_statement_store::StatementAllowance;
use xcm::latest::prelude::{Location, Parachain};

/// Dynamic runtime parameters configurable on-chain through [`pallet_parameters`].
///
/// These defaults preserve the Individuality integration's initial economics. Governance can
/// adjust them after deployment without another runtime upgrade.
#[dynamic_params(RuntimeParameters, pallet_parameters::Parameters::<Runtime>)]
pub mod dynamic_params {
	use super::*;

	/// Per-person statement and notification storage limits.
	#[dynamic_pallet_params]
	#[codec(index = 0)]
	pub mod statement_storage {
		#[codec(index = 0)]
		pub static AccountsApiAllowance: StatementAllowanceParameter =
			StatementAllowanceParameter { max_size: 500 * 1024, max_count: 2 };
		#[codec(index = 1)]
		pub static StmtStoreSlotsPerPeriod: u32 = 20;
		#[codec(index = 2)]
		pub static LiteStmtStoreSlotsPerPeriod: u32 = 10;
		#[codec(index = 3)]
		pub static StmtStoreCleanupLimit: u32 = 50;
		#[codec(index = 4)]
		pub static StmtStoreReplacementCooldown: u32 = 60;
		#[codec(index = 5)]
		pub static StmtStoreGraceWindow: u32 = 2 * 24 * 60 * 60;
		#[codec(index = 6)]
		pub static NotificationAllowance: StatementAllowanceParameter =
			StatementAllowanceParameter { max_size: 10 * 1024, max_count: 1 };
		#[codec(index = 7)]
		pub static NotificationSlotsPerPeriod: u8 = 16;
		#[codec(index = 8)]
		pub static LiteNotificationSlotsPerPeriod: u8 = 8;
		#[codec(index = 9)]
		pub static NotificationPeriodDuration: u32 = 24 * 60 * 60;
		#[codec(index = 10)]
		pub static LitePersonStatementLimit: StatementAllowanceParameter =
			StatementAllowanceParameter { max_size: 500 * 1024, max_count: 50 };
		#[codec(index = 11)]
		pub static PersonStatementLimit: StatementAllowanceParameter =
			StatementAllowanceParameter { max_size: 1024 * 1024, max_count: 200 };
	}

	/// Bulletin destination and long-term storage allocation limits.
	#[dynamic_pallet_params]
	#[codec(index = 1)]
	pub mod bulletin_storage {
		#[codec(index = 0)]
		pub static BulletinChainLocation: Location = Location::new(1, [Parachain(BULLETIN_ID)]);
		#[codec(index = 1)]
		pub static LongTermStoragePeriodDuration: u32 = 14 * 24 * 60 * 60;
		#[codec(index = 2)]
		pub static LongTermStorageGraceWindow: u32 = 60 * 60;
		#[codec(index = 3)]
		pub static LongTermStorageClaimsPerPeriod: u8 = 100;
		#[codec(index = 4)]
		pub static LongTermStorageCleanupLimit: u32 = 20;
		#[codec(index = 5)]
		pub static LongTermStorageAllowanceForPeople: LongTermStorageAllocation =
			LongTermStorageAllocation { transactions: 100, bytes: 8 * 1024 * 1024 };
		#[codec(index = 6)]
		pub static LongTermStorageAllowanceForLitePeople: LongTermStorageAllocation =
			LongTermStorageAllocation { transactions: 10, bytes: 4 * 1024 * 1024 };
	}
}

/// A [`StatementAllowance`] stored as a bounded dynamic parameter.
///
/// The SDK type does not implement [`MaxEncodedLen`], which `pallet_parameters` requires for its
/// storage value. Its two `u32` fields have a fixed maximum encoding length, so this equivalent
/// representation makes the value safely governable.
#[derive(
	Clone,
	Default,
	PartialEq,
	Eq,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct StatementAllowanceParameter {
	pub max_size: u32,
	pub max_count: u32,
}

impl From<StatementAllowanceParameter> for StatementAllowance {
	fn from(value: StatementAllowanceParameter) -> Self {
		Self { max_size: value.max_size, max_count: value.max_count }
	}
}

/// Adapts a dynamic [`StatementAllowanceParameter`] to the SDK's allowance type.
pub struct StatementAllowanceGetter<Parameter>(PhantomData<Parameter>);
impl<Parameter: Get<StatementAllowanceParameter>> Get<StatementAllowance>
	for StatementAllowanceGetter<Parameter>
{
	fn get() -> StatementAllowance {
		Parameter::get().into()
	}
}

pub type AccountsApiAllowance =
	StatementAllowanceGetter<dynamic_params::statement_storage::AccountsApiAllowance>;
pub type NotificationAllowance =
	StatementAllowanceGetter<dynamic_params::statement_storage::NotificationAllowance>;
pub type LitePersonStatementLimit =
	StatementAllowanceGetter<dynamic_params::statement_storage::LitePersonStatementLimit>;
pub type PersonStatementLimit =
	StatementAllowanceGetter<dynamic_params::statement_storage::PersonStatementLimit>;

/// The relay-chain Root origin and the Fellowship governance voice may update these parameters.
pub struct DynamicParameterOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParameterOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		_key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		RootOrFellows::ensure_origin(origin.clone()).map(|_| ()).map_err(|_| origin)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin(_key: &RuntimeParametersKey) -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}

impl pallet_parameters::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeParameters = RuntimeParameters;
	type AdminOrigin = DynamicParameterOrigin;
	type WeightInfo = weights::pallet_parameters::WeightInfo<Runtime>;
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for RuntimeParameters {
	fn default() -> Self {
		RuntimeParameters::StatementStorage(
			dynamic_params::statement_storage::Parameters::StmtStoreSlotsPerPeriod(
				dynamic_params::statement_storage::StmtStoreSlotsPerPeriod,
				None,
			),
		)
	}
}
