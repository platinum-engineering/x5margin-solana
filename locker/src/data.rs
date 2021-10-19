use std::mem::size_of;

use solana_api_types::Pubkey;
use solar::{
    account::{AccountBackend, AccountFields, AccountFieldsMut},
    entity::{AccountType, EntityBase, EntitySchema},
    reinterpret::{reinterpret_mut_unchecked, reinterpret_unchecked},
    time::SolTimestamp,
    util::is_zeroed,
};

use crate::error::Error;

#[repr(C)]
#[derive(Clone)]
#[cfg_attr(feature = "debug", derive(Debug))]
pub struct TokenLockState {
    pub withdraw_authority: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub program_authority: Pubkey,
    pub release_date: SolTimestamp,
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct LockerEntitySchema;

impl EntitySchema for LockerEntitySchema {
    const HEADER_RESERVED: usize = 0;

    type Header = ();
}

#[cfg_attr(feature = "debug", derive(Debug))]
pub struct TokenLockEntity;

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

#[cfg_attr(feature = "debug", derive(Debug))]
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

#[cfg(feature = "offchain")]
impl From<Pubkey> for TokenLock<solar::account::pubkey::PubkeyAccount> {
    fn from(pubkey: Pubkey) -> Self {
        Self {
            account: pubkey.into(),
        }
    }
}

#[cfg(feature = "offchain")]
pub fn find_locker_program_authority(
    program_id: &Pubkey,
    locker: &Pubkey,
    owner: &Pubkey,
    initial_nonce: u64,
) -> (Pubkey, u64) {
    let mut nonce = initial_nonce;
    loop {
        let authority = Pubkey::create_program_address(
            &[locker.as_ref(), owner.as_ref(), &nonce.to_le_bytes()],
            program_id,
        );

        if let Some(authority) = authority {
            return (authority, nonce);
        }

        nonce += 1;
    }
}
