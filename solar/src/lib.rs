// #![cfg_attr(target_arch = "bpf", feature(test))]
// #![cfg_attr(not(target_arch = "bpf"), feature(bench_black_box))]
// #![allow(stable_features)]
// #![feature(min_const_generics)]
// #![feature(maybe_uninit_ref)]

pub mod account;
pub mod collections;
pub mod data;
pub mod entrypoint;
pub mod input;
pub mod log;
pub mod mem;
pub mod spl;
pub mod util;

#[cfg(feature = "test")]
pub mod test;

pub mod prelude {
    pub use crate::account::AccountBackend;
    pub use crate::account::AccountBackendMut;
}
