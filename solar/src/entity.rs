use std::marker::PhantomData;

use solana_api_types::{
    sysvar::{rent::Rent, Sysvar},
    Pubkey,
};

use crate::{
    account::{AccountFields, AccountFieldsMut, Environment},
    prelude::AccountBackend,
    reinterpret::{reinterpret_mut_unchecked, reinterpret_unchecked},
    util::{is_rent_exempt_fixed_arithmetic, minimum_balance, ResultExt},
};

pub trait EntityHeader {
    type Discriminant: Eq;

    fn discriminant(&self) -> Self::Discriminant;
}

impl EntityHeader for () {
    type Discriminant = ();

    fn discriminant(&self) -> Self::Discriminant {}
}

pub trait EntitySchema {
    const HEADER_RESERVED: usize;

    type Header: EntityHeader;
}

pub trait AccountType {
    type Schema: EntitySchema;
    const KIND: <<Self::Schema as EntitySchema>::Header as EntityHeader>::Discriminant;

    fn is_valid_size(size: usize) -> bool;
    fn default_size() -> usize;

    fn default_lamports() -> u64 {
        minimum_balance(Self::default_size() as u64)
    }
}

#[derive(Debug)]
pub struct EntityBase<B: AccountBackend, T: AccountType> {
    pub account: B,
    _phantom: PhantomData<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityError {
    InvalidData,
    InvalidAlignment,
    InvalidOwner,
    NotRentExempt,
}

impl<B, T, S, H> EntityBase<B, T>
where
    B: AccountBackend,
    T: AccountType<Schema = S>,
    S: EntitySchema<Header = H>,
{
    pub fn raw_any(program_id: &Pubkey, account: B) -> Result<Self, EntityError> {
        let size = account.data().len();

        if size < S::HEADER_RESERVED || !T::is_valid_size(size - S::HEADER_RESERVED) {
            return Err(EntityError::InvalidData);
        }

        // require that account data is aligned on a 16-byte boundary
        // this is mostly important for offchain purposes
        if (account.data().as_ptr()) as usize % 16 != 0 {
            return Err(EntityError::InvalidAlignment);
        }

        let entity = Self {
            account,
            _phantom: Default::default(),
        };

        if entity.account.owner() != program_id {
            return Err(EntityError::InvalidOwner);
        }

        // require all entities to be rent-exempt to be valid
        if B::Env::supports_syscalls() && !entity.is_rent_exempt(&Rent::get().bpf_unwrap()) {
            return Err(EntityError::NotRentExempt);
        };

        Ok(entity)
    }

    pub fn header(&self) -> &H {
        let data = &self.account.data()[..S::HEADER_RESERVED];
        unsafe { reinterpret_unchecked(data) }
    }

    pub fn header_mut(&mut self) -> &mut H
    where
        B::Impl: AccountFieldsMut,
    {
        let data = &mut self.account.data_mut()[..S::HEADER_RESERVED];
        unsafe { reinterpret_mut_unchecked(data) }
    }

    pub fn body(&self) -> &[u8] {
        &self.account.data()[S::HEADER_RESERVED..]
    }

    pub fn body_mut(&mut self) -> &mut [u8]
    where
        B::Impl: AccountFieldsMut,
    {
        &mut self.account.data_mut()[S::HEADER_RESERVED..]
    }

    #[inline(never)]
    pub fn is_rent_exempt(&self, rent: &Rent) -> bool {
        is_rent_exempt_fixed_arithmetic(
            rent,
            self.account.lamports(),
            self.account.data().len() as u64,
        )
    }
}
