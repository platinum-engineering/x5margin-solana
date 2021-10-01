use data::TokenLock;
use fixed::types::U64F64;
use parity_scale_codec::Decode;

use solar::{
    account::AccountFields,
    authority::Authority,
    math::Checked,
    spl::{TokenProgram, WalletAccount},
    time::SolTimestamp,
};

pub mod data;
pub mod error;

#[cfg(feature = "onchain")]
pub mod logic;

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
    Withdraw {
        amount: TokenAmount,
    },
    Increment {
        amount: TokenAmount,
    },
    Split,
    ChangeOwner,
}

pub mod instructions {
    use super::*;

    account_schema! {
        name = CreateArgs,
        accounts = [
            token_program: &TokenProgram<B> = TokenProgram::load(this)?;
            locker: &mut TokenLock<B> = TokenLock::blank(&program_id, this)?;
            source_wallet: &mut WalletAccount<B> = WalletAccount::any(this)?;
            source_authority #s: &Authority<B> = Authority::expected_signed(this, source_wallet.authority())?;
            vault: &mut WalletAccount<B> = WalletAccount::any(this)?;
            program_authority: &Authority<B> = Authority::any(this);
            owner_authority #s: &Authority<B> = Authority::any_signed(this)?;
        ]
    }

    account_schema! {
        name = ReLock,
        accounts = [
            locker: &mut TokenLock<B> = TokenLock::initialized(&program_id, this)?;
            owner_authority #s: &Authority<B> = Authority::any_signed(this)?;
        ]
    }

    account_schema! {
        name = Withdraw,
        accounts = [
            token_program: &TokenProgram<B> = TokenProgram::load(this)?;
            locker: &mut TokenLock<B> = TokenLock::initialized(&program_id, this)?;
            vault: &mut WalletAccount<B> = WalletAccount::any(this)?;
            destination_wallet: &mut WalletAccount<B> = WalletAccount::any(this)?;
            program_authority: &Authority<B> = Authority::expected(this, &locker.read().program_authority)?;
            owner_authority #s: &Authority<B> = Authority::expected_signed(this, &locker.read().withdraw_authority)?;
        ]
    }

    account_schema! {
        name = Increment,
        accounts = [
            token_program: &TokenProgram<B> = TokenProgram::load(this)?;
            locker: &mut TokenLock<B> = TokenLock::initialized(&program_id, this)?;
            vault: &mut WalletAccount<B> = WalletAccount::any(this)?;
            source_wallet: &mut WalletAccount<B> = WalletAccount::any(this)?;
            source_authority: &Authority<B> = Authority::expected_signed(this, source_wallet.authority())?;
        ]
    }

    account_schema! {
        name = Split,
        accounts = [
            token_program: &TokenProgram<B> = TokenProgram::load(this)?;
            source_locker: &mut TokenLock<B> = TokenLock::initialized(&program_id, this)?;
            new_locker: &mut TokenLock<B> = TokenLock::blank(&program_id, this)?;
            source_vault: &mut WalletAccount<B> = WalletAccount::any(this)?;
            new_vault: &mut WalletAccount<B> = WalletAccount::any(this)?;
            owner_authority #s: &Authority<B> = Authority::expected_signed(this, &source_locker.read().withdraw_authority)?;
        ]
    }

    account_schema! {
        name = ChangeOwner,
        accounts = [
            locker: &mut TokenLock<B> = TokenLock::initialized(&program_id, this)?;
            owner_authority #s: &Authority<B> = Authority::expected_signed(this, &locker.read().withdraw_authority)?;
            new_owner_authority: &Authority<B> = Authority::any(this);
        ]
    }
}

#[cfg(feature = "onchain")]
pub use logic::{main, Program};

#[cfg(test)]
#[cfg(feature = "__disabled")]
mod test {
    use solana_api_types::{program_test::ProgramTest, Keypair, Pubkey, Signer};
    use solana_program_test::builtin_process_instruction;
    use solar::input::wrapped_entrypoint;

    #[async_std::test]
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

        // let locker_key = Keypair::new();
        // let locker_owner_key = Keypair::new();

        // let mut salt: u64 = 0;
        // let locker_program_authority = loop {
        //     let locker_program_authority = Pubkey::create_program_address(
        //         &[
        //             locker_key.pubkey().as_ref(),
        //             locker_owner_key.pubkey().as_ref(),
        //         ],
        //         &program_id,
        //     );

        //     match locker_program_authority {
        //         Some(s) => break s,
        //         None => {
        //             salt += 1;
        //         }
        //     }
        // };

        // let (mut client, payer, hash) = program_test.start().await;

        Ok(())
    }
}
