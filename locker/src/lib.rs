use fixed::types::U64F64;
use parity_scale_codec::Decode;
use solana_api_types::{program::ProgramError, Instruction, Pubkey};
use solar::{
    input::{AccountSource, BpfProgramInput, ProgramInput},
    math::Checked,
    prelude::AccountBackend,
    time::SolTimestamp,
    util::{ResultExt, pubkey_eq, timestamp_now},
    spl::WalletAccount,
    qlog,
};

use crate::error::Error;

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

#[derive(Debug)]
pub struct CreateArgsAccounts<B: AccountBackend> {
    pub locker: B, //(empty, uninitialized)
    pub source_spl_token_wallet: B,
    pub source_authority: B, //(signed)
    pub spl_token_wallet_vault: WalletAccount<B>, //(authority = program authority)
    pub program_authority: B,
    pub owner_authority: B, //withdraw authority
}

impl<B: AccountBackend> CreateArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker,
            &source_spl_token_wallet,
            &source_authority,
            &spl_token_wallet_vault,
            &program_authority,
            &owner_authority,
        }

        Ok(Self {
            locker,
            source_spl_token_wallet,
            source_authority,
            spl_token_wallet_vault,
            program_authority,
            owner_authority,
        })
    }
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

        let CreateArgsAccounts {
            locker,
            source_spl_token_wallet,
            source_authority,
            spl_token_wallet_vault,
            program_authority,
            owner_authority,
        } = CreateArgsAccounts::from_program_input(input)?;

        let mut entity = Self::raw_any(input.program_id(), locker)?;

        entity.owner = *owner_authority.key();
        entity.mint = source_spl_token_wallet.mint();
        entity.vault = *spl_token_wallet_vault;
        entity.program_authority = *program_authority.key();
        entity.release_date = unlock_date;

        let expected_program_authority = Pubkey::create_program_address(
            &[
                entity.account().key().as_ref(),
                owner_authority.key().as_ref(),
            ],
            input.program_id(),
        )
        .bpf_expect("couldn't derive program authority");

        if !pubkey_eq(program_authority.key(), &expected_program_authority) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(spl_token_wallet_vault.authority(), &expected_program_authority) {
            qlog!("spl token wallet vault authority does not match program authority");
            return Err(Error::InvalidAuthority);
        }

        let now = timestamp_now();

        if entity.release_date <= now {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData);
        }

        Ok(())
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

        let mut instructions = vec![];
        todo!();

        return instructions;
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

#[cfg(feature = "onchain")]
#[cfg(test)]
mod test {

    #[tokio::test]
    async fn init_test() -> anyhow::Result<()> {

        let mut program_test = ProgramTest::default();
        let program_id = Pubkey::new_unique();

        program_test.add_program(
            "locker",
            program_id,
            Some(|a, b, c| {
                builtin_process_instruction(wrapped_entrypoint::<super::Program>, a, b, c)
            }),
        );

        let locker_key = Keypair::new();
        let locker_owner_key = Keypair::new();

        let mut salt: u64 = 0;
        let locker_program_authority = loop {
            let locker_program_authority = Pubkey::create_program_address(
                &[
                    locker_key.pubkey().as_ref(),
                    locker_owner_key.pubkey().as_ref(),
                ],
                &program_id,
            );

            match locker_program_authority {
                Some(s) => break s,
                None => {
                    salt += 1;
                }
            }
        };

        let (mut client, payer, hash) = program_test.start().await;

        todo!();

        Ok(())
    }
}
