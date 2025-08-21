// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Migrations.

use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, weights::Weight};

pub(crate) mod add_accounts {
	use super::*;
	#[cfg(feature = "try-runtime")]
	use alloc::vec::Vec;
	use frame_support::parameter_types;
	use pallet_ranked_collective::{
		Config, IdToIndex, IndexToId, MemberCount, MemberRecord, Members, Rank,
	};

	parameter_types! {
		// Public key (hex)
		// This list was created by collating community-submitted addresses from an off-chain document source
		// https://docs.google.com/spreadsheets/d/1uE5nDKuMZDqlj9q2tvnk_tngyU1Cokl0tQKwSigvJLA/edit?gid=0#gid=0,
		// then converting each Polkadot SS58 address to its raw public key (32-byte hex) using:
		// `subkey inspect <SS58_ADDRESS>`
		// 
		// Ensuring compatibility with the runtime's account system.
		pub const Addresses: [(Rank, [u8; 32]); 126] = [
			(0, hex_literal::hex!("54361bceb4403e1af7c893688a76c35357477da7e36371b981728ddf8f978e0c")),
			(0, hex_literal::hex!("c0c799b66754bfb56799dfef8071772d8c5ea2a87dd0c969493066aed94e645c")),
			(0, hex_literal::hex!("30c9d60350b04b6bce9b4c692b2db6ab91a16cd990952716de59e4dfbc79406f")),
			(0, hex_literal::hex!("ea0ab1b08b58a3708b50ba9928c4e25ad71d68efcbb868a2f75b987d0e8e4108")),
			(0, hex_literal::hex!("08e1973238f78c6046f097a1aebc62d8590156b0ae414c3148cb8c8a5fee931a")),
			(0, hex_literal::hex!("0ed039e96de24a2f5e9954c9bddab6ea98712e16dc41140e715e9a98d9eca64e")),
			(0, hex_literal::hex!("46cd0f31add423547c14efd7593f07807c1783153317308d52c6fc9995ae2067")),
			(0, hex_literal::hex!("76126bccbd03939a60016a0719775d47876f7eb25713d165e592a81a7ce2957c")),
			(0, hex_literal::hex!("16b8eaf319bfa79d8d5494f3e3f20d6a5bb37766433d500eb1eac2fcb816276c")),
			(0, hex_literal::hex!("28e41f254f174a58c5499459af0f1c8834ebbcbc2402ee963707950c69480a77")),
			(0, hex_literal::hex!("6c7499cc79bee0f02862e75504c7c5924c9ea55e977fd5c23407919a7addb258")),
			(0, hex_literal::hex!("b4fbf400039d8159aa0ebbe79890cc0688187e353d1be52ea64e7772d5b73077")),
			(0, hex_literal::hex!("ae71605d54343a5b19964e876da7aaddaa8a6c9e17244d7839f344eefcce2c6c")),
			(0, hex_literal::hex!("325b2d831d106d13bf9436e4bd49c995032913a57d6772e3fc794ea200a42d67")),
			(0, hex_literal::hex!("1e5c61cb6941b247d22fa14392fb8710a23493db5857c2904a76b3bcfda7d217")),
			(0, hex_literal::hex!("f8c5e1a202bf503c8c0a8afe149ac475133431e2dcabe51e648703df550e5105")),
			(0, hex_literal::hex!("be40bdb401cc68a638044a6308e943ff799db9b7115037d67ccb6211ea03de7e")),
			(0, hex_literal::hex!("1c5c349b5335301171622573011bcc7739308e5fe611686b1ad0c4c852f3894e")),
			(0, hex_literal::hex!("12436ca45919fd6dd2c3f63dda8deac7545f7e1248bac71331e26260be490f13")),
			(0, hex_literal::hex!("ba14b5c1cc2d51537e2e74060a0003527fc885a51d4fd8a1370119322c6c9204")),
			(0, hex_literal::hex!("986fc63d9b3ed67f2f6b1a87847158b9a19dd61110585203543c3b2ec0b5664d")),
			(0, hex_literal::hex!("366e5c14e95218d6b71046b48a01e2450b2efe6c85c5d7b336e00c0d2effc14c")),
			(0, hex_literal::hex!("3aa151f9bdc2bb6f8e27d492446ace702955f1eed20efa5ee31a59dafdf3354e")),
			(0, hex_literal::hex!("0c7f10142a81fedec753f7c556f5b93a400c280805e7fcdff668719637b13434")),
			(0, hex_literal::hex!("7081fe1fe013c00fbe52543ccd437d45fc26b0d49b7df5c117de02c4193c7d12")),
			(0, hex_literal::hex!("a23b0237712599b173616f4d9f7d5b8e93d520bb54ca10a4f0d93b842dfea912")),
			(0, hex_literal::hex!("24bd04480522d65a8e7fe8d4537d7780ce1d75693e02a5d08ec9aa4ce60c1735")),
			(0, hex_literal::hex!("8c1860117351602843d192a9b4eb3b3641da38ab14c7974398761e7b7f3f3a14")),
			(0, hex_literal::hex!("7ef71c2632780ed5f5e2534b441e3dd57754644aa629c7297564fe7b5a74c12f")),
			(0, hex_literal::hex!("ea614ce8bc4baa3c39e8019636cc6f8d9a67291f19dabb2d898a308b6b93b72e")),
			(0, hex_literal::hex!("525a7278bbbfd26c7ed1d83365d7ee6cab307d0419a0e41cfbeefc0682803576")),
			(0, hex_literal::hex!("4ebc7602bade859ea93f2a43445db8b8d140da3f58dacd74f165f3dc8c288e1f")),
			(0, hex_literal::hex!("68bb9a3ef8b0732fae1a1eb2ccd0c0ba134cef99bacb3f31caa8dedff4d19823")),
			(0, hex_literal::hex!("d4a3507a544b4a0f9b2e1ffc293ef2b2936b67dceed36a8ab0488321760a3c2e")),
			(0, hex_literal::hex!("1ce0129f71f781f64748e8393c75d6481121e653281143bc5e2e66af7947374b")),
			(0, hex_literal::hex!("26f6ba0b2c596d1a471444945c9fdbdfe25e3f8862096f8f6de11d87da1a7111")),
			(0, hex_literal::hex!("54e20840d041d6626327c3ee6c0555e991b4e893e6b998f269bc85f1a7503f1a")),
			(0, hex_literal::hex!("4e1b6a6d9537722aedf9a33577c254942c64713ff96ad157ed483b0f6fcb291f")),
			(1, hex_literal::hex!("9229823b8640b9816abc85023a24492d04bd77d63a4037137099138fb7509770")),
			(1, hex_literal::hex!("1271a802d2e48417a15fd6f26341df049c5d36c9ef9139fc703f9cddabb6fe67")),
			(1, hex_literal::hex!("48a751e3ab512dda0be14000c63798a628ee5c8bad55e947c356dbf38b0b0210")),
			(1, hex_literal::hex!("669484e16015ec918956eef3efe7edaf85607dd642305bfb25529fcd2c3acc68")),
			(1, hex_literal::hex!("46c0928dcd420a1399b1f06de4cf2b30f79ad0acee1d6b63f8132265d9a74d60")),
			(1, hex_literal::hex!("0a82ffd7ae323eee7b33e2101be3a9d8bc3402e1f3f4ac1e21921752775be80d")),
			(1, hex_literal::hex!("4c8dfa422782f2769f18d9c8e0a3d56e44f2bc3a93703bf4d862455b8494d904")),
			(1, hex_literal::hex!("ba0f7abffbf5ce25cf269ea9971cc44c47aad5bff9ade8ba7dfd10c44e35f853")),
			(1, hex_literal::hex!("0250b3c3f57116b33f8074d60bce91c36c99ff8dc94c18e7648f35f2b0ccf049")),
			(1, hex_literal::hex!("fec42f0a29c224360b8d4798c96c0ae4840d1307f88552afca553113548f5628")),
			(1, hex_literal::hex!("a4d26dd19482ab1dfae06825850bc2a3e3c82821977573c80eb9f19747b9215d")),
			(1, hex_literal::hex!("1c3c4a573abf19a85b8aadaf0aedeef1ba07eed8b148449f66e55ced9a8c0d66")),
			(1, hex_literal::hex!("06e496c8f09c262953449758d1ac4299ab148c657d2b41eebe82a3c89b576b20")),
			(1, hex_literal::hex!("f808c48004f1454517bd4f0e52176a70102b67ab54fd6000925e9eaf5013332f")),
			(1, hex_literal::hex!("e869dd0892267ae94e5d929445a321212f20e4cd0b93d4c8529eb3cfb67d0611")),
			(1, hex_literal::hex!("ba03b3505b9415ad05cd4d6af8a68bdee39325677d518db05e4798697308b979")),
			(1, hex_literal::hex!("28ab88925fdc4f4a76394ab871afc012363685ae81accc030eae543282e46e20")),
			(1, hex_literal::hex!("48a53cd250eb816b6a3f190b655b8996d8204f4d84a590ac34133f2d176a012d")),
			(1, hex_literal::hex!("f8c309d6472fab1a89e619867d57934db759e5d76d63b9e67968e36f02787335")),
			(1, hex_literal::hex!("eaf08a7a37c3715a707645ea7fcbc29663002e19a1fc08e99e5dee6fe9793e4e")),
			(1, hex_literal::hex!("2297873efa230fc46f3ef8995ae5dc6c2131878085eb3285dc16cbbb56e4813b")),
			(1, hex_literal::hex!("b03c27bfcc8db6244c7eeaf11af18f1cc7b27af0bb919198251679c2b29f9565")),
			(2, hex_literal::hex!("29702dace3302756acd9d53ff74a858eab8f13b1ebba43bdd839085c00045c72")),
			(2, hex_literal::hex!("9e72780676ac4224839bf5bbf3af14b071c6ebad7d6fa6e516973a7a29295477")),
			(2, hex_literal::hex!("bc6e12d7ab70abea4c08db7055e84f16bab817b5fb359088ad5190422df9dd1d")),
			(2, hex_literal::hex!("cca044696fc5f11711266f7da0da4d0b94e34420cdcaa9f3e9e06939e551d27d")),
			(2, hex_literal::hex!("284492a0069965cc3f671d9e4d583cdee2bf11356546da5fd6a0e0c19f50f93f")),
			(2, hex_literal::hex!("c69d8c568b24a3108b6f9604f118359c268804e5a0de2b415ce978160dec2359")),
			(2, hex_literal::hex!("0854e5b78cbe4d011039a6d96faabe676c919bfe1809aa8928cad8f68ec10b71")),
			(2, hex_literal::hex!("d2eb07f02043788e254d9e2df57be11566d241c56302b91199b4647947af3020")),
			(2, hex_literal::hex!("ca2ad6a831977f590149f1417d4fbff7d19cc11932f5c34ba90c82729cb8e02a")),
			(2, hex_literal::hex!("80b786800110748e757290c5e221ff8ec45760e14a18914c4ba553dad98e9660")),
			(2, hex_literal::hex!("1c20046a7e79f6a36c1f8f74a8caf1e573c448f7f2ca4d7e75928579ddbc134b")),
			(2, hex_literal::hex!("1ccdf130995bda0c195a1c383532e4704f289b31d3923c18f76ff0ef6626f21c")),
			(2, hex_literal::hex!("7837d6d2430ff5a0e6f8b7aa57f325f01ae204332d4cda591324dc09b5da1268")),
			(2, hex_literal::hex!("e29d9024ce94e2d3636c627905efa536454fa6192b25cd3932d7269221db4c54")),
			(2, hex_literal::hex!("98f37af89509e846463f6874745b6f8c4e02a941d2c9a83b7d60d90ad5b81863")),
			(2, hex_literal::hex!("da95b7b5c463b7d00700c34294c283a21235f8d31377620dcd1a56bf60f3e320")),
			(2, hex_literal::hex!("c2ccc9706a4b7920839b77caa26adb2c7e4bfe5568718a8ca0067959aee59611")),
			(2, hex_literal::hex!("6811be18d13899d8077f1c3d154242567a57906621fe11984b63cda8cac33703")),
			(2, hex_literal::hex!("7ea2536ee6f4f8ae18d99ca7026d6c50acf16f7015558ee73bd05bc98a142e35")),
			(2, hex_literal::hex!("c2c3ba1a57b2c2480a3dc66c243cf32ec46640b185546870f4141134441b1403")),
			(2, hex_literal::hex!("560bec0b709ab4673bf4ff21e7a5858b3b4d2ab52a254d0786136c9105c83ea1")),
			(2, hex_literal::hex!("4a5e6693907b2e0647ddb0706e89f7227064060c1de7d10cadee84da22cd8360")),
			(2, hex_literal::hex!("20d84e9faa874218a9ded1fa7135b338d67b22d8eb75876acc73d69e9cdb036e")),
			(2, hex_literal::hex!("80d0f3b4ac7e29a54b4b4c6638d0a865aba5da8d2881ceac549c5e278e62a90d")),
			(2, hex_literal::hex!("60f94710848d9dce161724f257a240494c901728bdf2fa51c138fc5580ee3134")),
			(2, hex_literal::hex!("9681fe005baa3099a7d9c03a46e539b173d2b3de75b11e86cd5fbb7d4e92993c")),
			(2, hex_literal::hex!("42ae1e94aeaccefd1e31a68ee716a34a11c92f4f2cf984319a212f056b05655a")),
			(2, hex_literal::hex!("9692fa834a36faff24619a5a9559ce35082ea5247cfb0657e8ce2fe5fcce2d3e")),
			(2, hex_literal::hex!("fea195aa035fec06c1984c580c87a55253a1ee6581240a2278113305ef498a32")),
			(2, hex_literal::hex!("580ecbcececdc45961e340519fa57b4e9c33e1c7798c06ddade7309f89822c3b")),
			(3, hex_literal::hex!("1eb38b0d5178bc680c10a204f81164946a25078c6d3b5f6813cef61c3aef4843")),
			(3, hex_literal::hex!("1480bc228ee751c1aca34061c4952efb304aa94beed8e38fd9c5e693f62c3f26")),
			(3, hex_literal::hex!("b45983717ebdd396d31f95c98aa6e91595135508e97634724d9bd2fe4bf48008")),
			(3, hex_literal::hex!("86bd113dd4b242e8e82e41164cb78a721f5ddc270a8df853ffe549166765a059")),
			(3, hex_literal::hex!("8200b8e9ac57a75ac09201619f379352709a392fcd19eb24adba7db2a115f948")),
			(3, hex_literal::hex!("5604806a738c7e4d516625d47db7a4ac3197142b84203c1495b6689b066d9d44")),
			(3, hex_literal::hex!("e289e8c5bf9a8f4c0305cdc85a415d80f09b02e3516b64d6c33a56ebf1fd286c")),
			(3, hex_literal::hex!("42a8f4af92bc51e83093eeecb73f2aab526a11c41735ff54d9fc7de54ace5c6d")),
			(3, hex_literal::hex!("969f7c9e5a153c9d9965ae28a973fdd9ebc270fe431e04396c711e12c6dc2356")),
			(3, hex_literal::hex!("96c952e57d77a6acc899f39e7f043839309cc8527b089c5ca76d24c2b63ebb4b")),
			(3, hex_literal::hex!("1035801fd00144e10a3933ed859f8236bbffb93a7ac515bab9f1ca53cbb3f776")),
			(3, hex_literal::hex!("28d50241999da5b300f01f3004a67a25a11854608f1f437ab86ed2e115243a43")),
			(3, hex_literal::hex!("aa48a9775be7af0521acefce054dc7e9e461814dc167a5cabf52aef8534d8249")),
			(3, hex_literal::hex!("0ef2cc1000f878a3880a09d698b5375f20c4ab3d8b3a1b783c8150faca3da65a")),
			(3, hex_literal::hex!("e86b14052f27742916a13482c39afc8d9a8f873d799c9aa070bbd045570baa66")),
			(3, hex_literal::hex!("742f10a2b57e5ec3247c0ae6da2a2fdb4a731324dc7a2edfe0f4fc761e1a4d3f")),
			(3, hex_literal::hex!("4c2ded7ca2dd19095123de090a46149c2047d0aa4c6ce195490563d881f7491b")),
			(3, hex_literal::hex!("542aede1ac86f5f0326efb4b2edde2b0ccd56635d8d76e9ae86d9d93f8d02063")),
			(3, hex_literal::hex!("a0e1c76edd8151d4841665925d01a029fe176eecc24297bf931d594502293159")),
			(3, hex_literal::hex!("f8ff75032359f0de13264a134c3e88089cf6a2a31e5cf3cdfe405a4e272f0508")),
			(3, hex_literal::hex!("aea3ca653928298cd4d1d64cf916aacf72e4e2ca435453941a73327a8dd0a00b")),
			(3, hex_literal::hex!("a6796c4e02fca3ded862fce7bab207c1eadac8af1028fcb9e5bf4a0fa3aaae29")),
			(3, hex_literal::hex!("1cbf2d072567bdfeb00359e9d318e7b425a65449eb94b1a8f5ca0c28a9513878")),
			(3, hex_literal::hex!("c07b9222722b0cf30cd9034487a054a00ff3977ac034a7c05caa1b1ac45c8e73")),
			(3, hex_literal::hex!("0cd1574cf7fc649a8571d82427b42732e83bcd62f0e3192e92f202ca06f0d53c")),
			(3, hex_literal::hex!("0c691601793de060491dab143dfae19f5f6413d4ce4c363637e5ceacb2836a4e")),
			(3, hex_literal::hex!("2e5157fa386365bb2a60fc6f415bf070acdae47078be1396e3dc47ccb837d459")),
			(3, hex_literal::hex!("58efadc57a1952fc5829948986e5b86e2b7873ee16510800628e8bfd0344ac5a")),
			(3, hex_literal::hex!("86c3585c906e4928f030b4735d375cee0410db104908788133281b53533b5633")),
			(3, hex_literal::hex!("2055808c210d863dfc372ec85beafa8fd3a8ff497f8eaee401ef05bf27d3065b")),
			(3, hex_literal::hex!("da92c32ab2b4e1a46bb659cb6fd22fc824611e4c2803fbedd93c246f97c67118")),
			(3, hex_literal::hex!("6ae93e7162785a77d3a2c0413a9ee04af1b948ba5df9ac191552b72e1dd49b71")),
			(4, hex_literal::hex!("5c23daa77af39166a326377a006496753620093e13210a8567abed59e340eb06")),
			(4, hex_literal::hex!("d6b8ec23dc68f20b5d315007d9c1a6706f9bd5c883319181129e76a89e978155")),
			(4, hex_literal::hex!("00213c9e2131a7e4b10266e379a06774bf4a6a5e9f5632f31cb16bebd2cfb644")),
			(4, hex_literal::hex!("6a7cfc5b4a583e6ebeddeb24fcb17ee5cbfd5357bb41effa87adbe2a9e46fb41")),
		];
	}

	/// Adds the initial members of the Ambassador Fellowship.
	#[allow(dead_code)]
	pub struct InitialMemberSetup<T, I = ()>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> OnRuntimeUpgrade for InitialMemberSetup<T, I>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
			let has_members = <Members<T, I>>::iter_keys().next().is_some();
			ensure!(!has_members, "the collective must be uninitialized.");
			Ok(Vec::new())
		}

		fn on_runtime_upgrade() -> Weight {
			let mut weight = T::DbWeight::get().reads(1);

			for (desired_rank, account_id32) in Addresses::get() {
				let who: T::AccountId = account_id32.into();

				// Set collective pallet storage
				let record = MemberRecord::new(desired_rank);
				<Members<T, I>>::insert(&who, record);
				MemberCount::<T, I>::mutate(desired_rank, |count| *count = count.saturating_add(1));
				let count_at_rank = MemberCount::<T, I>::get(desired_rank);
				IdToIndex::<T, I>::insert(
					desired_rank,
					who.clone(),
					count_at_rank.saturating_sub(1),
				);
				IndexToId::<T, I>::insert(desired_rank, count_at_rank.saturating_sub(1), who);

				weight = weight
					.saturating_add(T::DbWeight::get().writes(2))
					.saturating_add(T::DbWeight::get().reads_writes(2, 2));
			}

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
			ensure!(MemberCount::<T, I>::get(0) == 37, "invalid members count at rank 0.");
			ensure!(MemberCount::<T, I>::get(1) == 22, "invalid members count at rank 1.");
			Ok(())
		}
	}
}

#[cfg(test)]
pub mod tests {
	use super::add_accounts::Addresses;
	use crate::{ambassador::AmbassadorCollectiveInstance as Ambassador, Runtime, System};
	use frame_support::traits::OnRuntimeUpgrade;
	use pallet_ranked_collective::Rank;
	use parachains_common::AccountId;
	use sp_core::crypto::Ss58Codec;
	use sp_runtime::{AccountId32, BuildStorage};

	#[test]
	fn check_addresses() {
		let addresses = Addresses::get();
		let ambassador_ss58: [(Rank, _); 126] = [
			(0, "12uR6ZinxBstfeh9zX5d415Z29XkSfNR4Nfkn6afAusKc52n"),
			(0, "15MmVHDWD5gkJzJfGVRGeHewNmRFqbuMXUSZN4spR22Lu9cM"),
			(0, "126yFs7wRkEsknx7NKWrw4UHhUcgcxR8XsiSdPfcpaoLXfe3"),
			(0, "16HsP9V3D2WQsKdaeyBBLmvSRXLsH5So62vnXoJumqGj5j8U"),
			(0, "1CeQ49R2mCScmBGuxoeS9FDq9Ssnb5xTttVWE6zFgRpbL2d"),
			(0, "1LRXY86unTYZgExx1cQnHxscaJb7VC8YFaPAWby5mbdCsgp"),
			(0, "12bqGW8fWoxawNkZWat5VW8UT3EP1rrsFsP8p3V2g8BYf2ZE"),
			(0, "13fp87SGGtXfCEdwqu9v7tVcb34gmhEGM1zDHvDAFzC3Yjk7"),
			(0, "1WnzAGy4czRWseZxJmHG6sgfEyaBrGtFsMgapFooryqtNFj"),
			(0, "1vcgYAMi3jNBugvufvo5wZWioArkSp1vDos8PLrMYu1Gwp9"),
			(0, "13TCowCJVpcD1iezJTFMaBPBm6xMyGyhYTYqoiavsfky4jox"),
			(0, "156JTy81GoyNtiAZYyXoivhoWzzz7NcMmrqJeQBs4Qhn1A61"),
			(0, "14wj1gbmKVLs61qczztaADNAYHtQ1TJyDneJFXZ6GSXDkTDo"),
			(0, "1292Uph4BwS9zcpoAextrGYJXFSC8NDuYnB5zqE87Er9y4AU"),
			(0, "5CkWjh9tdPkJCFVmbSknjDk8MVuUinhu6fCnbu6DVxggLpbv"),
			(0, "16dBfC3PW1KsDwXTEQzN66EPccPRMEwPJPoAvTyCYx3ubZkL"),
			(0, "15JTL9nGQHqDzd5dXRCMH9FeYdAeZghFYS4fFWd4Ye2ineqT"),
			(0, "1eBjRrKf7Kw1rSN6RynXsuWAspS2sbeV2T2Gy59EamiSucv"),
			(0, "1Qwtgq4cUrMiumWDjJMKTuvHs9nffnQWnBV22bccHGHBJtJ"),
			(0, "15Cz4QVNWdyaoCXbg3ZYHZqReHk89Tm1tEHZdvtYgiSGjdeV"),
			(0, "14SsUyoBYxe5WnJxoq7N3QXWz58x7dfXmsakMDRHxHQFgJvo"),
			(0, "12ENNRqKQN344Q6W1oqFnWUsmosX7p13qBUsNjPtdbdnaoRm"),
			(0, "12KshZsdpQueMDcbLnoXCYRt9z5dzknGkzCQau4LxUi7r4bx"),
			(0, "1HPKZzzd9nyr2DdvtPxytNMZm3Ld5nh3BBY4Ecgg9JxgL7G"),
			(0, "13YWynHAu8F8uKZFbQwvPgJ67xizvo21HCEQU3Ke8z1XHoyT"),
			(0, "14fiHcyMWAjihRyr2GxBv97tL6Sg8XLUzLYRBeqMvBpnuZcX"),
			(0, "1qAsfc59XsRLiy3DZfc4buS4umW6Q4xaJcWALj1eguZTi75"),
			(0, "14AgwoPjcRiEEJgjfHmvAqkjdERCG26WEvQUoGLuBzcXKMS2"),
			(0, "13sUSYigbmzszbAoLPDs5xKPAgQW7QBd2sjoWs5gpE66smkX"),
			(0, "16JK7LdoTGVoQy42YUiQcKjn1jz5WgUWjTENpGdFhg3PGX9X"),
			(0, "12ryo7Fp21tKkFQWJ2vSVMY2BH3t9syk65FUaywraHLs3T4T"),
			(0, "12nEijmsvxXbmdatam12kE6xT1K3Wd1xiHvvbWwDg5GLhPVJ"),
			(0, "13NKiA6MbAy5EVFfTgifbFUcAsPfc19mAzNzD7kNq1qTNmst"),
			(0, "15oofdcsxk1awE8ECSmhrK29vuCYDhp6zaTxvfu7GsjFSSqT"),
			(0, "1eruT12YRM5fDDn78PvRFwHdiWGd7NYmwhLwghZzYQ3t7dZ"),
			(0, "1t67YMM4myXZ745px9a8PNWhU8FMyYJf5uXGowuwSAnp4x8"),
			(0, "5Dz12QcEqTua1tTHWXuDXQjeZ5LvJ89R8z1oWTf1EG37RQUs"),
			(0, "12mQszZpLGLCpjAgUbPFWHQH8iWBzVrrGG1qhYeYns3LJN7V"),
			(1, "14JeKfePeBmNqdz85iH81wqPWjMq91oHhdshJ4WeCfHHs6vk"),
			(1, "1RBdF9imS5AyFhfcofWVXYfu74apATkf5RxXapeijHQYjVp"),
			(1, "5Dhy1VMbh7RkYJUV57kaZQPaCEdTbkSiSCXTnd4dyWhtF3C9"),
			(1, "13KW1DAoTjRhY2xDRCQ8EoBcpkmnKwaNvyUYxb6cmu1ra5mR"),
			(1, "12bmZMnjETAfpziCPHA2FujxtCA31kzcuj5ZJazM26YfTyMM"),
			(1, "5CJVFWTddhk6H8fzqthkNkMuFkFQFsDWkKNQLhXPYyHfdJG1"),
			(1, "12jNpHxcBibMb71GyaqeHZTCQfxLSes7gDGqyj4BvZZhaYrH"),
			(1, "15CxWHZrsWnqojXLD6aWq25uBRxX9A32KLw1kwSv6et72iM7"),
			(1, "14352EhN7Gvr1QKwo9gtUESHnXwoMoT4Xno4jTwtqFYrHkP"),
			(1, "16m3Sc5SEzHHGbiqYjK2HsQJgYfK7iRKuxqTRCyDtZ9CgH6T"),
			(1, "14j7N612MVBzLDw5rctvExdkF6P73uCoSdkFzdLDFJUYeb9n"),
			(1, "1e2FYZD149CBr1gX4sL4SW6nHABRy4g2rspvr9Bw1EsiewM"),
			(1, "1A3CAKernZo3hGaQYNmfuhK1wLG51VPhV95VH7NrHPVAk8M"),
			(1, "16cDUqTTLMS1xhD1BC7WXB7GcN4Dazo2APnafR6vVemZQm5a"),
			(1, "16FjZKbsPfF3K8Gzq7qvzffXrtAPL3BaS25opTLK2WHNenhA"),
			(1, "15Cu1LZvpzQFLF6vMCoVki5qfEbZ7x83aVk4eEQRdxnnTBE4"),
			(1, "1vKsYeVs6GicmZ7wMxAzPSCNmnri88ztqo6FJKvgRH7Eixd"),
			(1, "12eFXwk1qzbV6iQTUMjA5dvtCYFRpWvsHUTp1j7qmdX1CMHv"),
			(1, "16dApD2ox2qV2mA3u2xiR72kyAGE7MLbDxepZUFkcSJz3Gmw"),
			(1, "16K3fKGLTHTKvmcrVzk3Q15NWycHCS8QaMis1NLy91minNQG"),
			(1, "1nMeCddyV5v56YGk45qWS959Ejsr4hLmWCTmjtFLx5viTWX"),
			(1, "14z5JFyjNaKqiHCnWTz1ub3Vx8zJFJ9XYQL3yGVy6QRhtAET"),
			(2, "1wLHfDrpGNS29oHZuJn1ufKNwNDs6zi2XcZX9a9NjJVt6rm"),
			(2, "14aka4w1Sfiau5QpGGm71Zose4UanZDVtZqYnAk7WUqnCSav"),
			(2, "15G4hfDNtNhRc82As8Ep2YfvpM5xVdX7De3P9qSdHerGA6wC"),
			(2, "15dJNuttevEiJXv5gqN1B5G4MD3hWE1NVZkEsiP4BHSczKpH"),
			(2, "1uoHaGUTNd8qLhBEz9U5gq7BKLkHiDiTvqm5c3gpzBUQQbi"),
			(2, "15VRHQD9B2bhzjRxadArYu92uWPrE3GmvFb4NjqUNMqfhgUs"),
			(2, "1BvbxzsZEFGu5UjmMC5H8ZWC6FqCBaLHaVzZsPFEo8v1gMd"),
			(2, "15mYsj6DpBno58jRoV5HCTiVPFBuWhDLdsWtq3LxwZrfaTEZ"),
			(2, "15a5Q7ERVMYBAewqH7aNY2sEnsv4WU9mxD9y8rP7EezYCdQQ"),
			(2, "13umeZmNByyew5oAENQoNXgxqkKnxMZq6fpE7yxwBa1JFKur"),
			(2, "1dsrQjL34njJ4Y8FXGyxeLnmunPZ6XAvid9jSQe9S4pTUh2"),
			(2, "1emX4nsNHZwbbCej7UMC8gKNpXoy1K3ABwRmnVJLg5Z2PU6"),
			(2, "13idLN6FHW2YeJojk37oaM6w8pFo7FxySgv11326KcoQCs2M"),
			(2, "1688dRRSQ6yvT4adhjwNeNsnNVDMU3nmykanhQHrkecPNZb7"),
			(2, "14TYcDnUxErrjP6zsye12WSGxnucdLDqJik5LEsHnjxKnp4W"),
			(2, "15wbv2fXuFoeRcrWK4Q1Y5oM9LfsJyGmMXaUWDoAWXpZzWSp"),
			(2, "15QR8CGnmQxqjoWJrEN5cVbmJNjWt12bkyYTjJs5GN1ZMLDT"),
			(2, "13MTFYCHLwzuj5v3qpp92rS1t5fzQHuu82suxychrptorPi5"),
			(2, "13s3FkbuQWoN4kUbtvDm5uAgyytGhSEaTH9mdbRX8Zzkg469"),
			(2, "15QNS6ZDTqd6DeHNdHihcrpoA1R824ituiPGffxRW2zfc8US"),
			(2, "12wpfG98gTNfF2HJUuhK6msHSe1ZKbKupKYRk1AiQttwnCFp"),
			(2, "12gWb1R61chD3PDtNLba2d3hytHuSp5WRGnZaj9LzArP7KVs"),
			(2, "1k4nm2ExvCjYj43YTk7wG6PAcKprdi5k2ayeGF5hJTadLrK"),
			(2, "13uuCeyenBHFGU3VNemLEY97WZVSXw9KXv8AJG6VEx32MBfg"),
			(2, "13C9eaMNJPpEqEKRsEJ9P4DKxkb7XWQzeugEfdJkGdVFQPYh"),
			(2, "14QLoJzkneWvzWcLKXnnr32h9q66Cp8KyWSQvwhfxqpbNRh5"),
			(2, "E5kQJ88kBAwzk5ZqG8FbE5dJJg41dRpeg5BTrcskFNagBx2"),
			(2, "14QRqymDrSbmxcBQnoPeWTfrwcoCkKCFYQfFZfyqNLmsfS4G"),
			(2, "16ksAUbRPzzAJJmZBR1nYwxsNVsUrkaFPvDS4htLtv9fiU4a"),
			(2, "12zTcLRLU9LjFSCP5WKqAQqgKCUUrFGd5EVBJ8Br8aoryjDW"),
			(3, "1hFmn2CuqXqxHgKDqqs2xRBpsPkiRXzJfcLbfDgsW7qgmpA"),
			(3, "5CXb4GNCFZU5WKxcduG6rLDoyQXX6EFUzEvYYbxvUHCVKz5S"),
			(3, "155UDCSFsenWzWYDroTcZ28ApxrX8w9qAzE8XLqhsYx8oHmU"),
			(3, "143faiSeYd3agjdp5PBbg4iuFioeRJyC12ZuGsXJRGetaBNU"),
			(3, "5F1AJAW5JaZ3DdyPu1Wntn1qiNmBEGGMdGbGpybfbcwigmyC"),
			(3, "12wnTQnY3qUTCofi792fTdf9HpQQ6jmyxrVdwyT6X2Jed4Yg"),
			(3, "1682no4EwAfKspFBS5H1Yp3iPsLZTvrPBzR94xuWkyFJ2N3i"),
			(3, "12WQMLv8itdKJ83R4y3qNGYCRrsU6bFjyd4ZvcE4tmJoSiPv"),
			(3, "14QVZVSc1T7Gi55EL25LJfN5xDuCkdRNwPda4ZUvwMvJBC2g"),
			(3, "14QhzKsxmrECK8EDpLzzep8tWcYMUPYfXGfmgqM8cVtYRJjJ"),
			(3, "5CRxWu1z86Z9o9LGjv2QNXkxKQzRe4e29YYzdnnsQA3qesZb"),
			(3, "1vYC9dziqQmAqoeduwbkaKMXx2FiwhpDhj7gDAPqM3t1Xyt"),
			(3, "14rGj5dq7eiGJoDQsBHQhQKVgW71X4WLjV3TmBsJLHBxNw5x"),
			(3, "1LboBQLsa1iTpGzZvXcSd5VF7jfUYf6MzPNoRy2HT9D9FNk"),
			(3, "16FjvFZfimuixGF3zuqhmxq8XefzFGh6a2hoBJHpCTW4hJFK"),
			(3, "13dLY5aynbYDCy1DxQYQGjd4yy5uSeVtA4pLdR7SrcxZyvT1"),
			(3, "12itHPhSz9YKzydpCBPSxewfAJYZuReRPTP5qSuD5LPSeA41"),
			(3, "12uMmwxjpbntEkvfHDgb9Kdpeo98LNsj7KRLKMMqq4BdG5Qz"),
			(3, "14dwjTET5jJyfBELo1t37gr2uR7LGESPpUhSfdWyH9SL9KyF"),
			(3, "16dUmCQM28nzqAAG96rtLUbL9ni5LyT1wRx4SZoS11qnG94f"),
			(3, "14wyzJLVcCvjXcBnB4DTYvtbobmri5fjAQQKwaRA3JeCxLos"),
			(3, "14mH2752oH6phG36st3Zc62vFJD1X2JSdhMoNBoxRPnRAHNJ"),
			(3, "1eh8ed7Mv4Bv22xTKpfYKJdLCHr5oZwPH24eZmBB8Pzw9AB"),
			(3, "15MNuKZdQ65waaiRSmdzv2URNiHwprq8WTvYqbZDfpm8W4x6"),
			(3, "1HomBzZwkpQjhm3e9hQJ2AVFPmV4Ppon9ycCo91bh88xmGn"),
			(3, "5CLyeaugtNX8E24cFMRJZanhV1EZfTHwkt2MgFmcez9iuEe4"),
			(3, "123jNGxHk9ZV7oVVhFWFtMghNpmnmmTWxSpNxf8TTKzmCSQ2"),
			(3, "131cQySF87ceHM9F7JZb6rRfaGzGvX42oJKty6deyCKc5TRh"),
			(3, "143hSsxzuTfEsmbRBgdP1uc78nkgFN8z77ikT598bGs5ueMA"),
			(3, "1jPw3Qo72Ahn7Ynfg8kmYNLEPvHWHhPfPNgpJfp5bkLZdrF"),
			(3, "15wb37SKZ49jaTTtgz8312jKcWb1EwRTX3sDkJ1cs4RB97Rh"),
			(3, "13RBN6UF43sxkxUrd2H4QSJccvLNGr6HY4v3mN2WtW59WaNk"),
			(4, "135p4GMyXz4gGT4969k9kP1wR4KfURZrVoFZt2CDCWxxxpCm"),
			(4, "15rYBV5YwGmhzee5PWqrnQtb2HhwWP2rK2f4cLMhFfcNdPZL"),
			(4, "11Asf5YWRX1HGtTh1GcbtuaeYgbAJUWtSHCHaY23zgEeiit"),
			(4, "13QdCrHmSk2AnNgaYLAK8Mq9fdyvrouckBXTw5ccfJK8kkCS"),
		];

		for (index, val) in ambassador_ss58.iter().enumerate() {
			let account: AccountId32 = <AccountId as Ss58Codec>::from_string(val.1).unwrap();
			let account32: [u8; 32] = account.clone().into();
			assert_eq!(addresses[index].0, ambassador_ss58[index].0, "ranks must be equal.");
			assert_eq!(addresses[index].1, account32, "accounts must be equal.");
		}
	}

	#[test]
	fn test_add_accounts() {
		use super::add_accounts::InitialMemberSetup;
		use pallet_ranked_collective::{IdToIndex, IndexToId, MemberCount, MemberRecord, Members};

		let t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext.execute_with(|| {
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(0), 0);
			InitialMemberSetup::<Runtime, Ambassador>::on_runtime_upgrade();
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(0), 38);
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(1), 22);
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(2), 30);
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(3), 32);
			assert_eq!(MemberCount::<Runtime, Ambassador>::get(4), 4);
			for (rank, account_id32) in Addresses::get() {
				let who = <Runtime as frame_system::Config>::AccountId::from(account_id32);
				assert!(IdToIndex::<Runtime, Ambassador>::get(rank, &who).is_some());
				let index = IdToIndex::<Runtime, Ambassador>::get(rank, &who).unwrap();
				assert_eq!(IndexToId::<Runtime, Ambassador>::get(rank, index).unwrap(), who);
				assert_eq!(
					Members::<Runtime, Ambassador>::get(&who).unwrap(),
					MemberRecord::new(rank)
				);
			}
		});
	}
}
