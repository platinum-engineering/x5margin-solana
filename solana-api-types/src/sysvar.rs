use crate::{account_info::AccountInfo, program::ProgramError, Pubkey};

pub trait SysvarId {
    fn id() -> Pubkey;

    fn check_id(pubkey: &Pubkey) -> bool;
}

#[macro_export]
macro_rules! declare_sysvar_id(
    ($name:expr, $type:ty) => (
        $crate::declare_id!($name);

        impl $crate::sysvar::SysvarId for $type {
            fn id() -> $crate::pubkey::Pubkey {
                *ID
            }

            fn check_id(pubkey: &$crate::pubkey::Pubkey) -> bool {
                *pubkey == *ID
            }
        }

        #[cfg(test)]
        #[test]
        fn test_sysvar_id() {
            if !$crate::sysvar::is_sysvar_id(&id()) {
                panic!("sysvar::is_sysvar_id() doesn't know about {}", $name);
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
    fn from_account_info(account_info: &AccountInfo) -> Result<Self, ProgramError> {
        if !Self::check_id(account_info.unsigned_key()) {
            return Err(ProgramError::InvalidArgument);
        }
        bincode::deserialize(&account_info.data.borrow()).map_err(|_| ProgramError::InvalidArgument)
    }
    fn to_account_info(&self, account_info: &mut AccountInfo) -> Option<()> {
        bincode::serialize_into(&mut account_info.data.borrow_mut()[..], self).ok()
    }
    fn get() -> Result<Self, ProgramError> {
        Err(ProgramError::UnsupportedSysvar)
    }
}

#[macro_export]
macro_rules! impl_sysvar_get {
    ($syscall_name:ident) => {
        fn get() -> Result<Self, ProgramError> {
            let mut var = Self::default();
            let var_addr = &mut var as *mut _ as *mut u8;

            #[cfg(target_arch = "bpf")]
            let result = unsafe {
                extern "C" {
                    fn $syscall_name(var_addr: *mut u8) -> u64;
                }
                $syscall_name(var_addr)
            };
            #[cfg(not(target_arch = "bpf"))]
            let result = crate::syscalls::$syscall_name(var_addr);

            match result {
                crate::entrypoint::SUCCESS => Ok(var),
                e => Err(e.into()),
            }
        }
    };
}
