// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Governance-mutable runtime parameters.

use crate::{ExistentialDeposit, *};
use frame_support::{
	dynamic_params::{dynamic_pallet_params, dynamic_params},
	traits::{ConstU32, EnsureOrigin, EnsureOriginWithArg},
};
use indiv_pallet_resources::types::LongTermStorageAllocation;
use indiv_support::parameters::{
	AtLeast, AtLeastOne, AtMost, BenchmarkMax, SaturatingSubOne, StatementAllowanceGetter,
};
pub use indiv_support::parameters::StatementAllowanceParameter;
use polkadot_runtime_constants::system_parachain::BULLETIN_ID;
use xcm::latest::prelude::{Location, Parachain};

const SECONDS_PER_DAY: u32 = 24 * 60 * 60;

/// The largest statement-store cleanup batch covered by the current resources weights.
pub const STMT_STORE_CLEANUP_LIMIT_CAP: u32 = 50;

/// The largest long-term-storage cleanup batch covered by the current resources weights.
pub const LONG_TERM_STORAGE_CLEANUP_LIMIT_CAP: u32 = 20;

/// Dynamic runtime parameters configurable on-chain through [`pallet_parameters`].
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
		pub static StmtStoreGraceWindow: u32 = 24 * 60 * 60;
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
			StatementAllowanceParameter { max_size: 50 * 1024, max_count: 15 };
		#[codec(index = 11)]
		pub static PersonStatementLimit: StatementAllowanceParameter =
			StatementAllowanceParameter { max_size: 100 * 1024, max_count: 30 };
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
		/// Long-term storage granted to a full person by each claim.
		///
		/// Each claim gives 1 MiB of long-term storage to a full person.
		/// The default limit is 100 claims per period.
		/// The claims add together. The maximum is 100 MiB per person per period.
		/// This default supports the reviewed usage model. Governance can increase this value.
		#[codec(index = 5)]
		pub static LongTermStorageAllowanceForPeople: LongTermStorageAllocation =
			LongTermStorageAllocation { transactions: 100, bytes: 1024 * 1024 };
		/// Long-term storage granted to a lite person by each claim.
		///
		/// Each claim gives 100 KiB of long-term storage to a lite person.
		/// The default limit is 100 claims per period.
		/// The claims add together. The maximum is 10 MiB per person per period.
		/// This default supports the reviewed usage model. Governance can increase this value.
		#[codec(index = 6)]
		pub static LongTermStorageAllowanceForLitePeople: LongTermStorageAllocation =
			LongTermStorageAllocation { transactions: 10, bytes: 100 * 1024 };
		#[codec(index = 7)]
		pub static BulletinTransactionStoragePalletIndex: u8 = 40;
	}

	/// Allowances bounding on aliases and identity.
	#[dynamic_pallet_params]
	#[codec(index = 2)]
	pub mod origin_restriction {
		#[codec(index = 0)]
		pub static PeopleIdentityAndAliasAllowanceMax: Balance = UNITS;
		#[codec(index = 1)]
		pub static PeopleIdentityAndAliasAllowanceRecovery: Balance = 3 * CENTS;
		#[codec(index = 2)]
		pub static LitePeopleAllowanceMax: Balance = UNITS;
		#[codec(index = 3)]
		pub static LitePeopleAllowanceRecovery: Balance = 3 * MILLICENTS;
	}

	/// Lite-person registration pricing.
	#[dynamic_pallet_params]
	#[codec(index = 3)]
	pub mod lite_personhood {
		#[codec(index = 0)]
		pub static RegistrationFee: Balance = 75 * UNITS;
	}

	/// Coinage deposits, both in DOT.
	#[dynamic_pallet_params]
	#[codec(index = 4)]
	pub mod coinage {
		#[codec(index = 0)]
		pub static LoadDepositPrice: Balance = UNITS / 10;
		#[codec(index = 1)]
		pub static InstanceCreationDeposit: Balance = 10 * UNITS;
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

/// Statement-store slots per period, kept non-zero.
pub type StmtStoreSlotsPerPeriod =
	AtLeastOne<dynamic_params::statement_storage::StmtStoreSlotsPerPeriod>;

/// Lite statement-store slots per period, kept non-zero and within the full-person limit.
pub type LiteStmtStoreSlotsPerPeriod = AtMost<
	AtLeastOne<dynamic_params::statement_storage::LiteStmtStoreSlotsPerPeriod>,
	StmtStoreSlotsPerPeriod,
>;

/// Statement-store cleanup batch size, kept non-zero and within the benchmarked cap.
pub type StmtStoreCleanupLimit = BenchmarkMax<
	AtMost<
		AtLeastOne<dynamic_params::statement_storage::StmtStoreCleanupLimit>,
		ConstU32<STMT_STORE_CLEANUP_LIMIT_CAP>,
	>,
	ConstU32<STMT_STORE_CLEANUP_LIMIT_CAP>,
>;

/// Statement replacement cooldown, kept non-zero and at most one day.
pub type StmtStoreReplacementCooldown = AtMost<
	AtLeastOne<dynamic_params::statement_storage::StmtStoreReplacementCooldown>,
	ConstU32<SECONDS_PER_DAY>,
>;

/// Statement-store grace window, kept non-zero.
pub type StmtStoreGraceWindow = AtLeastOne<dynamic_params::statement_storage::StmtStoreGraceWindow>;

/// Highest valid notification slot identifier per period. Zero is valid.
pub type NotificationSlotsPerPeriod = dynamic_params::statement_storage::NotificationSlotsPerPeriod;

/// Highest valid lite notification slot identifier, kept within the full-person limit.
pub type LiteNotificationSlotsPerPeriod = AtMost<
	dynamic_params::statement_storage::LiteNotificationSlotsPerPeriod,
	NotificationSlotsPerPeriod,
>;

pub type NotificationPeriodDuration = dynamic_params::statement_storage::NotificationPeriodDuration;

/// Long-term storage period duration, kept non-zero.
pub type LongTermStoragePeriodDuration =
	AtLeastOne<dynamic_params::bulletin_storage::LongTermStoragePeriodDuration>;

/// Long-term storage grace window, kept smaller than the storage period.
pub type LongTermStorageGraceWindow = AtMost<
	dynamic_params::bulletin_storage::LongTermStorageGraceWindow,
	SaturatingSubOne<LongTermStoragePeriodDuration>,
>;

/// Long-term storage claims per period, kept non-zero.
pub type LongTermStorageClaimsPerPeriod =
	AtLeastOne<dynamic_params::bulletin_storage::LongTermStorageClaimsPerPeriod>;

/// Long-term storage cleanup batch size, kept non-zero and within the benchmarked cap.
pub type LongTermStorageCleanupLimit = BenchmarkMax<
	AtMost<
		AtLeastOne<dynamic_params::bulletin_storage::LongTermStorageCleanupLimit>,
		ConstU32<LONG_TERM_STORAGE_CLEANUP_LIMIT_CAP>,
	>,
	ConstU32<LONG_TERM_STORAGE_CLEANUP_LIMIT_CAP>,
>;

pub type LongTermStorageAllowanceForPeople =
	dynamic_params::bulletin_storage::LongTermStorageAllowanceForPeople;
pub type LongTermStorageAllowanceForLitePeople =
	dynamic_params::bulletin_storage::LongTermStorageAllowanceForLitePeople;

pub type PeopleIdentityAndAliasAllowanceMax =
	dynamic_params::origin_restriction::PeopleIdentityAndAliasAllowanceMax;
pub type LitePeopleAllowanceMax = dynamic_params::origin_restriction::LitePeopleAllowanceMax;

/// Recovery rates, kept non-zero.
pub type PeopleIdentityAndAliasAllowanceRecovery = AtLeast<
	dynamic_params::origin_restriction::PeopleIdentityAndAliasAllowanceRecovery,
	frame_support::traits::ConstU128<1>,
>;
pub type LitePeopleAllowanceRecovery = AtLeast<
	dynamic_params::origin_restriction::LitePeopleAllowanceRecovery,
	frame_support::traits::ConstU128<1>,
>;

pub type LitePersonRegistrationFee =
	AtLeast<dynamic_params::lite_personhood::RegistrationFee, ExistentialDeposit>;

/// Coinage load deposit price, kept non-zero so sponsored loads always take some collateral (the
/// pallet's integrity test requires it).
pub type CoinageLoadDepositPrice =
	AtLeast<dynamic_params::coinage::LoadDepositPrice, frame_support::traits::ConstU128<1>>;
pub type CoinageInstanceCreationDeposit = dynamic_params::coinage::InstanceCreationDeposit;

/// Root, the Fellowship governance voice, and Asset Hub's TechnicalMaintenance voice may update
/// these parameters.
pub struct DynamicParameterOrigin;
impl EnsureOriginWithArg<RuntimeOrigin, RuntimeParametersKey> for DynamicParameterOrigin {
	type Success = ();

	fn try_origin(
		origin: RuntimeOrigin,
		_key: &RuntimeParametersKey,
	) -> Result<Self::Success, RuntimeOrigin> {
		EitherOfDiverse::<
			RootOrFellows,
			EnsureXcm<IsVoiceOfBody<AssetHubLocation, TechnicalMaintenanceBodyId>>,
		>::ensure_origin(origin.clone())
		.map(|_| ())
		.map_err(|_| origin)
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
