use data::TokenLock;
use fixed::types::U64F64;
use parity_scale_codec::Decode;
use solana_api_types::{program::ProgramError, Instruction, Pubkey};
#[cfg(feature = "onchain")]
use solar::input::BpfProgramInput;
use solar::{
    account::{AccountFields, AccountFieldsMut},
    input::{AccountSource, Entrypoint, ProgramInput},
    math::Checked,
    prelude::AccountBackend,
    qlog,
    spl::WalletAccount,
    time::SolTimestamp,
    util::{pubkey_eq, sol_timestamp_now, timestamp_now, ResultExt},
};

pub mod data;
pub mod error;

#[macro_use]
extern crate parity_scale_codec;

#[macro_use]
extern crate solar_macros;

use crate::error::Error;

pub type TokenAmount = Checked<u64>;
pub type TokenAmountF64 = Checked<U64F64>;

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

#[derive(Debug)]
pub struct CreateArgsAccounts<B: AccountBackend> {
    pub locker: TokenLock<B>, //(empty, uninitialized)
    pub source_wallet: WalletAccount<B>,
    pub source_authority: B,     //(signed)
    pub vault: WalletAccount<B>, //(authority = program authority)
    pub program_authority: B,
    pub owner_authority: B, //withdraw authority
}

impl<B: AccountBackend> CreateArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker = TokenLock::blank(input.program_id(), this)?,
            &source_wallet = WalletAccount::any(this)?,
            &source_authority,
            &vault = WalletAccount::any(this)?,
            &program_authority,
            &owner_authority,
        }

        Ok(Self {
            locker,
            source_wallet,
            source_authority,
            vault,
            program_authority,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct ReLockArgsAccounts<B: AccountBackend> {
    pub locker: TokenLock<B>, //(empty, uninitialized)
    pub owner_authority: B,   //withdraw authority
}

impl<B: AccountBackend> ReLockArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker = TokenLock::initialized(input.program_id(), this)?,
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
    pub vault: WalletAccount<B>,
    pub destination_wallet: B,
    pub program_authority: B,
    pub owner_authority: B,
}

impl<B: AccountBackend> WithdrawArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut locker,
            &spl_token_wallet_vault = WalletAccount::any(this)?,
            &destination_spl_token_wallet,
            &program_authority,
            &owner_authority,
        }

        Ok(Self {
            locker,
            vault: spl_token_wallet_vault,
            destination_wallet: destination_spl_token_wallet,
            program_authority,
            owner_authority,
        })
    }
}

#[derive(Debug)]
pub struct IncrementArgsAccounts<B: AccountBackend> {
    pub locker: B,
    pub vault: WalletAccount<B>,
    pub source_wallet: B,
    pub source_authority: B,
}

impl<B: AccountBackend> IncrementArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &locker,
            &spl_token_wallet_vault = WalletAccount::any(this)?,
            &source_spl_token_wallet,
            &source_authority,
        }

        Ok(Self {
            locker,
            vault: spl_token_wallet_vault,
            source_wallet: source_spl_token_wallet,
            source_authority,
        })
    }
}

#[derive(Debug)]
pub struct SplitArgsAccounts<B: AccountBackend> {
    pub source_locker: TokenLock<B>,
    pub new_locker: TokenLock<B>, //(empty, uninitialized)
    pub source_vault: WalletAccount<B>,
    pub new_vault: WalletAccount<B>,
}

impl<B: AccountBackend> SplitArgsAccounts<B> {
    #[cfg(feature = "onchain")]
    #[inline]
    pub fn from_program_input<T: AccountSource<B>>(input: &mut T) -> Result<Self, Error> {
        parse_accounts! {
            &mut source_locker = TokenLock::initialized(input.program_id(), this)?,
            &mut new_locker = TokenLock::blank(input.program_id(), this)?,
            &source_vault = WalletAccount::any(this)?,
            &new_vault = WalletAccount::any(this)?,
        }

        Ok(Self {
            new_vault,
            source_locker,
            new_locker,
            source_vault,
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
    #[cfg(feature = "onchain")]
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
        mut input: S,
        unlock_date: SolTimestamp,
        amount: TokenAmount,
    ) -> Result<(), Error>
    where
        B::Impl: AccountFieldsMut,
    {
        let CreateArgsAccounts {
            mut locker,
            source_wallet: source_spl_token_wallet,
            source_authority,
            vault: spl_token_wallet_vault,
            program_authority,
            owner_authority,
        } = CreateArgsAccounts::from_program_input(&mut input)?;

        let now = sol_timestamp_now();

        if unlock_date <= now {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData);
        }

        let expected_program_authority = Pubkey::create_program_address(
            &[
                locker.account().key().as_ref(),
                owner_authority.key().as_ref(),
            ],
            input.program_id(),
        )
        .bpf_expect("couldn't derive program authority");

        if !pubkey_eq(program_authority.key(), &expected_program_authority) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(
            spl_token_wallet_vault.authority(),
            &expected_program_authority,
        ) {
            qlog!("spl token wallet vault authority does not match program authority");
            return Err(Error::InvalidAuthority);
        }

        let data = locker.read_mut();
        data.withdraw_authority = *owner_authority.key();
        data.mint = *source_spl_token_wallet.mint();
        data.vault = *spl_token_wallet_vault.key();
        data.program_authority = *program_authority.key();
        data.release_date = unlock_date;

        Ok(())
    }

    /// Relocks an existing locker with a new unlock date.
    ///
    /// Input accounts:
    /// Locker
    /// Locker Owner (signed)
    pub fn relock<S: AccountSource<B>>(mut input: S, unlock_date: SolTimestamp) -> Result<(), Error>
    where
        B::Impl: AccountFieldsMut,
    {
        let ReLockArgsAccounts {
            mut locker,
            owner_authority,
        } = ReLockArgsAccounts::from_program_input(&mut input)?;

        if !pubkey_eq(locker.read().withdraw_authority, owner_authority.key()) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if unlock_date <= locker.read().release_date {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData);
        }

        locker.read_mut().release_date = unlock_date;

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
            vault: spl_token_wallet_vault,
            destination_wallet: destination_spl_token_wallet,
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
            vault: spl_token_wallet_vault,
            source_wallet: source_spl_token_wallet,
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
            new_vault,
            source_locker,
            new_locker,
            source_vault: source_vault,
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
        Method::ReLock { unlock_date } => TokenLock::relock(input, unlock_date).bpf_unwrap(),
        Method::Withdraw => todo!(),
        Method::Increment => todo!(),
        Method::Split => todo!(),
        Method::ChangeOwner => todo!(),
    }

    Ok(())
}

struct Program;

impl Entrypoint for Program {
    fn call(input: BpfProgramInput) -> solana_api_types::program::ProgramResult {
        main(input)
    }
}

#[cfg(feature = "onchain")]
#[cfg(test)]
mod test {
    use solana_api_types::{program_test::ProgramTest, Keypair, Pubkey, Signer};
    use solana_program_test::builtin_process_instruction;
    use solar::input::wrapped_entrypoint;

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
