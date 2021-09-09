use std::marker::PhantomData;

use solana_api_types::{sysvar::rent::Rent, sysvar::Sysvar, Pubkey};
use solar::{
    account::{AccountBackend, AccountFields, AccountFieldsMut, Environment},
    reinterpret::{reinterpret_mut_unchecked, reinterpret_unchecked},
    util::{is_rent_exempt_fixed_arithmetic, minimum_balance, ResultExt},
};

use crate::error::Error;

pub const HEADER_RESERVED: usize = 96;

#[macro_export]
macro_rules! impl_entity_simple_deref {
    ($entity:ident, $target:ident) => {
        impl<E> std::ops::Deref for Entity<E, $entity>
        where
            E: solar::account::AccountBackend,
        {
            type Target = $target;

            #[inline]
            fn deref(&self) -> &Self::Target {
                unsafe { solar::reinterpret::reinterpret_unchecked(self.body()) }
            }
        }

        impl<E> std::ops::DerefMut for Entity<E, $entity>
        where
            E: solar::account::AccountBackend,
            E::Impl: solar::account::AccountFieldsMut,
        {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { solar::reinterpret::reinterpret_mut_unchecked(self.body_mut()) }
            }
        }
    };
}

pub trait AccountType {
    const KIND: EntityKind;

    fn is_valid_size(size: usize) -> bool;
    fn default_size() -> usize;

    fn default_lamports() -> u64 {
        minimum_balance(Self::default_size() as u64)
    }
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    None = 0x00,
    Root = 0x01,
    StakerRegistry = 0x02,
    RequestQueue = 0x03,

    Locker = 0x10,
    Vesting = 0x11,
}

#[repr(C)]
pub struct EntityHeader {
    pub root: Pubkey,

    pub id: EntityId,
    pub parent_id: EntityId,
    pub kind: EntityKind,
}

#[derive(Debug)]
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
    pub(crate) fn raw_any(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        let size = account.data().len();

        if size < HEADER_RESERVED || !T::is_valid_size(size - HEADER_RESERVED) {
            return Err(Error::InvalidData);
        }

        // require that account data is aligned on a 16-byte boundary
        // this is mostly important for offchain purposes
        if (account.data().as_ptr()) as usize % 16 != 0 {
            return Err(Error::InvalidAlignment);
        }

        let entity = Self {
            account,
            _phantom: Default::default(),
        };

        if entity.account.owner() != program_id {
            return Err(Error::InvalidOwner);
        }

        // require all entities to be rent-exempt to be valid
        if B::Env::supports_syscalls() && !entity.is_rent_exempt(&Rent::get().bpf_unwrap()) {
            return Err(Error::NotRentExempt);
        };

        Ok(entity)
    }

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
    pub(crate) fn raw_initialized(program_id: &Pubkey, account: B) -> Result<Self, Error> {
        let entity = Self::raw_any(program_id, account)?;

        if entity.header().kind != T::KIND {
            return Err(Error::InvalidKind);
        }

        Ok(entity)
    }

    pub fn account(&self) -> &B {
        &self.account
    }

    pub fn account_mut(&mut self) -> &mut B
    where
        B: AccountFieldsMut,
    {
        &mut self.account
    }

    pub fn header(&self) -> &EntityHeader {
        let data = &self.account.data()[..HEADER_RESERVED];
        unsafe { reinterpret_unchecked(data) }
    }

    pub fn header_mut(&mut self) -> &mut EntityHeader
    where
        B::Impl: AccountFieldsMut,
    {
        let data = &mut self.account.data_mut()[..HEADER_RESERVED];
        unsafe { reinterpret_mut_unchecked(data) }
    }

    pub(crate) fn body(&self) -> &[u8] {
        &self.account.data()[HEADER_RESERVED..]
    }

    pub(crate) fn body_mut(&mut self) -> &mut [u8]
    where
        B::Impl: AccountFieldsMut,
    {
        &mut self.account.data_mut()[HEADER_RESERVED..]
    }

    pub fn id(&self) -> EntityId {
        self.header().id
    }

    pub fn root(&self) -> &Pubkey {
        &self.header().root
    }

    pub fn parent_id(&self) -> EntityId {
        self.header().parent_id
    }

    pub fn is_parent<U: AccountType>(&self, other: &Entity<B, U>) -> bool {
        self.root() == other.root() && self.id() == other.parent_id()
    }

    pub fn is_child<U: AccountType>(&self, other: &Entity<B, U>) -> bool {
        self.root() == other.root() && self.parent_id() == other.id()
    }

    #[inline(never)]
    pub fn is_rent_exempt(&self, rent: &Rent) -> bool {
        is_rent_exempt_fixed_arithmetic(
            rent,
            self.account().lamports(),
            self.account().data().len() as u64,
        )
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
