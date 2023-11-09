// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(dead_code)] // orchestra events are not used
#![allow(clippy::all)]

//! A minimal demo to be used with cargo expand.

use orchestra::{self as orchestra, Spawner, *};
mod misc;

pub use self::misc::*;

#[orchestra(signal=SigSigSig, event=EvX, error=Yikes, gen=AllMessages)]
struct Dig {
	#[subsystem(consumes: Plinko)]
	goblin_tower: GoblinTower,

	#[subsystem(sends: [Plinko])]
	goldmine: Goldmine,
}

use self::messages::*;

#[derive(Default)]
pub struct Fortified;

#[orchestra::subsystem(GoblinTower, error=Yikes)]
impl<Context> Fortified {
	fn start(self, mut ctx: Context) -> SpawnedSubsystem<Yikes> {
		SpawnedSubsystem {
			name: "GoblinTower",
			future: Box::pin(async move {
				while let Ok(FromOrchestra::Communication { msg: _ }) = ctx.recv().await {
					println!("Look a plinko!")
				}
				Ok(())
			}),
		}
	}
}

#[derive(Default)]
pub struct DragonsLair;

#[orchestra::subsystem(Goldmine, error=Yikes)]
impl<Context> DragonsLair {
	fn start(self, mut ctx: Context) -> SpawnedSubsystem<Yikes> {
		let mut sender = ctx.sender().clone();
		let future = Box::pin(async move {
			sender.send_message(Plinko).await;
			Ok(())
		});

		SpawnedSubsystem { name: "RedThorntail", future }
	}
}

async fn setup() {
	let builder = Dig::builder();

	let builder = builder.goblin_tower(Fortified::default());
	let builder = builder.goldmine(DragonsLair::default());
	let builder = builder.spawner(DummySpawner);
	let (orchestra, _handle) = builder.build().unwrap();

	let orchestra_fut = orchestra
		.running_subsystems
		.into_future()
		.timeout(std::time::Duration::from_millis(300))
		.fuse();

	futures::pin_mut!(orchestra_fut);

	orchestra_fut.await;
}

fn assert_t_impl_trait_send<T: Send>(_: &T) {}

fn main() {
	let x = setup();
	assert_t_impl_trait_send(&x);
	futures::executor::block_on(x);
}
