// Copyright (C) Polkadot Fellows.
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

//! Integration-test harness for the AHM v2 migration.
//!
//! The Relay Chain and the Coretime chain are loaded from snapshots of live network state and
//! driven by hand: blocks are produced by calling the relevant hooks and DMP/UMP messages are
//! shuttled between the chains manually. There are no nodes and no networking involved.
#![cfg(test)]

pub mod mock;
pub mod tests;
