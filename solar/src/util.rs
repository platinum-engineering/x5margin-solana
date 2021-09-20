use std::{borrow::Borrow, mem::size_of};

use chrono::{DateTime, TimeZone, Utc};
use fixed::{traits::ToFixed, types::U64F64};

use solana_api_types::{
    sysvar::{clock::Clock, rent::Rent, Sysvar},
    Pubkey,
};

use crate::{log::Loggable, math::Checked, mem::memcmp, qlog, time::SolTimestamp};

#[macro_export]
macro_rules! bytecode_marker {
    ($name:ident) => {{
        #[inline(never)]
        fn $name() {
            solana_program::msg!(stringify!($name));
        }
        $name();
    }};
}

pub trait ToPubkey {
    fn to_pubkey(&self) -> Pubkey;
}

pub trait AsPubkey {
    fn as_pubkey(&self) -> &Pubkey;
}

impl<T: AsPubkey> ToPubkey for T {
    fn to_pubkey(&self) -> Pubkey {
        *self.as_pubkey()
    }
}

impl<T: Borrow<Pubkey>> AsPubkey for T {
    fn as_pubkey(&self) -> &Pubkey {
        self.borrow()
    }
}

#[cfg_attr(target_arch = "bpf", inline(never))]
pub fn pubkey_eq<A: AsPubkey, B: AsPubkey>(a: A, b: B) -> bool {
    unsafe {
        memcmp(
            a.as_pubkey().as_ref().as_ptr(),
            b.as_pubkey().as_ref().as_ptr(),
            32,
        ) == 0
    }
}

pub trait ResultExt<T, E> {
    fn bpf_unwrap(self) -> T;
    fn bpf_expect(self, message: &'static str) -> T;
    fn bpf_context(self, context: &'static str) -> Self;
}

impl<T, E: Loggable> ResultExt<T, E> for Result<T, E> {
    #[track_caller]
    fn bpf_unwrap(self) -> T {
        match self {
            Ok(v) => v,
            Err(error) => {
                let location = std::panic::Location::caller();
                qlog!(
                    location.file(),
                    ":",
                    location.line(),
                    ": unwrap on err value: ",
                    error
                );
                panic!("unwrap on err value");
            }
        }
    }

    #[track_caller]
    fn bpf_expect(self, message: &'static str) -> T {
        match self {
            Ok(v) => v,
            Err(error) => {
                let location = std::panic::Location::caller();
                qlog!(
                    location.file(),
                    ":",
                    location.line(),
                    ": expect on err value `",
                    message,
                    "` ",
                    error
                );
                panic!("expect on err value");
            }
        }
    }

    #[track_caller]
    fn bpf_context(self, context: &'static str) -> Self {
        if self.is_err() {
            let location = std::panic::Location::caller();
            qlog!(
                location.file(),
                ":",
                location.line(),
                ": error occured `",
                context,
                "`"
            );
        }

        self
    }
}

impl<T> ResultExt<T, ()> for Option<T> {
    #[track_caller]
    fn bpf_unwrap(self) -> T {
        match self {
            Some(v) => v,
            None => {
                let location = std::panic::Location::caller();
                qlog!(location.file(), ":", location.line(), ": unwrap on `None`");
                panic!("unwrap on `None`");
            }
        }
    }

    #[track_caller]
    fn bpf_expect(self, message: &'static str) -> T {
        match self {
            Some(v) => v,
            None => {
                let location = std::panic::Location::caller();
                qlog!(
                    location.file(),
                    ":",
                    location.line(),
                    ": expect on `None` `",
                    message,
                    "` "
                );
                panic!("expect on `None`");
            }
        }
    }

    #[track_caller]
    fn bpf_context(self, context: &'static str) -> Self {
        if self.is_none() {
            let location = std::panic::Location::caller();
            qlog!(
                location.file(),
                ":",
                location.line(),
                ": error occured `",
                context,
                "`"
            );
        }

        self
    }
}

#[cfg_attr(target_arch = "bpf", inline(never))]
pub fn is_zeroed(slice: &[u8]) -> bool {
    if cfg!(target_arch = "bpf") {
        const ALIGNMENT: usize = size_of::<u64>();
        const BATCH_SIZE: usize = 64;

        unsafe {
            let mut acc: u64 = 0;
            let start_misalign = slice.as_ptr() as usize % ALIGNMENT;
            let mut ptr = slice.as_ptr();
            let end_ptr = slice.as_ptr().add(slice.len());

            if start_misalign > 0 {
                for _ in 0..(ALIGNMENT - start_misalign) {
                    acc |= ptr.read() as u64;
                    ptr = ptr.add(1);
                }
            }

            // compare using 64-bit operations, takes 8 times less opcodes than if we were using u8's
            while end_ptr as usize - ptr as usize >= BATCH_SIZE {
                let aligned_ptr = ptr.cast::<u64>();

                // loop will be unrolled by optimizer, less overhead for branching
                for i in 0..BATCH_SIZE / 8 {
                    acc |= aligned_ptr.add(i).read();
                }

                ptr = ptr.add(BATCH_SIZE);
            }

            while end_ptr as usize - ptr as usize > 0 {
                acc |= ptr.read() as u64;
                ptr = ptr.add(1);
            }

            acc == 0
        }
    } else {
        for b in slice {
            if *b != 0 {
                return false;
            }
        }

        true
    }
}

pub fn timestamp_now() -> Checked<i64> {
    Clock::get().bpf_unwrap().unix_timestamp.into()
}

pub fn sol_timestamp_now() -> SolTimestamp {
    Clock::get().bpf_unwrap().unix_timestamp.into()
}

pub fn datetime_now() -> DateTime<Utc> {
    Utc.timestamp(timestamp_now().value(), 0)
}

pub fn minimum_balance(size: u64) -> u64 {
    pub const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;
    let rent = Rent::default();
    let exemption_threshold = rent.exemption_threshold.to_fixed::<U64F64>();
    let per_year_cost =
        ((ACCOUNT_STORAGE_OVERHEAD + size) * rent.lamports_per_byte_year).to_fixed::<U64F64>();
    let minimum_balance: u64 = (exemption_threshold * per_year_cost).to_num::<u64>();

    minimum_balance
}

pub fn is_rent_exempt_fixed_arithmetic(rent: &Rent, lamports: u64, size: u64) -> bool {
    pub const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;
    let exemption_threshold = rent.exemption_threshold.to_fixed::<U64F64>();
    let per_year_cost =
        ((ACCOUNT_STORAGE_OVERHEAD + size) * rent.lamports_per_byte_year).to_fixed::<U64F64>();
    let minimum_balance: u64 = (exemption_threshold * per_year_cost).to_num::<u64>();

    lamports >= minimum_balance
}
