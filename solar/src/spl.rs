use std::{mem::size_of, ops::Deref};

use solana_program::pubkey::Pubkey;

use crate::{
    account::AccountBackend,
    data::{is_valid_for_type, reinterpret_unchecked},
    util::pubkey_eq,
};

solana_program::declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

#[repr(packed)]
pub struct Mint {
    mint_authority_tag: u32,
    mint_authority: Pubkey,
    supply: u64,
    decimals: u8,
    is_initialized: bool,
    freeze_authority_tag: u32,
    freeze_authority: Pubkey,
}

impl Mint {
    pub fn mint_authority(&self) -> Option<&Pubkey> {
        if self.mint_authority_tag == 1 {
            Some(&self.mint_authority)
        } else {
            None
        }
    }

    pub fn freeze_authority(&self) -> Option<&Pubkey> {
        if self.freeze_authority_tag == 1 {
            Some(&self.freeze_authority)
        } else {
            None
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub fn supply(&self) -> u64 {
        self.supply
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

#[repr(packed)]
pub struct Wallet {
    mint: Pubkey,
    owner: Pubkey,
    amount: u64,
    delegate_tag: u32,
    delegate: Pubkey,
    state: u8,
    is_native_tag: u32,
    is_native: u64,
    delegated_amount: u64,
    close_authority_tag: u32,
    close_authority: Pubkey,
}

pub enum AccountState {
    Uninitialized,
    Initialized,
    Frozen,
    Invalid,
}

impl Wallet {
    pub fn mint(&self) -> &Pubkey {
        &self.mint
    }

    pub fn owner(&self) -> &Pubkey {
        &self.owner
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }

    pub fn delegate(&self) -> Option<&Pubkey> {
        if self.delegate_tag == 1 {
            Some(&self.delegate)
        } else {
            None
        }
    }

    pub fn state(&self) -> AccountState {
        match self.state {
            0 => AccountState::Uninitialized,
            1 => AccountState::Initialized,
            2 => AccountState::Frozen,
            _ => AccountState::Invalid,
        }
    }

    pub fn is_native(&self) -> bool {
        self.is_native_tag == 1
    }

    pub fn native_reserve(&self) -> Option<u64> {
        if self.is_native_tag == 1 {
            Some(self.is_native)
        } else {
            None
        }
    }

    pub fn delegated_amount(&self) -> u64 {
        self.delegated_amount
    }

    pub fn close_authority(&self) -> Option<&Pubkey> {
        if self.close_authority_tag == 1 {
            Some(&self.close_authority)
        } else {
            None
        }
    }
}

pub enum SplReadError {
    InvalidData,
    InvalidOwner,
    InvalidMint,
}

pub struct MintAccount<B> {
    account: B,
}

impl<B: AccountBackend> MintAccount<B> {
    pub fn any(account: B) -> Result<Self, SplReadError> {
        let data = account.data();

        if !pubkey_eq(account.owner(), &ID) {
            Err(SplReadError::InvalidOwner)
        } else if data.len() != size_of::<Mint>() || !is_valid_for_type::<Mint>(data) {
            Err(SplReadError::InvalidData)
        } else {
            Ok(Self { account })
        }
    }
}

impl<B: AccountBackend> Deref for MintAccount<B> {
    type Target = Mint;

    fn deref(&self) -> &Self::Target {
        unsafe { reinterpret_unchecked(self.account.data()) }
    }
}

pub struct WalletAccount<B> {
    account: B,
}

impl<B: AccountBackend> WalletAccount<B> {
    pub fn any(account: B) -> Result<Self, SplReadError> {
        let data = account.data();

        if !pubkey_eq(account.owner(), &ID) {
            Err(SplReadError::InvalidOwner)
        } else if data.len() != size_of::<Wallet>() || !is_valid_for_type::<Wallet>(data) {
            Err(SplReadError::InvalidData)
        } else {
            Ok(Self { account })
        }
    }
}

impl<B: AccountBackend> Deref for WalletAccount<B> {
    type Target = Wallet;

    fn deref(&self) -> &Self::Target {
        unsafe { reinterpret_unchecked(self.account.data()) }
    }
}

pub struct TokenProgram<B> {
    account: B,
}

impl<B: AccountBackend> TokenProgram<B> {
    pub fn load(account: B) -> Result<Self, SplReadError> {
        if !pubkey_eq(account.key(), &ID) {
            Err(SplReadError::InvalidOwner)
        } else {
            Ok(Self { account })
        }
    }

    pub fn account(&self) -> &B {
        &self.account
    }
}
