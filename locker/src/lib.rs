use std::mem::size_of;

use data::HEADER_RESERVED;
use fixed::types::U64F64;
use parity_scale_codec::Decode;
use solana_api_types::{program::ProgramError, Instruction, Pubkey};
#[cfg(feature = "onchain")]
use solar::input::BpfProgramInput;
use solar::{
    account::{AccountFields, AccountFieldsMut},
    input::AccountSource,
    math::Checked,
    prelude::AccountBackend,
    qlog,
    spl::WalletAccount,
    time::SolTimestamp,
    util::{pubkey_eq, timestamp_now, ResultExt},
};

pub mod data;
pub mod error;

#[macro_use]
extern crate parity_scale_codec;

#[macro_use]
extern crate solar_macros;

use crate::{
    data::{AccountType, Entity, EntityKind},
    error::Error,
};

pub type TokenAmount = Checked<u64>;
pub type TokenAmountF64 = Checked<U64F64>;

pub type TokenLockEntity<B> = Entity<B, TokenLock>;

#[derive(Debug, PartialEq, Eq, Clone, Encode, Decode)]
pub enum Method {
    CreateLock {
        unlock_date: SolTimestamp,
        amount: TokenAmount,
    },
    ReLock {
        unlock_date: SolTimestamp,
    },
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

#[derive(Debug)]
pub struct TokenLock;

impl AccountType for TokenLock {
    const KIND: EntityKind = EntityKind::Locker;

    fn is_valid_size(size: usize) -> bool {
        size == size_of::<TokenLockState>()
    }

    fn default_size() -> usize {
        size_of::<TokenLockState>() + HEADER_RESERVED
    }
}

#[derive(Debug)]
pub struct CreateArgsAccounts<B: AccountBackend> {
    pub locker: B, //(empty, uninitialized)
    pub source_spl_token_wallet: B,
    pub source_authority: B,                      //(signed)
    pub spl_token_wallet_vault: WalletAccount<B>, //(authority = program authority)
    pub program_authority: B,
    pub owner_authority: B, //withdraw authority
}

impl<B: AccountBackend> CreateArgsAccounts<B> {
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
            spl_token_wallet_vault: WalletAccount::any(spl_token_wallet_vault)?,
            program_authority,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct ReLockArgsAccounts<B: AccountBackend> {
    pub locker: B,          //(empty, uninitialized)
    pub owner_authority: B, //withdraw authority
}

impl<B: AccountBackend> ReLockArgsAccounts<B> {
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker,
            &owner_authority,
        }

        Ok(Self {
            locker,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct WithdrawArgsAccounts<B: AccountBackend> {
    pub locker: B,
    pub spl_token_wallet_vault: WalletAccount<B>,
    pub destination_spl_token_wallet: B,
    pub program_authority: B,
    pub owner_authority: B,
}

impl<B: AccountBackend> WithdrawArgsAccounts<B> {
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker,
            &spl_token_wallet_vault,
            &destination_spl_token_wallet,
            &program_authority,
            &owner_authority,
        }

        Ok(Self {
            locker,
            spl_token_wallet_vault: WalletAccount::any(spl_token_wallet_vault)?,
            destination_spl_token_wallet,
            program_authority,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct IncrementArgsAccounts<B: AccountBackend> {
    pub locker: B,
    pub spl_token_wallet_vault: WalletAccount<B>,
    pub source_spl_token_wallet: B,
    pub source_authority: B,
}

impl<B: AccountBackend> IncrementArgsAccounts<B> {
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &locker,
            &spl_token_wallet_vault,
            &source_spl_token_wallet,
            &source_authority,
        }

        Ok(Self {
            locker,
            spl_token_wallet_vault: WalletAccount::any(spl_token_wallet_vault)?,
            source_spl_token_wallet,
            source_authority,
        })
    }
}

#[derive(Debug)]
pub struct SplitArgsAccounts<B: AccountBackend> {
    pub source_locker: B,
    pub new_locker: B, //(empty, uninitialized)
    pub source_spl_token_wallet_vault: WalletAccount<B>,
    pub new_spl_token_wallet_vault: WalletAccount<B>,
}

impl<B: AccountBackend> SplitArgsAccounts<B> {
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &source_locker,
            &mut new_locker,
            &source_spl_token_wallet_vault,
            &new_spl_token_wallet_vault,
        }

        Ok(Self {
            source_locker,
            new_locker,
            source_spl_token_wallet_vault: WalletAccount::any(source_spl_token_wallet_vault)?,
            new_spl_token_wallet_vault: WalletAccount::any(new_spl_token_wallet_vault)?,
        })
    }
}

#[derive(Debug)]
pub struct ChangeOwnerArgsAccounts<B: AccountBackend> {
    pub locker: B,
    pub source_owner_authority: B,
    pub new_owner_authority: B,
}

impl<B: AccountBackend> ChangeOwnerArgsAccounts<B> {
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &locker,
            &source_owner_authority,
            &new_owner_authority,
        }

        Ok(Self {
            locker,
            source_owner_authority,
            new_owner_authority,
        })
    }
}

impl<B> Entity<B, TokenLock>
where
    B: AccountBackend,
{
    /// Create a new locker.
    ///
    /// Account inputs:
    /// Locker (empty, uninitialized)
    /// SPL Token Wallet source
    /// Source Authority (signed)
    /// SPL Token Wallet vault (authority = program authority)
    /// Program Authority
    /// Owner (withdraw authority)
    pub fn create<S>(
        mut input: S,
        unlock_date: SolTimestamp,
        amount: TokenAmount,
    ) -> Result<(), ProgramError>
    where
        S: AccountSource<B>,
        B::Impl: AccountFieldsMut,
    {
        let CreateArgsAccounts {
            locker,
            source_spl_token_wallet,
            source_authority,
            spl_token_wallet_vault,
            program_authority,
            owner_authority,
        } = CreateArgsAccounts::from_program_input(&mut input)?;

        let mut entity = Self::raw_any(input.program_id(), locker)?;

        // entity.owner = *owner_authority.key();
        // entity.mint = source_spl_token_wallet.mint();
        // entity.vault = *spl_token_wallet_vault;
        // entity.program_authority = *program_authority.key();
        // entity.release_date = unlock_date;

        // let id = entity.allocator.allocate_id();
        let entity_key = *entity.account().key();
        let header = entity.header_mut();
        header.kind = EntityKind::Locker;
        // header.id = id;
        // header.parent_id = id;
        header.root = entity_key;

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
            return Err(Error::InvalidAuthority.into());
        }

        if !pubkey_eq(
            spl_token_wallet_vault.authority(),
            &expected_program_authority,
        ) {
            qlog!("spl token wallet vault authority does not match program authority");
            return Err(Error::InvalidAuthority.into());
        }

        let now = timestamp_now();

        // if entity.release_date <= now {
        //     qlog!("can`t initialize new locker with invalid unlock date");
        //     return Err(Error::InvalidData.into());
        // }

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
        let mut instructions = todo!();
    }

    /// Relocks an existing locker with a new unlock date.
    ///
    /// Input accounts:
    /// Locker
    /// Locker Owner (signed)
    pub fn relock<S: AccountSource<B>>(
        mut input: S,
        unlock_date: SolTimestamp,
    ) -> Result<(), ProgramError> {
        let ReLockArgsAccounts {
            locker,
            owner_authority,
        } = ReLockArgsAccounts::from_program_input(&mut input)?;

        if !pubkey_eq(locker.owner(), owner_authority.key()) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority.into());
        }

        /*
        if unlock_date <= locker.release_date {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData.into());
        }

        locker.release_date = unlock_date;
        */

        Ok(())
    }

    /// Withdraw funds from locker.
    /// Input accounts:
    /// Locker
    /// SPL Token Wallet vault
    /// SPL Token Wallet destination
    /// Program Authority
    /// Owner (signed)
    pub fn withdraw<S: AccountSource<B>>(
        mut input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        let WithdrawArgsAccounts {
            locker,
            spl_token_wallet_vault,
            destination_spl_token_wallet,
            program_authority,
            owner_authority,
        } = WithdrawArgsAccounts::from_program_input(&mut input)?;

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
        mut input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        let IncrementArgsAccounts {
            locker,
            spl_token_wallet_vault,
            source_spl_token_wallet,
            source_authority,
        } = IncrementArgsAccounts::from_program_input(&mut input)?;

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
    pub fn split<S: AccountSource<B>>(
        mut input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        let SplitArgsAccounts {
            source_locker,
            new_locker,
            source_spl_token_wallet_vault,
            new_spl_token_wallet_vault,
        } = SplitArgsAccounts::from_program_input(&mut input)?;

        todo!()
    }

    /// Change locker owner
    ///
    /// Input accounts:
    /// Locker
    /// Owner (signed)
    /// New Owner
    pub fn change_owner<S: AccountSource<B>>(
        mut input: S,
        amount: TokenAmount,
    ) -> Result<(), ProgramError> {
        let ChangeOwnerArgsAccounts {
            locker,
            source_owner_authority,
            new_owner_authority,
        } = ChangeOwnerArgsAccounts::from_program_input(&mut input)?;

        todo!()
    }
}

#[cfg(feature = "onchain")]
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
        Method::ReLock { unlock_date } => TokenLock::relock(input, unlock_date).bpf_unwra(),
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
