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

#![allow(deprecated, missing_docs)]

use super::*;

/// Unreleased migrations. Add new ones here:
pub type Unreleased = (
	// xcmp-queue storage v6 -> v7 (SDK stable2606-1).
	cumulus_pallet_xcmp_queue::migration::v7::MigrateV6ToV7<Runtime>,
	cumulus_pallet_parachain_system::migration::Migration<Runtime>,
	// Seed storage authorizations for the rank-1+ Fellowship.
	//
	// TEMPORARY. The grants lapse on their own after `AuthorizationPeriod` (14 days) and are
	// not renewed.
	//
	// Once Individuality lands (https://github.com/polkadot-fellows/runtimes/pull/1233), Proof of
	// Personhood on the People Chain is the only authorizer, so there is no way to store data here
	// until that logic is switched on live. This grant opens a short window to sanity-check the
	// chain itself first — Bitswap retrieval, collator p2p setup, and the like — before PoP/DIM
	// go live.
	//
	// NOTE: this is entirely optional. If we decide we do not want this testing window, drop this
	// one line before release — nothing else in the runtime depends on it, and the rest of
	// `authorize_fellowship` (snapshot, tests) goes with it.
	authorize_fellowship::AuthorizeFellowshipMembers,
);

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = (
	pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>,
	// Idempotent: initializes `RetentionPeriod` when zero, a no-op once set.
	pallet_bulletin_transaction_storage::migrations::SetRetentionPeriodIfZero<
		Runtime,
		pallet_bulletin_transaction_storage::DefaultRetentionPeriod,
	>,
);

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = (Unreleased, Permanent);

/// MBM migrations to apply on runtime upgrade.
pub type MbmMigrations = ();

/// Grant storage authorizations to the rank-1+ Polkadot Technical Fellowship.
///
/// Self-contained: the migration, the hard-coded member snapshot it grants to, its unit tests,
/// and the live-state check that keeps the snapshot honest all live here.
pub mod authorize_fellowship {
	use super::*;
	use frame_support::traits::OnRuntimeUpgrade;
	use frame_system::RawOrigin;
	use hex_literal::hex;
	use pallet_bulletin_transaction_storage::WeightInfo as _;
	use sp_runtime::traits::Saturating;

	/// Number of `store`/`renew` transactions granted to each Fellowship member.
	pub const FELLOWSHIP_TRANSACTIONS: u32 = 100;
	/// Total byte allowance granted to each Fellowship member: 200 MiB.
	pub const FELLOWSHIP_BYTES: u64 = 200 * 1024 * 1024;

	/// Snapshot of the Fellowship members holding rank 1 or higher, ordered by descending rank
	/// then address.
	///
	/// # Provenance
	///
	/// Taken from `FellowshipCollective::Members` on Collectives Polkadot at block `9,496,200`
	/// (hash `0x23aee9a890b2800f95e74871617401805604ebcc5bb15aefd9552a935eed7c99`), keeping
	/// every member whose `MemberRecord::rank` is `>= 1` — i.e. all inducted Fellows, excluding
	/// rank-0 candidates. Each entry carries a Subsquare link so the rank can be verified by
	/// hand.
	///
	/// This is a *snapshot*: the Fellowship changes over time, and the Bulletin chain cannot
	/// read Collectives state, so the list has to be hard-coded. Refresh it (and the block
	/// reference above) whenever the migration is re-armed.
	pub const FELLOWSHIP_RANK1_PLUS_MEMBERS: [[u8; 32]; 58] = [
		// Rank 7 - 16SDAKg9N6kKAbhgDyxBXdHEwpwHUHs2CNEiLNGeZV55qHna
		// https://collectives.subsquare.io/user/16SDAKg9N6kKAbhgDyxBXdHEwpwHUHs2CNEiLNGeZV55qHna/fellowship
		hex!("f0673d30606ee26672707e4fd2bc8b58d3becb7aba2d5f60add64abb5fea4710"),
		// Rank 6 - 13fvj4bNfrTo8oW6U8525soRp6vhjAFLum6XBdtqq9yP22E7
		// https://collectives.subsquare.io/user/13fvj4bNfrTo8oW6U8525soRp6vhjAFLum6XBdtqq9yP22E7/fellowship
		hex!("7628a5be63c4d3c8dbb96c2904b1a9682e02831a1af836c7efc808020b92fa63"),
		// Rank 5 - 12MrP337azmkTdfCUKe5XLnSQrbgEKqqfZ4PQC7CZTJKAWR3
		// https://collectives.subsquare.io/user/12MrP337azmkTdfCUKe5XLnSQrbgEKqqfZ4PQC7CZTJKAWR3/fellowship
		hex!("3c235e80e35082b668682531b9b062fda39a46edb94f884d9122d86885fd5f1b"),
		// Rank 5 - 15G1iXDLgFyfnJ51FKq1ts44TduMyUtekvzQi9my4hgYt2hs
		// https://collectives.subsquare.io/user/15G1iXDLgFyfnJ51FKq1ts44TduMyUtekvzQi9my4hgYt2hs/fellowship
		hex!("bc64065524532ed9e805fb0d39a5c0199216b52871168e5e4d0ab612f8797d61"),
		// Rank 5 - 1eTPAR2TuqLyidmPT9rMmuycHVm9s9czu78sePqg2KHMDrE
		// https://collectives.subsquare.io/user/1eTPAR2TuqLyidmPT9rMmuycHVm9s9czu78sePqg2KHMDrE/fellowship
		hex!("1c90e3dabd3fd0f6bc648045018f78fcee8fe24122c22d8d2a14e9905073d10f"),
		// Rank 4 - 123SVCkcHnNKyng8EPmaUeay5kKHu1jig99RT21E2cEx5pQF
		// https://collectives.subsquare.io/user/123SVCkcHnNKyng8EPmaUeay5kKHu1jig99RT21E2cEx5pQF/fellowship
		hex!("2e1884c53071526483b14004e894415f02b55fc2e2aef8e1df8ccf7ce5bd5570"),
		// Rank 4 - 12hAtDZJGt4of3m2GqZcUCVAjZPALfvPwvtUTFZPQUbdX1Ud
		// https://collectives.subsquare.io/user/12hAtDZJGt4of3m2GqZcUCVAjZPALfvPwvtUTFZPQUbdX1Ud/fellowship
		hex!("4adf51a47b72795366d52285e329229c836ea7bbfe139dbe8fa0700c4f86fc56"),
		// Rank 4 - 12pRzYaysQz6Tr1e78sRmu9FGB8gu8yTek9x6xwVFFAwXTM8
		// https://collectives.subsquare.io/user/12pRzYaysQz6Tr1e78sRmu9FGB8gu8yTek9x6xwVFFAwXTM8/fellowship
		hex!("5068e605e8d0653954d518d7b9025d374b2e374731bbe800cfa6ca89743f7053"),
		// Rank 4 - 1363HWTPzDrzAQ6ChFiMU6mP4b6jmQid2ae55JQcKtZnpLGv
		// https://collectives.subsquare.io/user/1363HWTPzDrzAQ6ChFiMU6mP4b6jmQid2ae55JQcKtZnpLGv/fellowship
		hex!("5c5062779d44ea2ab0469e155b8cf3e004fce71b3b3d38263cd9fa9478f12f28"),
		// Rank 4 - 14DsLzVyTUTDMm2eP3czwPbH53KgqnQRp3CJJZS9GR7yxGDP
		// https://collectives.subsquare.io/user/14DsLzVyTUTDMm2eP3czwPbH53KgqnQRp3CJJZS9GR7yxGDP/fellowship
		hex!("8e851ed992228f2268ee8c614fe6075d3800060ae14098e0309413a0a81c4470"),
		// Rank 4 - 14YDyDZ9o1Nr2hMqLSbjYpr4Wm5s1gux6CvjYZfUTJ4Np3w1
		// https://collectives.subsquare.io/user/14YDyDZ9o1Nr2hMqLSbjYpr4Wm5s1gux6CvjYZfUTJ4Np3w1/fellowship
		hex!("9c84f75e0b1b92f6b003bde6212a8b2c9b776f3720f942b33fed8709f103a268"),
		// Rank 3 - 121dd6J26VUnBZ8BqLGjANWkEAXSb9mWq1SB7LsS9QNTGFvz
		// https://collectives.subsquare.io/user/121dd6J26VUnBZ8BqLGjANWkEAXSb9mWq1SB7LsS9QNTGFvz/fellowship
		hex!("2cb783d5c0ddcccd2608c83d43ee6fc19320408c24764c2f8ac164b27beaee37"),
		// Rank 3 - 128pmEUBSGjGeXZXNaAmomAJgVn77L74YT7Zdjd3fP63HWNP
		// https://collectives.subsquare.io/user/128pmEUBSGjGeXZXNaAmomAJgVn77L74YT7Zdjd3fP63HWNP/fellowship
		hex!("3233bc0f629a2b2436e4517d18919f01acaa4d7a958d3dbe83e238dc309eb47e"),
		// Rank 3 - 12HWjfYxi7xt7EvpTxUis7JoNWF7YCqa19JXmuiwizfwJZY2
		// https://collectives.subsquare.io/user/12HWjfYxi7xt7EvpTxUis7JoNWF7YCqa19JXmuiwizfwJZY2/fellowship
		hex!("38d442452b078203a37f178906e67650b19128ad687051f1aaca4c0c428b3c04"),
		// Rank 3 - 13QdJvnJgfoitjrxESwrCWTaLMN8KvXxufDUucXM6EWGuxqh
		// https://collectives.subsquare.io/user/13QdJvnJgfoitjrxESwrCWTaLMN8KvXxufDUucXM6EWGuxqh/fellowship
		hex!("6a7d56a801015edae2ebdeafb80bd4191554965942141024a2014ea2a11c8477"),
		// Rank 3 - 13psJuWEjBuZGaFqXvFnLMC6ME8RVVfQAhtFhydYjW45oKgZ
		// https://collectives.subsquare.io/user/13psJuWEjBuZGaFqXvFnLMC6ME8RVVfQAhtFhydYjW45oKgZ/fellowship
		hex!("7cfa5ae9b993c5c7ca42efc9834bbdfa67d216e2a5a7b5ad4986907719da0976"),
		// Rank 3 - 142zGifFwRrDbFLJD7LvbyoHQAqDaXeHjkxJbUVwmDYBD7Gf
		// https://collectives.subsquare.io/user/142zGifFwRrDbFLJD7LvbyoHQAqDaXeHjkxJbUVwmDYBD7Gf/fellowship
		hex!("8638bcb5e5ae5e04954d7fa2d29ccedf7f323482573198732af0b3fe32f8da03"),
		// Rank 3 - 148f8D1P4CP2tV8JuaVHzEXQQgj3jBxEg3k9qZydPzkJjbQG
		// https://collectives.subsquare.io/user/148f8D1P4CP2tV8JuaVHzEXQQgj3jBxEg3k9qZydPzkJjbQG/fellowship
		hex!("8a8bc181a8b2a5e27a7c66051fe9119875f4c5718cc808aaf2258b4b2fa37832"),
		// Rank 3 - 14Mjra9hNggCSLyVv3AmtvpjeyBhUMwiEEBXGFidu3ZWTEQN
		// https://collectives.subsquare.io/user/14Mjra9hNggCSLyVv3AmtvpjeyBhUMwiEEBXGFidu3ZWTEQN/fellowship
		hex!("9485dec323c8a1d5c4bf4d08eada2fe7ba6cf4e885ab8e53c0c96af4ea546312"),
		// Rank 3 - 14T9NGF7LdCY7SK2j6oNXmB9NqfKYyrxBChUvYRjtFdvZBMo
		// https://collectives.subsquare.io/user/14T9NGF7LdCY7SK2j6oNXmB9NqfKYyrxBChUvYRjtFdvZBMo/fellowship
		hex!("98a53ecfdfed160f11f89ef674b05311de07888af414d03fc14ceacf6edc1a07"),
		// Rank 3 - 15DCWHQknBjc5YPFoVj8Pn2KoqrqYywJJ95BYNYJ4Fj3NLqz
		// https://collectives.subsquare.io/user/15DCWHQknBjc5YPFoVj8Pn2KoqrqYywJJ95BYNYJ4Fj3NLqz/fellowship
		hex!("ba3e9b87792bcfcc237fa8181185b8883c77f3e24f45e4a92ab31d07a4703520"),
		// Rank 3 - 15db5ksZgmhWE9U8MDq4wLKUdFivLVBybztWV8nmaJvv3NU1
		// https://collectives.subsquare.io/user/15db5ksZgmhWE9U8MDq4wLKUdFivLVBybztWV8nmaJvv3NU1/fellowship
		hex!("ccd87fa65729f7bdaa8305581a7a499aa24c118e83f5714152c0e22617c6fc63"),
		// Rank 3 - 15tRXfXoZXkjScB3Awv8s2saPjaicKYAyL1WZ3JP5kycoG9n
		// https://collectives.subsquare.io/user/15tRXfXoZXkjScB3Awv8s2saPjaicKYAyL1WZ3JP5kycoG9n/fellowship
		hex!("d8290537d6e31fe1ff165eaa62b63f6f3556dcc720b0d3a6d7eab96275617304"),
		// Rank 3 - 1682A5hxfiS1Kn1jrUnMYv14T9EuEnsgnBbujGfYbeEbSK3w
		// https://collectives.subsquare.io/user/1682A5hxfiS1Kn1jrUnMYv14T9EuEnsgnBbujGfYbeEbSK3w/fellowship
		hex!("e287c7494655d636a846f5c3347ad2cb3c462a8d46e0832be70fcc0ab54ee62d"),
		// Rank 3 - 16Q4qkRcWd4r8196dVGNLYVfy7H86MJYJBMockPaMigFXCyv
		// https://collectives.subsquare.io/user/16Q4qkRcWd4r8196dVGNLYVfy7H86MJYJBMockPaMigFXCyv/fellowship
		hex!("eec4bd650a277342ebba0954ac786df2623bd6a9d6d3e69b484482336c549f79"),
		// Rank 3 - 16a357f5Sxab3V2ne4emGQvqJaCLeYpTMx3TCjnQhmJQ71DX
		// https://collectives.subsquare.io/user/16a357f5Sxab3V2ne4emGQvqJaCLeYpTMx3TCjnQhmJQ71DX/fellowship
		hex!("f65f3cade8f68e8f34c6266b0d37e58a754059ca96816e964f98e17c79505073"),
		// Rank 2 - 126X27SbhrV19mBFawys3ovkyBS87SGfYwtwa8J2FjHrtbmA
		// https://collectives.subsquare.io/user/126X27SbhrV19mBFawys3ovkyBS87SGfYwtwa8J2FjHrtbmA/fellowship
		hex!("307183930b2264c5165f4a210a99520c5f1672b0413d57769fabc19e6866fb25"),
		// Rank 2 - 129EYiTbv2J4LkYqRNssUfMuxNLYN8TW2LgfG1Gqyj8wCcs7
		// https://collectives.subsquare.io/user/129EYiTbv2J4LkYqRNssUfMuxNLYN8TW2LgfG1Gqyj8wCcs7/fellowship
		hex!("3283cc9f4408df3ccaef653fc163e56509619c1e0e46bb4e677d227fa50bef7f"),
		// Rank 2 - 12aoZXwbUzsv3z5HF5HCrtEwBJYCeKne6rYsxFEKDZ86Wdv8
		// https://collectives.subsquare.io/user/12aoZXwbUzsv3z5HF5HCrtEwBJYCeKne6rYsxFEKDZ86Wdv8/fellowship
		hex!("460411e07f93dc4bc2b3a6cb67dad89ca26e8a54054d13916f74c982595c2e0e"),
		// Rank 2 - 12zsKEDVcHpKEWb99iFt3xrTCQQXZMu477nJQsTBBrof5k2h
		// https://collectives.subsquare.io/user/12zsKEDVcHpKEWb99iFt3xrTCQQXZMu477nJQsTBBrof5k2h/fellowship
		hex!("585e982d74da4f4290d20a73800cfd705cf59e1f5880aaee5506b5eaaf544f49"),
		// Rank 2 - 13WGadgNgqSjiGQvfhimw9pX26mvGdYQ6XgrjPANSEDRoGMt
		// https://collectives.subsquare.io/user/13WGadgNgqSjiGQvfhimw9pX26mvGdYQ6XgrjPANSEDRoGMt/fellowship
		hex!("6ecb07b7a8f63febc1c458983d4ec414e4cbb76ec3f128d9cedd5f4c4fd2965b"),
		// Rank 2 - 13aYUFHB3umoPoxBEAHSv451iR3RpsNi3t5yBZjX2trCtTp6
		// https://collectives.subsquare.io/user/13aYUFHB3umoPoxBEAHSv451iR3RpsNi3t5yBZjX2trCtTp6/fellowship
		hex!("720d807d46b941703ffe0278e8b173dc6738c5af8af812ceffc90c69390bbf1f"),
		// Rank 2 - 1436xp47dm3w1yTvSncsf4cgVLH5dVsgBzMqtHkSx1XjG1Wb
		// https://collectives.subsquare.io/user/1436xp47dm3w1yTvSncsf4cgVLH5dVsgBzMqtHkSx1XjG1Wb/fellowship
		hex!("864f430e2c463fb124c1ba204838d9081a88657cbb0ac86d109740923c5a8943"),
		// Rank 2 - 14uA7Vc828e2Q6oL5GBHP9UzTkEvwqbroERwRmucGrLmPuuL
		// https://collectives.subsquare.io/user/14uA7Vc828e2Q6oL5GBHP9UzTkEvwqbroERwRmucGrLmPuuL/fellowship
		hex!("ac7c228c0c2f9f8bd69a79694a21c0aaa11fa0bdffb8a24f8a2b2c7c71dd4464"),
		// Rank 2 - 1556APd4jcMDRod9SUxfTwGLasqFy3y3QFMGokkBwTdk2tev
		// https://collectives.subsquare.io/user/1556APd4jcMDRod9SUxfTwGLasqFy3y3QFMGokkBwTdk2tev/fellowship
		hex!("b40f4abb8698fdab096b0dcbe258ca86e64cd7cdd21633393b54fd74f22b1818"),
		// Rank 2 - 15roJ4ZrgrZam5BQWJgiGHpgp7ShFQBRNLq6qUfiNqXDZjMK
		// https://collectives.subsquare.io/user/15roJ4ZrgrZam5BQWJgiGHpgp7ShFQBRNLq6qUfiNqXDZjMK/fellowship
		hex!("d6ebcc75c7ea9a0c4459162b495e90c7ed5306e3a27f73125d6fbd2a34601323"),
		// Rank 2 - 16JGzEsi8gcySKjpmxHVrkLTHdFHodRepEz8n244gNZpr9J
		// https://collectives.subsquare.io/user/16JGzEsi8gcySKjpmxHVrkLTHdFHodRepEz8n244gNZpr9J/fellowship
		hex!("040a61ca8223dcb3c203b27167fdee611801375fff6c6f25c71c3e3ca86cea65"),
		// Rank 2 - 16JskuojL6mSp6HNcjiHYa9jqksWbLD8L9YGWU1ppiPWQ9sa
		// https://collectives.subsquare.io/user/16JskuojL6mSp6HNcjiHYa9jqksWbLD8L9YGWU1ppiPWQ9sa/fellowship
		hex!("eacf33e37aff83d33c7472ff69683a322ab320d9de25586311e75b6ac8270f5c"),
		// Rank 2 - 16YCL3UVpVWQLGW3p3Zx4k5WAEp9W1DwdDnxAbyAaPxVxnp3
		// https://collectives.subsquare.io/user/16YCL3UVpVWQLGW3p3Zx4k5WAEp9W1DwdDnxAbyAaPxVxnp3/fellowship
		hex!("f4f7e8c9dcf45daa322fa14585d8f14a37aceca839106ed3414bb04778696145"),
		// Rank 2 - 1HFq3DbX4tqanTLAx2CAToWnHXg6LRLMzSD4JzYCzCQpw5E
		// https://collectives.subsquare.io/user/1HFq3DbX4tqanTLAx2CAToWnHXg6LRLMzSD4JzYCzCQpw5E/fellowship
		hex!("0c65d7c1489d18dedea3b4acf5443bd0f088e6062d20f15e774ee860e39e3d2a"),
		// Rank 2 - 1QhVP5qzR2LfXqP77N1JcuwHoY7NH8JVRNFm1hSooE9d4pR
		// https://collectives.subsquare.io/user/1QhVP5qzR2LfXqP77N1JcuwHoY7NH8JVRNFm1hSooE9d4pR/fellowship
		hex!("1212f18a064433a237a56022beffd0ca0f0baef317b34c7f6f12b19968f10233"),
		// Rank 1 - 12GyGD3QhT4i2JJpNzvMf96sxxBLWymz4RdGCxRH5Rj5agKW
		// https://collectives.subsquare.io/user/12GyGD3QhT4i2JJpNzvMf96sxxBLWymz4RdGCxRH5Rj5agKW/fellowship
		hex!("386a4f5a0311a2834e28c84daa299fe14414137807e201a1941e502c7a784467"),
		// Rank 1 - 12YzxR5TvGzfMVZNnhAJ5Hwi5zExpRWMKv2MuMwZTrddvgoi
		// https://collectives.subsquare.io/user/12YzxR5TvGzfMVZNnhAJ5Hwi5zExpRWMKv2MuMwZTrddvgoi/fellowship
		hex!("44a3efb5bfa9023d4ef27b7d31d76f531b4d7772b1679b7fb32b6263ac39100e"),
		// Rank 1 - 12fiMKJP9t3gTFpHEXGYtdbZzwQciCpUcVFRnVXU4JYQLWvA
		// https://collectives.subsquare.io/user/12fiMKJP9t3gTFpHEXGYtdbZzwQciCpUcVFRnVXU4JYQLWvA/fellowship
		hex!("49c2c1981a0b162c31996d231b2dfcc63e0b10c0c63f8f921dd8e13f62011010"),
		// Rank 1 - 12gMhxHw8QjEwLQvnqsmMVY1z5gFa54vND74aMUbhhwN6mJR
		// https://collectives.subsquare.io/user/12gMhxHw8QjEwLQvnqsmMVY1z5gFa54vND74aMUbhhwN6mJR/fellowship
		hex!("4a4081d3f77f3ae9304f78983183b8f015dacb620ce7eb0444e733b85422d931"),
		// Rank 1 - 12pCUGSwoW4Xek48TLUHCFhrvAdjmciMMLJoRJD8HWP5saXH
		// https://collectives.subsquare.io/user/12pCUGSwoW4Xek48TLUHCFhrvAdjmciMMLJoRJD8HWP5saXH/fellowship
		hex!("503b6118faf45539e1aa5c28fb3e23aab869d20044ee9798d8b49f9fed8f0c31"),
		// Rank 1 - 1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM
		// https://collectives.subsquare.io/user/1333zsMafds2sKAr8nG3zwXTCHPYv2Nm6CRgakpu6YVGt7nM/fellowship
		hex!("5a090c88f0438b46b451026597cee760a7bac9d396c9c7b529b68fb78aec5f43"),
		// Rank 1 - 13jBAtYJar4xujPaEx41FxjSt9PqU7LqJRbySJiVdMtuWN42
		// https://collectives.subsquare.io/user/13jBAtYJar4xujPaEx41FxjSt9PqU7LqJRbySJiVdMtuWN42/fellowship
		hex!("78a302c5370300ab2bd8a65469a5061220cd07b3b3a17e5e3233d283e2ad46f0"),
		// Rank 1 - 14AgwoPjcRiEEJgjfHmvAqkjdERCG26WEvQUoGLuBzcXKMS2
		// https://collectives.subsquare.io/user/14AgwoPjcRiEEJgjfHmvAqkjdERCG26WEvQUoGLuBzcXKMS2/fellowship
		hex!("8c1860117351602843d192a9b4eb3b3641da38ab14c7974398761e7b7f3f3a14"),
		// Rank 1 - 14SRqZTC1d8rfxL8W1tBTnfUBPU23ACFVPzp61FyGf4ftUFg
		// https://collectives.subsquare.io/user/14SRqZTC1d8rfxL8W1tBTnfUBPU23ACFVPzp61FyGf4ftUFg/fellowship
		hex!("981971ee9a37cbccb18e0690120709269bb120ad578148a0852505b45af06c41"),
		// Rank 1 - 14VZNZ1x7QvkM1k48EgSmtn2wjpBMN2YN7iCfSW6cnt57H8R
		// https://collectives.subsquare.io/user/14VZNZ1x7QvkM1k48EgSmtn2wjpBMN2YN7iCfSW6cnt57H8R/fellowship
		hex!("9a7c8b4db61eaa9db696335e6f1ba52ca060547d31fa27634245c4e6a9798c42"),
		// Rank 1 - 14oHMAJ5btnDCusHrTWraw1wTsLJwZeqPDLxusm1R1Zh3Vxa
		// https://collectives.subsquare.io/user/14oHMAJ5btnDCusHrTWraw1wTsLJwZeqPDLxusm1R1Zh3Vxa/fellowship
		hex!("a801051533332369ef5a24d9d7d13709896d3e170297851c63123b26a4034137"),
		// Rank 1 - 15K1tpRFoFsGvqYU2358GE4hK85zQeiKcYo1HT9pnaepRs4U
		// https://collectives.subsquare.io/user/15K1tpRFoFsGvqYU2358GE4hK85zQeiKcYo1HT9pnaepRs4U/fellowship
		hex!("beae5bcad1a8c156291b7ddf46b38b0c61a6aaacebd57b21c75627bfe7f9ab71"),
		// Rank 1 - 15Sm4Do29Ci2X458Pwv9MJa52aqfQg6t2Qw3QGpEHpCS1SKK
		// https://collectives.subsquare.io/user/15Sm4Do29Ci2X458Pwv9MJa52aqfQg6t2Qw3QGpEHpCS1SKK/fellowship
		hex!("c4965f7fe7be8174717a24ffddf684986d122c7e293ddf875cdf9700a07b6812"),
		// Rank 1 - 165wJzybiNv9VVUypbNRiK5WPKZABTQ5hCFNr9qTAgNCJR12
		// https://collectives.subsquare.io/user/165wJzybiNv9VVUypbNRiK5WPKZABTQ5hCFNr9qTAgNCJR12/fellowship
		hex!("e0f0f94962fc0a8c1a0f0527dc8e592c67939c46c903b6016cc0a8515da0044d"),
		// Rank 1 - 1HXh7kCk2Z9Er4TpqF7TPX6ivSnJTECesp44RP7jnP7RCeL
		// https://collectives.subsquare.io/user/1HXh7kCk2Z9Er4TpqF7TPX6ivSnJTECesp44RP7jnP7RCeL/fellowship
		hex!("0c9b3e6a2fd9858cb1b14ebd3187f4418b40d7a1696e9562fd5db42e1343931e"),
		// Rank 1 - 1L66uQMKFnXKSZx9pCD5o56GvvP1i2Qns7CaS2AaKp9mnwc
		// https://collectives.subsquare.io/user/1L66uQMKFnXKSZx9pCD5o56GvvP1i2Qns7CaS2AaKp9mnwc/fellowship
		hex!("0e8ed639d511aae6e8213a795a521b6e088b292e45b9ff1e2dcf31cad748e91b"),
		// Rank 1 - 1RaxuqWvyd6sdAEiansxmtget47PVcsSR38d9V2uPzKu2vo
		// https://collectives.subsquare.io/user/1RaxuqWvyd6sdAEiansxmtget47PVcsSR38d9V2uPzKu2vo/fellowship
		hex!("12c039004da5e1e846aae808277098c719cef1f4985aed00161a42ac4f0e002f"),
	];

	/// Grants a storage authorization to every rank-1+ Fellowship member listed in
	/// [`FELLOWSHIP_RANK1_PLUS_MEMBERS`].
	///
	/// **Temporary.** The grants expire on their own after
	/// [`AuthorizationPeriod`](crate::storage::AuthorizationPeriod) (14 days) and nothing renews
	/// them, so this needs no follow-up call to undo. It buys a window for quick sanity testing of
	/// the chain itself (Bitswap retrieval, collator p2p setup, …) before Proof of Personhood
	/// becomes the only way in — see
	/// [#1233](https://github.com/polkadot-fellows/runtimes/pull/1233) for the Individuality
	/// integration on People and Asset Hub.
	///
	/// Deliberately **not** a `VersionedMigration`: it introduces no new storage layout, so there
	/// is no pallet version to bump. Idempotence comes from the `account_has_active_authorization`
	/// guard instead — members who already hold an unexpired authorization are skipped rather than
	/// having a second allowance added on top (`authorize_account` is additive on the unexpired
	/// path), so re-running is a no-op until the grants lapse.
	pub struct AuthorizeFellowshipMembers;

	impl OnRuntimeUpgrade for AuthorizeFellowshipMembers {
		fn on_runtime_upgrade() -> Weight {
			let members = FELLOWSHIP_RANK1_PLUS_MEMBERS;
			let db_weight = <Runtime as frame_system::Config>::DbWeight::get();
			// One `Authorizations` read per member for the idempotence guard.
			let mut weight = db_weight.reads(members.len() as u64);
			let mut granted = 0u32;

			for raw in members.iter() {
				let who = AccountId::from(*raw);
				if TransactionStorage::account_has_active_authorization(&who) {
					continue;
				}

				// `authorize_account` is benchmarked for the `Authorizations` read/write; a fresh
				// grant additionally bumps `frame_system::Account` providers.
				weight.saturating_accrue(
					<Runtime as pallet_bulletin_transaction_storage::Config>::WeightInfo::authorize_account()
						.saturating_add(db_weight.reads_writes(1, 1)),
				);

				match TransactionStorage::authorize_account(
					RawOrigin::Root.into(),
					who.clone(),
					FELLOWSHIP_TRANSACTIONS,
					FELLOWSHIP_BYTES,
				) {
					Ok(()) => granted.saturating_inc(),
					// Unreachable with a Root origin and a non-zero `FELLOWSHIP_BYTES`, but a
					// single bad entry must not abort the remaining grants.
					Err(error) => log::error!(
						target: "runtime::bulletin",
						"[AuthorizeFellowshipMembers] failed to authorize {who:?}: {error:?}",
					),
				}
			}

			log::info!(
				target: "runtime::bulletin",
				"[AuthorizeFellowshipMembers] granted {granted}/{} authorizations of {} transactions / {} bytes",
				members.len(),
				FELLOWSHIP_TRANSACTIONS,
				FELLOWSHIP_BYTES,
			);

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
			use frame_support::ensure;

			// A duplicate entry would take the additive `authorize_account` path and hand that
			// member twice the intended allowance.
			let members = FELLOWSHIP_RANK1_PLUS_MEMBERS;
			for (i, member) in members.iter().enumerate() {
				ensure!(
					!members[i.saturating_add(1)..].contains(member),
					"FELLOWSHIP_RANK1_PLUS_MEMBERS contains a duplicate account"
				);
			}

			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
			use frame_support::ensure;

			for raw in FELLOWSHIP_RANK1_PLUS_MEMBERS.iter() {
				let who = AccountId::from(*raw);
				ensure!(
					TransactionStorage::account_has_active_authorization(&who),
					"every Fellowship member must hold an active authorization"
				);

				let extent = TransactionStorage::account_authorization_extent(who.clone());
				ensure!(
					extent.transactions_allowance >= FELLOWSHIP_TRANSACTIONS &&
						extent.bytes_allowance >= FELLOWSHIP_BYTES,
					"Fellowship member authorization is below the granted allowance"
				);
			}

			log::info!(target: "runtime::bulletin", "AuthorizeFellowshipMembers is OK!");
			Ok(())
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;
		use crate::{BuildStorage, RuntimeGenesisConfig};
		use pallet_bulletin_transaction_storage::{
			AuthorizationExtent, DEFAULT_MAX_TRANSACTION_SIZE,
		};

		#[test]
		fn grants_every_member_the_configured_allowance() {
			let genesis = RuntimeGenesisConfig::default().build_storage().unwrap();
			sp_io::TestExternalities::new(genesis).execute_with(|| {
				AuthorizeFellowshipMembers::on_runtime_upgrade();

				for raw in FELLOWSHIP_RANK1_PLUS_MEMBERS.iter() {
					let who = AccountId::from(*raw);
					assert_eq!(
						TransactionStorage::account_authorization_extent(who.clone()),
						AuthorizationExtent {
							transactions_allowance: FELLOWSHIP_TRANSACTIONS,
							bytes_allowance: FELLOWSHIP_BYTES,
							..Default::default()
						},
					);
					// The grant must be usable right away for a `store` of any allowed size.
					assert!(TransactionStorage::can_store(&who, DEFAULT_MAX_TRANSACTION_SIZE));
				}
			});
		}
	}
}
