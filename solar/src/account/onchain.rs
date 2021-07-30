use std::{
    fmt::Debug,
    mem::{align_of, size_of},
    slice::{from_raw_parts, from_raw_parts_mut},
};

use solana_program::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::{log::Loggable, util::AsPubkey};

use super::{AccountBackend, AccountFields, AccountFieldsMut};

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
    fn key(&self) -> &Pubkey {
        unsafe { &*self.key }
    }

    fn owner(&self) -> &Pubkey {
        unsafe { &*self.owner }
    }

    fn is_signer(&self) -> bool {
        self.is_signer
    }

    fn is_writable(&self) -> bool {
        self.is_writable
    }

    fn is_executable(&self) -> bool {
        self.is_executable
    }

    fn lamports(&self) -> u64 {
        unsafe { *self.lamports }
    }

    fn rent_epoch(&self) -> u64 {
        self.rent_epoch
    }

    fn data(&self) -> &[u8] {
        unsafe { from_raw_parts(self.data, self.data_len) }
    }
}

impl AccountFieldsMut for Account {
    fn set_lamports(&mut self, value: u64) {
        unsafe { *self.lamports = value }
    }

    fn data_mut(&mut self) -> &mut [u8] {
        unsafe { from_raw_parts_mut(self.data, self.data_len) }
    }
}

impl<'a> AccountBackend for &'a mut Account {
    type Impl = Account;

    fn backend(&self) -> &Self::Impl {
        self
    }

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
    fn as_account_ref(&self) -> &Account {
        self
    }

    fn as_account_mut(&mut self) -> &mut Account {
        self
    }
}

impl AsPubkey for Account {
    fn as_pubkey(&self) -> &Pubkey {
        self.key()
    }
}

impl<T> AsAccount for T
where
    T: AccountBackend<Impl = Account>,
{
    fn as_account_ref(&self) -> &Account {
        todo!()
    }

    fn as_account_mut(&mut self) -> &mut Account {
        todo!()
    }
}
