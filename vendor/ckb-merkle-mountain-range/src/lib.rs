#![cfg_attr(not(feature = "std"), no_std)]

mod error;
pub mod helper;
mod merge;
mod mmr;
mod mmr_store;
#[cfg(test)]
mod tests;
pub mod util;

pub use error::{Error, Result};
pub use helper::{leaf_index_to_mmr_size, leaf_index_to_pos};
pub use merge::Merge;
pub use mmr::{MerkleProof, MMR};
pub use mmr_store::MMRStore;

cfg_if::cfg_if! {
    if #[cfg(feature = "std")] {
        use std::borrow;
        use std::collections;
        use std::vec;
        use std::string;
    } else {
        extern crate alloc;
        use alloc::borrow;
        use alloc::collections;
        use alloc::vec;
        use alloc::string;
    }
}
