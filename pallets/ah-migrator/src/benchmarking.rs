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

//! To run these benchmarks, you will need a modified version of `frame-omni-bencher` that can load
//! snapshots of the relay and asset hub. You can find it on branch `oty-ahm-omni-bencher` of the
//! SDK. Install it with
//! `cargo install --path substrate/utils/frame/omni-bencher --profile production`
//!
//! ```bash
//! frame-omni-bencher v1 benchmark pallet --runtime=target/release/wbuild/asset-hub-polkadot-runtime/asset_hub_polkadot_runtime.wasm --pallet "pallet-ah-migrator" --extrinsic "" --snap=ah-polkadot.snap --rc-snap=polkadot.snap
//! ```

use crate::*;
use frame_benchmarking::v2::*;
use frame_support::traits::{
	schedule::DispatchTime, tokens::IdAmount, Consideration, Currency, Footprint, Polling,
	VoteTally,
};
use frame_system::RawOrigin;
use pallet_asset_rate::AssetKindFactory;
use pallet_bounties::BountyStatus;
use pallet_conviction_voting::{AccountVote, Casting, Delegations, Vote, Voting};
use pallet_nomination_pools::TotalUnbondingPools;
use pallet_proxy::ProxyDefinition;
use pallet_rc_migrator::{
	bounties::{alias::Bounty, RcBountiesMessage},
	claims::{alias::EthereumAddress, RcClaimsMessage},
	conviction_voting::RcConvictionVotingMessage,
	crowdloan::RcCrowdloanMessage,
	indices::RcIndicesIndex,
	preimage::alias::RequestStatus as PreimageRequestStatus,
	proxy::{RcProxy, RcProxyAnnouncement},
	scheduler::{alias::Scheduled, RcSchedulerMessage},
	staking::{
		bags_list::alias::Node,
		nom_pools_alias::{SubPools, UnbondPool},
	},
	treasury::{alias::SpendStatus, RcTreasuryMessage},
};
use pallet_referenda::{Deposit, ReferendumInfo, ReferendumStatus, TallyOf, TracksInfo};
use pallet_treasury::PaymentState;

/// The minimum amount used for deposits, transfers, etc.
///
/// Equivalent to Polkadot `UNITS`, which is larger than Kusama `UNITS`.
pub const UNITS: u128 = 10_000_000_000;

type CurrencyOf<T> = pallet_balances::Pallet<T>;

pub trait ParametersFactory<
	RcMultisig,
	RcAccount,
	RcClaimsMessage,
	RcProxy,
	RcProxyAnnouncement,
	RcVestingSchedule,
	RcNomPoolsMessage,
	RcFastUnstakeMessage,
	RcReferendumInfo,
	RcSchedulerMessage,
	RcBagsListMessage,
	RcIndicesIndex,
	RcConvictionVotingMessage,
	RcBountiesMessage,
	RcAssetKindMessage,
	RcCrowdloanMessage,
	RcTreasuryMessage,
	RcPreimageLegacyStatus,
	RcPreimageRequestStatus,
>
{
	fn create_multisig(n: u8) -> RcMultisig;
	fn create_account(n: u8) -> RcAccount;
	fn create_liquid_account(n: u8) -> RcAccount;
	fn create_vesting_msg(n: u8) -> RcClaimsMessage;
	fn create_proxy(n: u8) -> RcProxy;
	fn create_proxy_announcement(n: u8) -> RcProxyAnnouncement;
	fn create_vesting_schedule(n: u8) -> RcVestingSchedule;
	fn create_nom_sub_pool(n: u8) -> RcNomPoolsMessage;
	fn create_fast_unstake(n: u8) -> RcFastUnstakeMessage;
	fn create_referendum_info(n: u8) -> (u32, RcReferendumInfo);
	fn create_scheduler_agenda(n: u8) -> RcSchedulerMessage;
	fn create_scheduler_lookup(n: u8) -> RcSchedulerMessage;
	fn create_bags_list(n: u8) -> RcBagsListMessage;
	fn create_indices_index(n: u8) -> RcIndicesIndex;
	fn create_conviction_vote(n: u8) -> RcConvictionVotingMessage;
	fn create_bounties(n: u8) -> RcBountiesMessage;
	fn create_asset_rate(n: u8) -> RcAssetKindMessage;
	fn create_crowdloan(n: u8) -> RcCrowdloanMessage;
	fn create_treasury(n: u8) -> RcTreasuryMessage;
	fn create_preimage_legacy_status(n: u8) -> RcPreimageLegacyStatus;
	fn create_preimage_request_status(n: u8) -> RcPreimageRequestStatus;
}

pub struct BenchmarkFactory<T: Config>(PhantomData<T>);
impl<T: Config>
	ParametersFactory<
		RcMultisig<AccountId32, u128>,
		RcAccount<AccountId32, u128, T::RcHoldReason, T::RcFreezeReason>,
		RcClaimsMessage<AccountId32, u128, u32>,
		RcProxy<AccountId32, u128, T::RcProxyType, u32>,
		RcProxyAnnouncement<AccountId32, u128>,
		RcVestingSchedule<T>,
		RcNomPoolsMessage<T>,
		RcFastUnstakeMessage<T>,
		RcReferendumInfoOf<T, ()>,
		RcSchedulerMessageOf<T>,
		RcBagsListMessage<T>,
		RcIndicesIndexOf<T>,
		RcConvictionVotingMessageOf<T>,
		RcBountiesMessageOf<T>,
		(<T as pallet_asset_rate::Config>::AssetKind, FixedU128),
		RcCrowdloanMessageOf<T>,
		RcTreasuryMessageOf<T>,
		RcPreimageLegacyStatusOf<T>,
		RcPreimageRequestStatusOf<T>,
	> for BenchmarkFactory<T>
where
	<<T as pallet_conviction_voting::Config>::Polls as Polling<
		pallet_conviction_voting::TallyOf<T, ()>,
	>>::Index: From<u8>,
	<<T as pallet_preimage::Config>::Currency as Currency<sp_runtime::AccountId32>>::Balance:
		From<u128>,
{
	fn create_multisig(n: u8) -> RcMultisig<AccountId32, u128> {
		let creator: AccountId32 = [n; 32].into();
		let deposit: u128 = UNITS;
		let _ = CurrencyOf::<T>::deposit_creating(&creator, (deposit * 10).into());
		let _ = CurrencyOf::<T>::reserve(&creator, deposit.into()).unwrap();

		RcMultisig { creator, deposit, details: Some([2u8; 32].into()) }
	}

	fn create_account(n: u8) -> RcAccount<AccountId32, u128, T::RcHoldReason, T::RcFreezeReason> {
		let who: AccountId32 = [n; 32].into();
		let _ = CurrencyOf::<T>::deposit_creating(
			&who,
			<CurrencyOf<T> as Currency<_>>::minimum_balance(),
		);

		let hold_amount = UNITS;
		let holds = vec![IdAmount { id: T::RcHoldReason::default(), amount: hold_amount }];

		let freeze_amount = 2 * UNITS;
		let freezes = vec![IdAmount { id: T::RcFreezeReason::default(), amount: freeze_amount }];

		let lock_amount = 3 * UNITS;
		let locks = vec![pallet_balances::BalanceLock::<u128> {
			id: [1u8; 8],
			amount: lock_amount,
			reasons: pallet_balances::Reasons::All,
		}];

		let unnamed_reserve = 4 * UNITS;

		let free = UNITS + hold_amount + freeze_amount + lock_amount + unnamed_reserve;
		let reserved = hold_amount + unnamed_reserve;
		let frozen = freeze_amount + lock_amount;

		RcAccount {
			who,
			free,
			reserved,
			frozen,
			holds: holds.try_into().unwrap(),
			freezes: freezes.try_into().unwrap(),
			locks: locks.try_into().unwrap(),
			unnamed_reserve,
			consumers: 1,
			providers: 1,
		}
	}

	fn create_liquid_account(
		n: u8,
	) -> RcAccount<AccountId32, u128, T::RcHoldReason, T::RcFreezeReason> {
		let who: AccountId32 = [n; 32].into();
		let _ = CurrencyOf::<T>::deposit_creating(
			&who,
			<CurrencyOf<T> as Currency<_>>::minimum_balance(),
		);

		RcAccount {
			who,
			free: UNITS,
			reserved: 0,
			frozen: 0,
			holds: Default::default(),
			freezes: Default::default(),
			locks: Default::default(),
			unnamed_reserve: 0,
			consumers: 1,
			providers: 1,
		}
	}

	fn create_vesting_msg(n: u8) -> RcClaimsMessage<AccountId32, u128, u32> {
		RcClaimsMessage::Vesting { who: EthereumAddress([n; 20]), schedule: (100, 200, 300) }
	}

	fn create_proxy(n: u8) -> RcProxy<AccountId32, u128, T::RcProxyType, u32> {
		let proxy_def = ProxyDefinition {
			proxy_type: T::RcProxyType::default(),
			delegate: [n; 32].into(),
			delay: 100,
		};
		let proxies = vec![proxy_def; T::MaxProxies::get() as usize];

		RcProxy { delegator: [n; 32].into(), deposit: 200, proxies }
	}

	fn create_proxy_announcement(n: u8) -> RcProxyAnnouncement<AccountId32, u128> {
		let creator: AccountId32 = [n; 32].into();
		let deposit: u128 = UNITS;
		let _ = CurrencyOf::<T>::deposit_creating(&creator, (deposit * 10).into());
		let _ = CurrencyOf::<T>::reserve(&creator, deposit.into()).unwrap();

		RcProxyAnnouncement { depositor: creator, deposit }
	}

	fn create_vesting_schedule(n: u8) -> RcVestingSchedule<T> {
		let max_schedule = pallet_vesting::MaxVestingSchedulesGet::<T>::get();
		let schedule = pallet_vesting::VestingInfo::new(n.into(), n.into(), n.into());
		RcVestingSchedule {
			who: [n; 32].into(),
			schedules: vec![schedule; max_schedule as usize].try_into().unwrap(),
		}
	}

	fn create_nom_sub_pool(n: u8) -> RcNomPoolsMessage<T> {
		let mut with_era = BoundedBTreeMap::<_, _, _>::new();
		for i in 0..TotalUnbondingPools::<T>::get() {
			let key = i.into();
			with_era
				.try_insert(key, UnbondPool { points: n.into(), balance: n.into() })
				.unwrap();
		}

		RcNomPoolsMessage::SubPoolsStorage {
			sub_pools: (
				n.into(),
				SubPools { no_era: UnbondPool { points: n.into(), balance: n.into() }, with_era },
			),
		}
	}

	fn create_fast_unstake(n: u8) -> RcFastUnstakeMessage<T> {
		RcFastUnstakeMessage::Queue { member: ([n; 32].into(), n.into()) }
	}

	fn create_referendum_info(n: u8) -> (u32, RcReferendumInfoOf<T, ()>) {
		let id = n.into();
		let tracks = <T as pallet_referenda::Config>::Tracks::tracks();
		let track_id = tracks.iter().next().unwrap().0;
		let deposit = Deposit { who: [n; 32].into(), amount: n.into() };
		let call: <T as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark { remark: vec![n; 2048] }.into();
		(
			id,
			ReferendumInfo::Ongoing(ReferendumStatus {
				track: track_id,
				origin: Default::default(),
				proposal: <T as pallet_referenda::Config>::Preimages::bound(call).unwrap(),
				enactment: DispatchTime::At(n.into()),
				submitted: n.into(),
				submission_deposit: deposit.clone(),
				decision_deposit: Some(deposit),
				deciding: None,
				tally: TallyOf::<T, ()>::new(track_id),
				in_queue: false,
				alarm: None,
			}),
		)
	}

	fn create_scheduler_agenda(n: u8) -> RcSchedulerMessageOf<T> {
		let call: <T as frame_system::Config>::RuntimeCall =
			frame_system::Call::remark { remark: vec![n; 2048] }.into();
		let scheduled = Scheduled {
			maybe_id: Some([n; 32]),
			priority: n,
			call: <T as pallet_referenda::Config>::Preimages::bound(call).unwrap(),
			maybe_periodic: None,
			origin: Default::default(),
		};
		// one task but big, 2048 byte call.
		RcSchedulerMessage::Agenda((n.into(), vec![Some(scheduled)]))
	}

	fn create_scheduler_lookup(n: u8) -> RcSchedulerMessageOf<T> {
		RcSchedulerMessage::Lookup(([n; 32], (n.into(), n.into())))
	}

	fn create_bags_list(n: u8) -> RcBagsListMessage<T> {
		RcBagsListMessage::Node {
			id: [n; 32].into(),
			node: Node {
				id: [n; 32].into(),
				prev: Some([n; 32].into()),
				next: Some([n; 32].into()),
				bag_upper: n.into(),
				score: n.into(),
			},
		}
	}

	fn create_indices_index(n: u8) -> RcIndicesIndexOf<T> {
		return RcIndicesIndex {
			index: n.into(),
			who: [n; 32].into(),
			deposit: n.into(),
			frozen: false,
		}
	}

	fn create_conviction_vote(n: u8) -> RcConvictionVotingMessageOf<T> {
		let class = <T as pallet_conviction_voting::Config>::Polls::classes()
			.iter()
			.skip(n as usize)
			.next()
			.unwrap()
			.clone();
		let votes = BoundedVec::<(_, AccountVote<_>), _>::try_from(
			(0..<T as pallet_conviction_voting::Config<()>>::MaxVotes::get())
				.map(|_| {
					(
						n.into(),
						AccountVote::Standard {
							vote: Vote { aye: true, conviction: Default::default() },
							balance: n.into(),
						},
					)
				})
				.collect::<Vec<_>>(),
		)
		.unwrap();
		RcConvictionVotingMessage::VotingFor(
			[n; 32].into(),
			class,
			Voting::Casting(Casting {
				votes,
				delegations: Delegations { votes: n.into(), capital: n.into() },
				prior: Default::default(),
			}),
		)
	}

	fn create_bounties(n: u8) -> RcBountiesMessageOf<T> {
		RcBountiesMessage::Bounties((
			n.into(),
			Bounty {
				proposer: [n; 32].into(),
				value: n.into(),
				fee: n.into(),
				curator_deposit: n.into(),
				bond: n.into(),
				status: BountyStatus::Active { curator: [n; 32].into(), update_due: n.into() },
			},
		))
	}

	fn create_asset_rate(n: u8) -> (<T as pallet_asset_rate::Config>::AssetKind, FixedU128) {
		(
			<T as pallet_asset_rate::Config>::BenchmarkHelper::create_asset_kind(n.into()),
			FixedU128::from_u32(n as u32),
		)
	}

	fn create_crowdloan(n: u8) -> RcCrowdloanMessageOf<T> {
		RcCrowdloanMessage::CrowdloanContribution {
			withdraw_block: n.into(),
			contributor: [n.into(); 32].into(),
			para_id: (n as u32).into(),
			amount: n.into(),
			crowdloan_account: [n.into(); 32].into(),
		}
	}

	fn create_treasury(n: u8) -> RcTreasuryMessageOf<T> {
		RcTreasuryMessage::Spends {
			id: n.into(),
			status: SpendStatus {
				asset_kind: VersionedLocatableAsset::V4 {
					location: Location::new(0, [Parachain(1000)]),
					asset_id: Location::new(0, [PalletInstance(n.into()), GeneralIndex(n.into())])
						.into(),
				},
				amount: n.into(),
				beneficiary: VersionedLocation::V4(Location::new(
					0,
					[xcm::latest::Junction::AccountId32 { network: None, id: [n; 32].into() }],
				)),
				valid_from: n.into(),
				expire_at: n.into(),
				status: PaymentState::Pending,
			},
		}
	}

	fn create_preimage_legacy_status(n: u8) -> RcPreimageLegacyStatusOf<T> {
		let depositor: AccountId32 = [n; 32].into();
		let deposit = <CurrencyOf<T> as Currency<_>>::minimum_balance();
		let _ = CurrencyOf::<T>::deposit_creating(&depositor, (deposit * 10).into());
		let _ = CurrencyOf::<T>::reserve(&depositor, deposit.into()).unwrap();

		RcPreimageLegacyStatusOf::<T> { hash: [n; 32].into(), depositor, deposit: deposit.into() }
	}

	fn create_preimage_request_status(n: u8) -> RcPreimageRequestStatusOf<T> {
		let preimage = vec![n; 512];
		let hash = T::Preimage::note(preimage.into()).unwrap();

		let depositor: AccountId32 = [n; 32].into();
		let old_footprint = Footprint::from_parts(1, 1024);
		<T as pallet_preimage::Config>::Consideration::ensure_successful(&depositor, old_footprint);
		let consideration =
			<T as pallet_preimage::Config>::Consideration::new(&depositor, old_footprint).unwrap();
		RcPreimageRequestStatusOf::<T> {
			hash,
			request_status: PreimageRequestStatus::Unrequested {
				ticket: (depositor, consideration),
				len: 512, // smaller than old footprint
			},
		}
	}
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks]
pub mod benchmarks {
	use super::*;

	#[benchmark]
	fn on_finalize() {
		let block_num = BlockNumberFor::<T>::from(1u32);
		DmpDataMessageCounts::<T>::put((1, 0));

		#[block]
		{
			Pallet::<T>::on_finalize(block_num)
		}
	}

	// TODO: breaks CI, not needed for now
	// #[benchmark]
	// fn receive_multisigs_from_snap(n: Linear<1, 255>) {
	// 	verify_snapshot::<T>();
	// 	let (mut messages, _cursor) = relay_snapshot(|| {
	// 		unwrap_no_debug(
	// 			pallet_rc_migrator::multisig::MultisigMigrator::<T, ()>::migrate_out_many(
	// 				None,
	// 				&mut WeightMeter::new(),
	// 				&mut WeightMeter::new(),
	// 			),
	// 		)
	// 	});

	// 	// TODO: unreserve fails since accounts should migrate first to make it successful. we will
	// 	// have a similar issue with the other calls benchmarks.
	// 	// TODO: possible we can truncate to n to have weights based on the number of messages
	// 	// TODO: for calls that have messages with `m` number of variants, we perhaps need to have
	// 	// `m` parameters like `n` parameter in this function. and we filter the returned by
	// 	// `migrate_out_many` `messages` or we pass these parameters to `migrate_out_many`.
	// 	messages.truncate(n as usize);

	// 	#[extrinsic_call]
	// 	receive_multisigs(RawOrigin::Root, messages);

	// 	for event in frame_system::Pallet::<T>::events() {
	// 		let encoded = event.encode();
	// 		log::info!("Event of pallet: {} and event: {}", encoded[0], encoded[1]);
	// 	}
	// }

	#[benchmark]
	fn receive_multisigs(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_multisig(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Multisig,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_accounts(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_account(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Balances,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_liquid_accounts(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_liquid_account(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		receive_accounts(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Balances,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_claims(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_vesting_msg(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed { pallet: PalletEventName::Claims, count_good: n, count_bad: 0 }
				.into(),
		);
	}

	#[benchmark]
	fn receive_proxy_proxies(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_proxy(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::ProxyProxies,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_proxy_announcements(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_proxy_announcement(i.try_into().unwrap())
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::ProxyAnnouncements,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_vesting_schedules(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_vesting_schedule(i.try_into().unwrap())
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed { pallet: PalletEventName::Vesting, count_good: n, count_bad: 0 }
				.into(),
		);
	}

	#[benchmark]
	fn receive_nom_pools_messages(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_nom_sub_pool(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::NomPools,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_fast_unstake_messages(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_fast_unstake(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::FastUnstake,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_referenda_values() {
		let referendum_count = 50;
		let mut deciding_count = vec![];
		let mut track_queue = vec![];

		let tracks = <T as pallet_referenda::Config>::Tracks::tracks();
		for (i, (id, _)) in tracks.iter().enumerate() {
			deciding_count.push((id.clone(), (i as u32).into()));

			track_queue.push((
				id.clone(),
				vec![
					(i as u32, (i as u32).into());
					<T as pallet_referenda::Config>::MaxQueued::get() as usize
				],
			));
		}

		#[extrinsic_call]
		_(RawOrigin::Root, referendum_count, deciding_count, track_queue);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::ReferendaValues,
				count_good: 1,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_active_referendums(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_referendum_info(i.try_into().unwrap())
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		receive_referendums(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::ReferendaReferendums,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_complete_referendums(n: Linear<1, 255>) {
		let mut referendums: Vec<(u32, RcReferendumInfoOf<T, ()>)> = vec![];
		for i in 0..n {
			let i_as_byte: u8 = i.try_into().unwrap();
			let deposit = Deposit { who: [i_as_byte; 32].into(), amount: n.into() };
			referendums.push((
				i,
				ReferendumInfo::Approved(i.into(), Some(deposit.clone()), Some(deposit)),
			));
		}

		#[extrinsic_call]
		receive_referendums(RawOrigin::Root, referendums);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::ReferendaReferendums,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_scheduler_agenda(n: Linear<1, 255>) {
		let agendas = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_scheduler_agenda(i.try_into().unwrap())
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		receive_scheduler_messages(RawOrigin::Root, agendas);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Scheduler,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_scheduler_lookup(n: Linear<1, 255>) {
		let lookups = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_scheduler_lookup(i.try_into().unwrap())
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		receive_scheduler_messages(RawOrigin::Root, lookups);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Scheduler,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_bags_list_messages(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_bags_list(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::BagsList,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_indices(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_indices_index(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed { pallet: PalletEventName::Indices, count_good: n, count_bad: 0 }
				.into(),
		);
	}

	#[benchmark]
	fn receive_conviction_voting_messages(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_conviction_vote(i.try_into().unwrap())
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::ConvictionVoting,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_bounties_messages(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_bounties(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Bounties,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_asset_rates(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_asset_rate(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::AssetRates,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_crowdloan_messages(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_crowdloan(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Crowdloan,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_referenda_metadata(n: Linear<1, 255>) {
		let messages = (0..n).map(|i| (i.into(), H256::from([i as u8; 32]))).collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::ReferendaMetadata,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_treasury_messages(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| <<T as Config>::BenchmarkHelper>::create_treasury(i.try_into().unwrap()))
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::Treasury,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_preimage_legacy_status(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_preimage_legacy_status(
					i.try_into().unwrap(),
				)
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::PreimageLegacyStatus,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn receive_preimage_request_status(n: Linear<1, 255>) {
		let messages = (0..n)
			.map(|i| {
				<<T as Config>::BenchmarkHelper>::create_preimage_request_status(
					i.try_into().unwrap(),
				)
			})
			.collect::<Vec<_>>();

		#[extrinsic_call]
		_(RawOrigin::Root, messages);

		assert_last_event::<T>(
			Event::BatchProcessed {
				pallet: PalletEventName::PreimageRequestStatus,
				count_good: n,
				count_bad: 0,
			}
			.into(),
		);
	}

	#[benchmark]
	fn force_set_stage() {
		let stage = MigrationStage::DataMigrationOngoing;

		#[extrinsic_call]
		_(RawOrigin::Root, stage.clone());

		assert_last_event::<T>(
			Event::StageTransition { old: MigrationStage::Pending, new: stage }.into(),
		);
	}

	#[benchmark]
	fn start_migration() {
		#[extrinsic_call]
		_(RawOrigin::Root);

		assert_last_event::<T>(
			Event::StageTransition {
				old: MigrationStage::Pending,
				new: MigrationStage::DataMigrationOngoing,
			}
			.into(),
		);
	}

	#[benchmark]
	fn finish_migration() {
		#[extrinsic_call]
		_(RawOrigin::Root, MigrationFinishedData { rc_balance_kept: 100 });

		assert_last_event::<T>(
			Event::StageTransition {
				old: MigrationStage::Pending,
				new: MigrationStage::MigrationDone,
			}
			.into(),
		);
	}

	#[cfg(feature = "std")]
	pub fn test_receive_multisigs<T: Config>(n: u32) {
		_receive_multisigs::<T>(n, true /* enable checks */)
	}

	#[cfg(feature = "std")]
	pub fn test_on_finalize<T: Config>() {
		_on_finalize::<T>(true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_proxy_proxies<T: Config>(n: u32) {
		_receive_proxy_proxies::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_proxy_announcements<T: Config>(n: u32) {
		_receive_proxy_announcements::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_claims<T: Config>(n: u32) {
		_receive_claims::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_nom_pools_messages<T: Config>(n: u32) {
		_receive_nom_pools_messages::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_vesting_schedules<T: Config>(n: u32) {
		_receive_vesting_schedules::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_fast_unstake_messages<T: Config>(n: u32) {
		_receive_fast_unstake_messages::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_referenda_values<T: Config>() {
		_receive_referenda_values::<T>(true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_active_referendums<T: Config>(n: u32) {
		_receive_active_referendums::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_complete_referendums<T: Config>(n: u32) {
		_receive_complete_referendums::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_accounts<T: Config>(n: u32) {
		_receive_accounts::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_liquid_accounts<T: Config>(n: u32) {
		_receive_liquid_accounts::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_scheduler_agenda<T: Config>(n: u32) {
		_receive_scheduler_agenda::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_scheduler_lookup<T: Config>(n: u32) {
		_receive_scheduler_lookup::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_bags_list_messages<T: Config>(n: u32) {
		_receive_bags_list_messages::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_indices<T: Config>(n: u32) {
		_receive_indices::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_conviction_voting_messages<T: Config>(n: u32) {
		_receive_conviction_voting_messages::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_bounties_messages<T: Config>(n: u32) {
		_receive_bounties_messages::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_asset_rates<T: Config>(n: u32) {
		_receive_asset_rates::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_crowdloan_messages<T: Config>(n: u32) {
		_receive_crowdloan_messages::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_referenda_metadata<T: Config>(n: u32) {
		_receive_referenda_metadata::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_treasury_messages<T: Config>(n: u32) {
		_receive_treasury_messages::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_force_set_stage<T: Config>() {
		_force_set_stage::<T>(true)
	}

	#[cfg(feature = "std")]
	pub fn test_start_migration<T: Config>() {
		_start_migration::<T>(true)
	}

	#[cfg(feature = "std")]
	pub fn test_finish_migration<T: Config>() {
		_finish_migration::<T>(true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_preimage_legacy_status<T: Config>(n: u32) {
		_receive_preimage_legacy_status::<T>(n, true)
	}

	#[cfg(feature = "std")]
	pub fn test_receive_preimage_request_status<T: Config>(n: u32) {
		_receive_preimage_request_status::<T>(n, true)
	}
}

/// Unwrap something that does not implement Debug. Otherwise we would need to require
/// `pallet_rc_migrator::Config` on out runtime `T`.
pub fn unwrap_no_debug<T, E>(result: Result<T, E>) -> T {
	match result {
		Ok(t) => t,
		Err(_) => panic!("unwrap_no_debug"),
	}
}

/// Check that Oliver's account has some balance on AH and Relay.
///
/// This serves as sanity check that the snapshots were loaded correctly.
fn verify_snapshot<T: Config>() {
	let raw_acc: [u8; 32] =
		hex::decode("6c9e3102dd2c24274667d416e07570ebce6f20ab80ee3fc9917bf4a7568b8fd2")
			.unwrap()
			.try_into()
			.unwrap();
	let acc = AccountId32::from(raw_acc);
	frame_system::Pallet::<T>::reset_events();

	// Sanity check that this is the right account
	let ah_acc = frame_system::Account::<T>::get(&acc);
	if ah_acc.data.free == 0 {
		panic!("No or broken snapshot: account does not have any balance");
	}

	let key = frame_system::Account::<T>::hashed_key_for(&acc);
	let raw_acc = relay_snapshot(|| {
		frame_support::storage::unhashed::get::<
			pallet_balances::AccountData<<T as pallet_balances::Config>::Balance>,
		>(key.as_ref())
	})
	.unwrap();

	if raw_acc.free == 0 {
		panic!("No or broken snapshot: account does not have any balance");
	}
}

/// Read something from the relay chain snapshot instead of the asset hub one.
fn relay_snapshot<R, F: FnOnce() -> R>(f: F) -> R {
	sp_io::storage::get(b"relay_chain_enable");
	let result = f();
	sp_io::storage::get(b"relay_chain_disable");
	result
}
