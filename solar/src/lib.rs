#![cfg_attr(target_arch = "bpf", feature(test))]
#![cfg_attr(not(target_arch = "bpf"), feature(bench_black_box))]
#![allow(stable_features)]
#![feature(min_const_generics)]
#![feature(maybe_uninit_ref)]
#![feature(maybe_uninit_uninit_array)]
#![feature(slice_as_chunks)]

#[macro_use]
extern crate strum;

pub mod account;
pub mod authority;
pub mod collections;
pub mod entity;
pub mod error;
pub mod input;
pub mod log;
pub mod math;
pub mod mem;
pub mod reinterpret;
pub mod spl;
pub mod time;
pub mod util;

#[cfg(feature = "onchain")]
pub mod entrypoint;
#[cfg(feature = "onchain")]
pub mod invoke;

pub mod prelude {
    pub use crate::account::AccountBackend;
}

#[cfg(feature = "offchain")]
pub mod offchain {
    pub mod client;
}
