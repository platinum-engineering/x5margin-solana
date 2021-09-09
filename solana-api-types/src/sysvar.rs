use crate::{program::ProgramError, Pubkey};

pub trait SysvarId {
    fn id() -> Pubkey;

    fn check_id(pubkey: &Pubkey) -> bool;
}

#[macro_export]
macro_rules! declare_sysvar_id(
    ($name:expr, $type:ty) => (
        pub const ID: &$crate::Pubkey = &$crate::Pubkey::new(solar_macros::parse_base58!($name));

        impl $crate::sysvar::SysvarId for $type {
            fn id() -> $crate::pubkey::Pubkey {
                *ID
            }

            fn check_id(pubkey: &$crate::pubkey::Pubkey) -> bool {
                *pubkey == *ID
            }
        }
    )
);

// Sysvar utilities
pub trait Sysvar:
    SysvarId + Default + Sized + serde::Serialize + serde::de::DeserializeOwned
{
    fn size_of() -> usize {
        bincode::serialized_size(&Self::default()).unwrap() as usize
    }
    fn get() -> Result<Self, ProgramError> {
        Err(ProgramError::UnsupportedSysvar)
    }
}

#[macro_export]
macro_rules! impl_sysvar_get {
    ($sysvar_struct:ident, $sysvar_mod:ident, $syscall_name:ident) => {
        #[cfg(target_arch = "bpf")]
        fn get() -> Result<Self, $crate::program::ProgramError> {
            let mut var = Self::default();
            let var_addr = &mut var as *mut _ as *mut u8;

            let result = unsafe {
                extern "C" {
                    fn $syscall_name(var_addr: *mut u8) -> u64;
                }
                $syscall_name(var_addr)
            };

            match result {
                $crate::entrypoint::SUCCESS => Ok(var),
                e => Err(e.into()),
            }
        }

        #[cfg(all(not(target_arch = "bpf"), feature = "runtime-test"))]
        fn get() -> Result<Self, $crate::program::ProgramError> {
            use crate::sdk_proxy::FromSdk;
            use solana_sdk::sysvar::Sysvar;

            solana_program::sysvar::$sysvar_mod::$sysvar_struct::get()
                .map(|s| $sysvar_struct::from_sdk(&s))
                .map_err(|err| crate::program::ProgramError::from_sdk(&err))
        }

        #[cfg(not(target_arch = "bpf"))]
        fn get() -> Result<Self, $crate::program::ProgramError> {
            let mut var = Self::default();
            let var_addr = &mut var as *mut _ as *mut u8;

            match $crate::syscalls::$syscall_name(var_addr) {
                $crate::entrypoint::SUCCESS => Ok(var),
                e => Err(e.into()),
            }
        }
    };
}

pub mod clock {
    use super::Sysvar;
    use serde::{Deserialize, Serialize};

    use crate::{Epoch, Slot, UnixTimestamp};

    crate::declare_sysvar_id!("SysvarC1ock11111111111111111111111111111111", Clock);

    /// Clock represents network time.  Members of Clock start from 0 upon
    ///  network boot.  The best way to map Clock to wallclock time is to use
    ///  current Slot, as Epochs vary in duration (they start short and grow
    ///  as the network progresses).
    ///
    #[repr(C)]
    #[derive(Serialize, Clone, Deserialize, Debug, Default, PartialEq)]
    pub struct Clock {
        /// the current network/bank Slot
        pub slot: Slot,
        /// the timestamp of the first Slot in this Epoch
        pub epoch_start_timestamp: UnixTimestamp,
        /// the bank Epoch
        pub epoch: Epoch,
        /// the future Epoch for which the leader schedule has
        ///  most recently been calculated
        pub leader_schedule_epoch: Epoch,
        /// originally computed from genesis creation time and network time
        /// in slots (drifty); corrected using validator timestamp oracle as of
        /// timestamp_correction and timestamp_bounding features
        pub unix_timestamp: UnixTimestamp,
    }

    impl Sysvar for Clock {
        crate::impl_sysvar_get!(Clock, clock, sol_get_clock_sysvar);
    }
}

pub mod rent {

    use crate::{impl_sysvar_get, sysvar::Sysvar};
    use serde::{Deserialize, Serialize};

    crate::declare_sysvar_id!("SysvarRent111111111111111111111111111111111", Rent);

    #[repr(C)]
    #[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
    pub struct Rent {
        /// Rental rate
        pub lamports_per_byte_year: u64,

        /// exemption threshold, in years
        pub exemption_threshold: f64,

        // What portion of collected rent are to be destroyed, percentage-wise
        pub burn_percent: u8,
    }

    pub const DEFAULT_LAMPORTS_PER_BYTE_YEAR: u64 = 1_000_000_000 / 100 * 365 / (1024 * 1024);
    pub const DEFAULT_EXEMPTION_THRESHOLD: f64 = 2.0;
    pub const DEFAULT_BURN_PERCENT: u8 = 50;

    impl Default for Rent {
        fn default() -> Self {
            Self {
                lamports_per_byte_year: DEFAULT_LAMPORTS_PER_BYTE_YEAR,
                exemption_threshold: DEFAULT_EXEMPTION_THRESHOLD,
                burn_percent: DEFAULT_BURN_PERCENT,
            }
        }
    }

    impl Sysvar for Rent {
        impl_sysvar_get!(Rent, rent, sol_get_rent_sysvar);
    }
}
