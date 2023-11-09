extern crate libc;
extern crate lz4_sys;

pub mod liblz4;

mod decoder;
mod encoder;

pub mod block;

pub use crate::decoder::Decoder;
pub use crate::encoder::Encoder;
pub use crate::encoder::EncoderBuilder;
pub use crate::liblz4::version;
pub use crate::liblz4::BlockMode;
pub use crate::liblz4::BlockSize;
pub use crate::liblz4::ContentChecksum;

#[cfg(not(all(
    target_arch = "wasm32",
    not(any(target_env = "wasi", target_os = "wasi"))
)))]
use libc::{c_char, size_t};

#[cfg(all(
    target_arch = "wasm32",
    not(any(target_env = "wasi", target_os = "wasi"))
))]
use std::os::raw::c_char;

#[cfg(all(
    target_arch = "wasm32",
    not(any(target_env = "wasi", target_os = "wasi"))
))]
#[allow(non_camel_case_types)]
type size_t = usize;
