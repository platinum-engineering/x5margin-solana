use std::{
    fmt::Debug,
    mem::{align_of, size_of},
    slice::{from_raw_parts, from_raw_parts_mut},
};

use static_assertions::const_assert_eq;

use solana_api_types::Pubkey;

use crate::{log::Loggable, util::AsPubkey};

use super::{AccountBackend, AccountFields, AccountFieldsMut, Environment};

pub struct Onchain;

impl Environment for Onchain {
    fn supports_syscalls() -> bool {
        true
    }

    fn is_native() -> bool {
        !cfg!(target_arch = "bpf")
    }
}

#[repr(C)]
pub struct Account {
    pub(crate) key: *const Pubkey,
    pub(crate) lamports: *mut u64,
    pub(crate) data_len: usize,
    pub(crate) data: *mut u8,
    pub(crate) owner: *const Pubkey,
    pub(crate) rent_epoch: u64,
    pub(crate) is_signer: bool,
    pub(crate) is_writable: bool,
    pub(crate) is_executable: bool,
}

impl<'a> AccountFields for Account {
    #[inline]
    fn key(&self) -> &Pubkey {
        unsafe { &*self.key }
    }

    #[inline]
    fn owner(&self) -> &Pubkey {
        unsafe { &*self.owner }
    }

    #[inline]
    fn is_signer(&self) -> bool {
        self.is_signer
    }

    #[inline]
    fn is_writable(&self) -> bool {
        self.is_writable
    }

    #[inline]
    fn is_executable(&self) -> bool {
        self.is_executable
    }

    #[inline]
    fn lamports(&self) -> u64 {
        unsafe { *self.lamports }
    }

    #[inline]
    fn rent_epoch(&self) -> u64 {
        self.rent_epoch
    }

    #[inline]
    fn data(&self) -> &[u8] {
        unsafe { from_raw_parts(self.data, self.data_len) }
    }
}

impl AccountFieldsMut for Account {
    #[inline]
    fn set_lamports(&mut self, value: u64) {
        unsafe { *self.lamports = value }
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        unsafe { from_raw_parts_mut(self.data, self.data_len) }
    }
}

impl<'a> AccountBackend for &'a mut Account {
    type Impl = Account;
    type Env = Onchain;

    #[inline]
    fn backend(&self) -> &Self::Impl {
        self
    }

    #[inline]
    fn backend_mut(&mut self) -> &mut Self::Impl {
        self
    }
}

const_assert_eq!(size_of::<Account>(), 56);
const_assert_eq!(align_of::<Account>(), 8);

impl Account {
    pub(crate) unsafe fn copy(&self) -> Self {
        Self {
            key: self.key,
            lamports: self.lamports,
            data_len: self.data_len,
            data: self.data,
            owner: self.owner,
            rent_epoch: self.rent_epoch,
            is_signer: self.is_signer,
            is_writable: self.is_writable,
            is_executable: self.is_executable,
        }
    }
}

pub type AccountRef = &'static mut Account;

impl Debug for AccountRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccountRef {:p}", self)
    }
}

impl Loggable for AccountRef {
    fn push_to_logger<const S: usize>(&self, logger: &mut crate::log::Logger<S>) {
        logger.push_str("AccountRef (");
        logger.push_int(*self as *const _ as usize);
        logger.push_str(")");
    }
}

pub trait AsAccount {
    fn as_account_ref(&self) -> &Account;
    fn as_account_mut(&mut self) -> &mut Account;
}

impl AsAccount for Account {
    #[inline]
    fn as_account_ref(&self) -> &Account {
        self
    }

    #[inline]
    fn as_account_mut(&mut self) -> &mut Account {
        self
    }
}

impl AsPubkey for Account {
    #[inline]
    fn as_pubkey(&self) -> &Pubkey {
        self.key()
    }
}

impl<T> AsAccount for T
where
    T: AccountBackend<Impl = Account>,
{
    #[inline]
    fn as_account_ref(&self) -> &Account {
        self.backend()
    }

    #[inline]
    fn as_account_mut(&mut self) -> &mut Account {
        self.backend_mut()
    }
}
