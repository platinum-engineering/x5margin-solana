use std::{
    marker::PhantomData,
    mem::size_of,
    ops::{Deref, DerefMut},
};

use solana_program::pubkey::Pubkey;
use solar::{
    account::{AccountBackend, AccountBackendMut},
    data::{reinterpret_mut_unchecked, reinterpret_unchecked},
};

use crate::error::Error;

pub const HEADER_RESERVED: usize = 96;
pub const FARM_ROOT_RESERVED: usize = 512;

pub trait AccountType {
    fn is_valid_size(size: usize) -> bool;
}

#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct EntityId {
    id: u64,
}

impl EntityId {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn value(&self) -> u64 {
        self.id
    }
}

#[repr(u8)]
pub enum EntityKind {
    None = 0,
    Root = 1,
    StakerRegistry = 2,
    RequestQueue = 3,
}

#[repr(C)]
pub struct EntityHeader {
    pub root: Pubkey,

    pub id: EntityId,
    pub parent_id: EntityId,
    pub kind: EntityKind,
}

pub struct Entity<B, T>
where
    B: AccountBackend,
    T: AccountType,
{
    account: B,
    _phantom: PhantomData<fn() -> T>,
}

impl<B, T> Entity<B, T>
where
    B: AccountBackend,
    T: AccountType,
{
    /// Raw constructor for protocol entities.
    ///
    /// # Safety
    ///
    /// This function will validate basic requirements of `T`, such as the size of account data
    /// and alignment, so it will not cause immediate UB if called with invalid inputs.
    ///
    /// *However*, consumers of this struct can rely on all instances of [`Entity`] to uphold
    /// invariants required by `T`, so callers are required to ensure that this account is actually
    /// an instance of account type `T` before returning it elsewhere.
    pub(crate) unsafe fn raw(account: B) -> Result<Self, Error> {
        let size = account.data().len();

        if size < HEADER_RESERVED || !T::is_valid_size(size - HEADER_RESERVED) {
            return Err(Error::InvalidData);
        }

        // require that account data is aligned on a 16-byte boundary
        // this is mostly important for offchain purposes
        if (account.data().as_ptr()) as usize % 16 != 0 {
            return Err(Error::InvalidAlignment);
        }

        Ok(Self {
            account,
            _phantom: Default::default(),
        })
    }

    pub fn account(&self) -> &B {
        &self.account
    }

    pub fn header(&self) -> &EntityHeader {
        let data = &self.account.data()[..HEADER_RESERVED];
        unsafe { reinterpret_unchecked(data) }
    }

    pub(crate) fn body(&self) -> &[u8] {
        &self.account.data()[HEADER_RESERVED..]
    }
}

impl<B, T> Entity<B, T>
where
    B: AccountBackendMut,
    T: AccountType,
{
    pub fn account_mut(&mut self) -> &mut B {
        &mut self.account
    }

    pub fn header_mut(&mut self) -> &mut EntityHeader {
        let data = &mut self.account.data_mut()[..HEADER_RESERVED];
        unsafe { reinterpret_mut_unchecked(data) }
    }

    pub(crate) fn body_mut(&mut self) -> &mut [u8] {
        &mut self.account.data_mut()[HEADER_RESERVED..]
    }
}

#[derive(Default)]
#[repr(C)]
pub struct EntityAllocator {
    counter: u64,
}

impl EntityAllocator {
    pub fn allocate_id(&mut self) -> EntityId {
        let id = self.counter;
        self.counter += 1;
        EntityId { id }
    }
}

#[repr(C)]
pub struct FarmState {
    pub administrator_authority: Pubkey,
    pub program_authority: Pubkey,
    pub active_stake_vault: Pubkey,
    pub inactive_stake_vault: Pubkey,
    pub reward_vault: Pubkey,

    pub allocator: EntityAllocator,
    pub active_stake: u64,
    pub inactive_stake: u64,
    pub program_authority_salt: u64,
    pub program_authority_nonce: u8,
}

const_assert!(size_of::<FarmState>() <= FARM_ROOT_RESERVED);

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
    fn is_valid_size(size: usize) -> bool {
        size >= FARM_ROOT_RESERVED
    }
}

impl<B: AccountBackend> Deref for Entity<B, Farm> {
    type Target = FarmState;

    fn deref(&self) -> &Self::Target {
        unsafe { reinterpret_unchecked(self.body()) }
    }
}

impl<B: AccountBackendMut> DerefMut for Entity<B, Farm> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { reinterpret_mut_unchecked(self.body_mut()) }
    }
}

impl<B: AccountBackendMut> Entity<B, Farm> {
    pub fn initialize(destination: B) -> Result<Self, Error> {
        let mut farm = unsafe { Entity::<_, Farm>::raw(destination)? };

        farm.header_mut().kind = EntityKind::Root;
        farm.header_mut().root = *farm.account().key();

        Ok(farm)
    }
}
