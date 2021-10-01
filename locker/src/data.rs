use std::mem::size_of;

use solana_api_types::Pubkey;
use solar::{
    account::{pubkey::PubkeyAccount, AccountBackend, AccountFields, AccountFieldsMut},
    entity::{AccountType, EntityBase, EntitySchema},
    reinterpret::{reinterpret_mut_unchecked, reinterpret_unchecked},
    time::SolTimestamp,
    util::is_zeroed,
};

use crate::error::Error;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TokenLockState {
    pub withdraw_authority: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub program_authority: Pubkey,
    pub release_date: SolTimestamp,
}

#[derive(Debug)]
pub struct LockerEntitySchema;

impl EntitySchema for LockerEntitySchema {
    const HEADER_RESERVED: usize = 0;

    type Header = ();
}

#[derive(Debug)]
struct TokenLockEntity;

impl AccountType for TokenLockEntity {
    type Schema = LockerEntitySchema;
    const KIND: () = ();

    fn is_valid_size(size: usize) -> bool {
        Self::default_size() == size
    }

    fn default_size() -> usize {
        size_of::<TokenLockState>()
    }
}

#[derive(Debug)]
pub struct TokenLock<B: AccountBackend> {
    account: EntityBase<B, TokenLockEntity>,
}

impl<B: AccountBackend> TokenLock<B> {
    pub fn any(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        Ok(Self {
            account: EntityBase::<B, TokenLockEntity>::raw_any(program_id, account)?,
        })
    }

    pub fn blank(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        let lock = Self::any(program_id, account)?;

        if lock.is_blank() {
            Ok(lock)
        } else {
            Err(Error::InvalidAccount)
        }
    }

    pub fn initialized(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        let lock = Self::any(program_id, account)?;

        if !lock.is_blank() {
            Ok(lock)
        } else {
            Err(Error::InvalidAccount)
        }
    }

    pub fn account(&self) -> &B {
        &self.account.account
    }

    pub fn key(&self) -> &Pubkey {
        self.account.account.key()
    }

    pub fn is_blank(&self) -> bool {
        is_zeroed(self.account.body())
    }

    pub fn read(&self) -> &TokenLockState {
        unsafe { reinterpret_unchecked(self.account.body()) }
    }

    pub fn read_mut(&mut self) -> &mut TokenLockState
    where
        B::Impl: AccountFieldsMut,
    {
        unsafe { reinterpret_mut_unchecked(self.account.body_mut()) }
    }
}

impl From<Pubkey> for TokenLock<PubkeyAccount> {
    fn from(pubkey: Pubkey) -> Self {
        Self {
            account: pubkey.into(),
        }
    }
}
