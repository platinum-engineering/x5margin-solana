use fixed::types::U64F64;
use parity_scale_codec::Decode;
use solana_api_types::{program::ProgramError, Instruction, Pubkey};
use solar::{
    input::{AccountSource, BpfProgramInput, ProgramInput},
    math::Checked,
    prelude::AccountBackend,
    time::SolTimestamp,
    util::ResultExt,
};

#[macro_use]
extern crate parity_scale_codec;

#[macro_use]
extern crate solar_macros;

pub type TokenAmount = Checked<u64>;
pub type TokenAmountF64 = Checked<U64F64>;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum Method {
    CreateLock {
        unlock_date: SolTimestamp,
        amount: TokenAmount,
    },
    ReLock,
    Withdraw,
    Increment,
    Split,
    ChangeOwner,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TokenLockState {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub program_authority: Pubkey,
    pub release_date: SolTimestamp,
}

pub struct TokenLock<B: AccountBackend> {
    account: B,
}

impl<B: AccountBackend> TokenLock<B> {
    /// Create a new locker.
    ///
    /// Account inputs:
    /// Locker (empty, uninitialized)
    /// SPL Token Wallet source
    /// Source Authority (signed)
    /// SPL Token Wallet vault (authority = program authority)
    /// Program Authority
    /// Owner (withdraw authority)
    pub fn create<S: AccountSource<B>>(
        input: S,
        unlock_date: SolTimestamp,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        todo!()
    }

    /// Returns 4 instructions:
    /// SystemProgram::CreateAccount (locker vault)
    /// SplToken::Initialize (locker vault)
    /// SystemProgram::Create (locker)
    /// Locker::Create (locker)
    pub fn create_instruction(
        locker: Pubkey,
        owner: Pubkey,
        source_wallet: Pubkey,
        source_mint: Pubkey,
        source_authority: Pubkey,
    ) -> [Instruction; 4] {
        todo!()
    }

    /// Relocks an existing locker with a new unlock date.
    ///
    /// Input accounts:
    /// Locker
    /// Locker Owner (signed)
    pub fn relock<S: AccountSource<B>>(
        input: S,
        unlock_date: SolTimestamp,
    ) -> Result<(), ProgramError> {
        todo!()
    }

    /// Withdraw funds from locker.
    /// Input accounts:
    /// Locker
    /// SPL Token Wallet vault
    /// SPL Token Wallet destination
    /// Program Authority
    /// Owner (signed)
    pub fn withdraw<S: AccountSource<B>>(
        input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        todo!()
    }

    /// Add funds to locker
    ///
    /// Input accounts:
    /// Locker
    /// SPL Token Wallet vault
    /// SPL Token Wallet source
    /// Source Authority
    pub fn increment<S: AccountSource<B>>(
        input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        todo!()
    }

    /// Split locker
    ///
    /// Input accounts:
    /// Source Locker
    /// New Locker
    /// Program Authority
    /// SPL Token Vault (Source Locker)
    /// SPL Token Vault (New Locker)
    pub fn split<S: AccountSource<B>>(input: S, amount: TokenAmount) -> Result<(), ProgramError> {
        todo!()
    }

    /// Change locker owner
    ///
    /// Input accounts:
    /// Locker
    /// Owner (signed)
    /// New Owner
    pub fn change_owner<S: AccountSource<B>>(
        input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        todo!()
    }
}

pub fn main(mut input: BpfProgramInput) -> Result<(), ProgramError> {
    let mut data = input.data();
    let method = Method::decode(&mut data)
        .ok()
        .bpf_expect("couldn't parse method");

    match method {
        Method::CreateLock {
            unlock_date,
            amount,
        } => TokenLock::create(input, unlock_date, amount).bpf_unwrap(),
        Method::ReLock => todo!(),
        Method::Withdraw => todo!(),
        Method::Increment => todo!(),
        Method::Split => todo!(),
        Method::ChangeOwner => todo!(),
    }

    Ok(())
}
