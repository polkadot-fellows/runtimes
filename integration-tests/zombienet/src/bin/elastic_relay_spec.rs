//! Wrapper used as zombienet's `chain_spec_command` for the relay chain in elastic
//! scaling integration tests.
//!
//! Polkadot relay has no `pallet_sudo`, so we cannot call `Coretime::assign_core` at
//! runtime. 
//! 
//! This wrapper instead pre-injects extra entries for `para_id` into `paras.paras`,
//! so that the relay runtime's `paras::genesis_build` calls
//! `AssignCoretime::assign_coretime` once per entry — that writes both
//! `CoreDescriptors` and `CoreSchedules`, which is the same path the live
//! `Coretime::assign_core` extrinsic takes.
//!
//! Combined with the entry zombienet itself appends later, this allocates cores 0,
//! 1 and 2 to the parachain at genesis.
//!
//! Usage (zombienet template):
//!   `elastic-relay-spec <chainName> --inject-para-id <para_id> [--extra-paras N]`

use std::{env, process};

const DUMMY_HEX: &str = "0x00";

fn main() {
	let mut args = env::args().skip(1);
	let chain = args.next().unwrap_or_else(|| die("missing chain name argument"));
	let mut inject_para_id: Option<u32> = None;
	let mut extra_paras: u32 = 2;
	while let Some(flag) = args.next() {
		match flag.as_str() {
			"--inject-para-id" => {
				let v = args.next().unwrap_or_else(|| die("--inject-para-id needs a value"));
				inject_para_id = Some(v.parse().unwrap_or_else(|e| die(&format!("bad para id: {e}"))));
			},
			"--extra-paras" => {
				let v = args.next().unwrap_or_else(|| die("--extra-paras needs a value"));
				extra_paras = v.parse().unwrap_or_else(|e| die(&format!("bad extra-paras: {e}")));
			},
			other => die(&format!("unknown flag: {other}")),
		}
	}

	let output = process::Command::new("chain-spec-generator")
		.arg(&chain)
		.output()
		.unwrap_or_else(|e| die(&format!("running chain-spec-generator failed: {e}")));
	if !output.status.success() {
		die(&format!(
			"chain-spec-generator returned {}: {}",
			output.status,
			String::from_utf8_lossy(&output.stderr)
		));
	}

	let mut spec: serde_json::Value = serde_json::from_slice(&output.stdout)
		.unwrap_or_else(|e| die(&format!("parsing chain-spec-generator output: {e}")));

	if let Some(para_id) = inject_para_id {
		let patch = spec
			.pointer_mut("/genesis/runtimeGenesis/patch")
			.unwrap_or_else(|| die("no /genesis/runtimeGenesis/patch in chain spec"));
		let patch_obj = patch.as_object_mut().unwrap_or_else(|| die("patch is not an object"));
		let paras = patch_obj
			.entry("paras".to_string())
			.or_insert_with(|| serde_json::json!({ "paras": [] }))
			.as_object_mut()
			.unwrap_or_else(|| die("paras is not an object"));
		let paras_vec = paras
			.entry("paras".to_string())
			.or_insert_with(|| serde_json::json!([]))
			.as_array_mut()
			.unwrap_or_else(|| die("paras.paras is not an array"));
		// Each extra entry triggers another AssignCoretime::assign_coretime call when
		// `paras::genesis_build` runs, which is what populates both `CoreDescriptors`
		// and `CoreSchedules` correctly.
		for _ in 0..extra_paras {
			paras_vec.push(serde_json::json!([para_id, [DUMMY_HEX, DUMMY_HEX, true]]));
		}
	}

	let serialized = serde_json::to_vec_pretty(&spec)
		.unwrap_or_else(|e| die(&format!("serializing patched spec: {e}")));
	use std::io::Write as _;
	std::io::stdout()
		.write_all(&serialized)
		.unwrap_or_else(|e| die(&format!("writing stdout: {e}")));
}

fn die(msg: &str) -> ! {
	eprintln!("elastic-relay-spec: {msg}");
	process::exit(1);
}
