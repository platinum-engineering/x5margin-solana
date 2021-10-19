use parity_scale_codec::Decode;
use solana_api_types::{program::ProgramError, Pubkey};
use solar::{
    account::{onchain, AccountFields, AccountFieldsMut},
    authority::Authority,
    input::{AccountSource, BpfProgramInput, ProgramInput},
    prelude::AccountBackend,
    qlog,
    time::SolTimestamp,
    util::{pubkey_eq, sol_timestamp_now, ResultExt},
};

use crate::{data::TokenLock, error::Error, instructions, Method, TokenAmount, UnlockDate};

impl<B: AccountBackend> TokenLock<B> {
    /// Create a new locker.
    pub fn create<S: AccountSource<B>>(
        mut input: S,
        unlock_date: UnlockDate,
        amount: TokenAmount,
        nonce: u64,
    ) -> Result<(), Error>
    where
        B: AccountBackend<Impl = onchain::Account>,
    {
        let mut parsed = instructions::CreateArgs::from_program_input(&mut input)?;
        let instructions::CreateArgsParsed {
            token_program,
            locker,
            source_wallet,
            source_authority,
            vault,
            program_authority,
            // owner_authority,
        } = parsed.borrow();

        let owner_authority = if input.is_empty() {
            *source_authority.key()
        } else {
            let owner_authority = Authority::any(input.next_account());
            *owner_authority.key()
        };

        let now = sol_timestamp_now();
        let unlock_date = match unlock_date {
            UnlockDate::Absolute(timestamp) => timestamp,
            UnlockDate::Relative(delta) => SolTimestamp::from(Into::<i64>::into(now) + delta),
        };

        if unlock_date <= now {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData);
        }

        let expected_program_authority = Pubkey::create_program_address(
            &[
                locker.account().key().as_ref(),
                owner_authority.as_ref(),
                &nonce.to_le_bytes(),
            ],
            input.program_id(),
        )
        .bpf_expect("couldn't derive program authority");

        if !pubkey_eq(program_authority.key(), &expected_program_authority) {
            qlog!("provided program authority does not match expected authority");
            return Err(Error::InvalidAuthority);
        }

        if !pubkey_eq(vault.authority(), &expected_program_authority) {
            qlog!("vault authority does not match program authority");
            return Err(Error::InvalidAuthority);
        }

        token_program
            .transfer(source_wallet, vault, amount.value(), source_authority, &[])
            .bpf_expect("transfer")
            .bpf_expect("transfer");

        let data = locker.read_mut();
        data.withdraw_authority = owner_authority;
        data.mint = *source_wallet.mint();
        data.vault = *vault.key();
        data.program_authority = *program_authority.key();
        data.release_date = unlock_date;

        Ok(())
    }

    /// Relocks an existing locker with a new unlock date.
    pub fn relock<S: AccountSource<B>>(mut input: S, unlock_date: SolTimestamp) -> Result<(), Error>
    where
        B::Impl: AccountFieldsMut,
    {
        let mut parsed = instructions::ReLock::from_program_input(&mut input)?;
        let instructions::ReLockParsed {
            locker,
            owner_authority: _,
        } = parsed.borrow();

        if unlock_date <= locker.read().release_date {
            qlog!("can`t initialize new locker with invalid unlock date");
            return Err(Error::InvalidData);
        }

        locker.read_mut().release_date = unlock_date;

        Ok(())
    }

    /// Withdraw funds from locker.
    pub fn withdraw<S: AccountSource<B>>(mut input: S, amount: TokenAmount) -> Result<(), Error>
    where
        B: AccountBackend<Impl = onchain::Account>,
    {
        let mut parsed = instructions::Withdraw::from_program_input(&mut input)?;
        let instructions::WithdrawParsed {
            token_program,
            locker,
            vault,
            destination_wallet,
            program_authority,
            owner_authority,
        } = parsed.borrow();

        if locker.read().release_date > sol_timestamp_now() {
            qlog!("too early to withdraw");
            return Err(Error::Validation);
        }

        if !pubkey_eq(&locker.read().vault, vault.key()) {
            qlog!("invalid vault");
            return Err(Error::Validation);
        }

        token_program
            .transfer(
                vault,
                destination_wallet,
                amount.value(),
                program_authority,
                &[&[
                    locker.account().key().as_ref(),
                    owner_authority.key().as_ref(),
                ]],
            )
            .bpf_expect("transfer")
            .bpf_expect("transfer");

        Ok(())
    }

    /// Add funds to locker
    pub fn increment<S: AccountSource<B>>(mut input: S, amount: TokenAmount) -> Result<(), Error>
    where
        B: AccountBackend<Impl = onchain::Account>,
    {
        let mut parsed = instructions::Increment::from_program_input(&mut input)?;
        let instructions::IncrementParsed {
            token_program,
            locker,
            vault,
            source_wallet,
            source_authority,
        } = parsed.borrow();

        if locker.read().release_date <= sol_timestamp_now() {
            qlog!("too late to increment");
            return Err(Error::Validation);
        }

        if !pubkey_eq(&locker.read().vault, vault.key()) {
            qlog!("invalid vault");
            return Err(Error::Validation);
        }

        token_program
            .transfer(source_wallet, vault, amount.value(), source_authority, &[])
            .bpf_expect("transfer")
            .bpf_expect("transfer");

        Ok(())
    }
}

pub fn main(input: BpfProgramInput) -> Result<(), ProgramError> {
    let mut data = input.data();
    let method = Method::decode(&mut data)
        .ok()
        .bpf_expect("couldn't parse method");

    match method {
        Method::CreateLock {
            unlock_date,
            amount,
            nonce,
        } => TokenLock::create(input, unlock_date, amount, nonce).bpf_unwrap(),
        Method::ReLock { unlock_date } => TokenLock::relock(input, unlock_date).bpf_unwrap(),
        Method::Withdraw { amount } => TokenLock::withdraw(input, amount).bpf_unwrap(),
        Method::Increment { amount } => TokenLock::increment(input, amount).bpf_unwrap(),
        Method::Split => todo!(),
        Method::ChangeOwner => todo!(),
    }

    Ok(())
}

pub struct Program;

impl solar::input::Entrypoint for Program {
    fn call(input: BpfProgramInput) -> solana_api_types::program::ProgramResult {
        main(input)
    }
}
