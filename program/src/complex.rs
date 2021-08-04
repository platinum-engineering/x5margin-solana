use std::{
    mem::size_of,
    ops::{Deref, DerefMut},
};

use solana_program::pubkey::Pubkey;
use solar::{
    account::{AccountFields, AccountFieldsMut},
    prelude::AccountBackend,
    reinterpret::{reinterpret_mut_unchecked, reinterpret_unchecked},
    spl::{MintAccount, WalletAccount},
};

use crate::{
    data::{AccountType, Entity, EntityAllocator, EntityId, EntityKind},
    error::Error,
};

#[repr(C)]
pub struct StakePoolState {
    pub administrator_authority: Pubkey,
    pub program_authority: Pubkey,
    pub stake_mint: Pubkey,
    pub active_stake_vault: Pubkey,
    pub inactive_stake_vault: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_vault: Pubkey,

    pub allocator: EntityAllocator,
    pub active_stake: u64,
    pub inactive_stake: u64,
    pub program_authority_salt: u64,
    pub program_authority_nonce: u8,
}

pub const STAKE_POOL_STATE_RESERVED: usize = 512;
const_assert!(size_of::<StakePoolState>() <= STAKE_POOL_STATE_RESERVED);

#[repr(C)]
pub struct Request {
    pub slot: u64,
    pub kind: RequestKind,
}

#[repr(C)]
pub enum RequestKind {
    AddStake { staker: Pubkey, amount: u64 },
    RemoveStake { staker: Pubkey, amount: u64 },
}

#[repr(C)]
pub struct Staker {
    pub authority: Pubkey,
    pub active_stake: u64,
    pub inactive_stake: u64,
    pub unclaimed_reward: u64,
}

pub struct Farm;
pub struct RequestQueue;
pub struct StakerRegistry;

impl AccountType for Farm {
    const KIND: EntityKind = EntityKind::Root;

    fn is_valid_size(size: usize) -> bool {
        size >= STAKE_POOL_STATE_RESERVED
    }

    fn default_size() -> usize {
        todo!()
    }
}

impl<B: AccountBackend> Deref for Entity<B, Farm> {
    type Target = StakePoolState;

    fn deref(&self) -> &Self::Target {
        unsafe { reinterpret_unchecked(self.body()) }
    }
}

impl<B: AccountBackend> DerefMut for Entity<B, Farm>
where
    B::Impl: AccountFieldsMut,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { reinterpret_mut_unchecked(self.body_mut()) }
    }
}

impl<B: AccountBackend> Entity<B, Farm> {
    pub fn load(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        let farm = Self::raw_initialized(program_id, account)?;

        let header = farm.header();

        if &header.root != farm.account().key()
            || header.parent_id != EntityId::new(0)
            || header.id != EntityId::new(0)
        {
            return Err(Error::InvalidData);
        }

        Ok(farm)
    }

    fn wallet<T: AccountBackend>(&self, account: T) -> Result<WalletAccount<T>, Error> {
        let wallet = WalletAccount::any(account)?;
        if wallet.authority() != &self.program_authority {
            return Err(Error::Validation);
        }

        Ok(wallet)
    }

    pub fn load_stake_vault<T: AccountBackend>(
        &self,
        account: T,
    ) -> Result<WalletAccount<T>, Error> {
        let wallet = self.wallet(account)?;
        if wallet.mint() != &self.stake_mint {
            return Err(Error::Validation);
        }

        Ok(wallet)
    }

    pub fn load_reward_vault<T: AccountBackend>(
        &self,
        account: T,
    ) -> Result<WalletAccount<T>, Error> {
        let wallet = self.wallet(account)?;
        if wallet.mint() != &self.reward_mint {
            return Err(Error::Validation);
        }

        Ok(wallet)
    }

    pub fn load_stake_mint<T: AccountBackend>(&self, account: T) -> Result<MintAccount<T>, Error> {
        let mint = MintAccount::any(account)?;
        if mint.key() != &self.stake_mint {
            return Err(Error::Validation);
        }

        Ok(mint)
    }

    pub fn load_reward_mint<T: AccountBackend>(&self, account: T) -> Result<MintAccount<T>, Error> {
        let mint = MintAccount::any(account)?;
        if mint.key() != &self.reward_mint {
            return Err(Error::Validation);
        }

        Ok(mint)
    }
}

impl<B: AccountBackend> Entity<B, Farm>
where
    B::Impl: AccountFieldsMut,
{
    pub fn initialize(program_id: &Pubkey, destination: B) -> Result<Self, Error> {
        let mut farm = Self::raw_initialized(program_id, destination)?;

        let key = *farm.account().key();
        let header = farm.header_mut();
        header.kind = EntityKind::Root;
        header.root = key;
        header.parent_id = EntityId::new(0);
        header.id = EntityId::new(0);

        Ok(farm)
    }
}
