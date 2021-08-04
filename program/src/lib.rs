#![allow(stable_features)]
#![feature(min_const_generics)]

use fixed::types::U64F64;
use simple_stake::StakePoolEntity;
use solana_program::entrypoint::ProgramResult;
use solar::{
    input::{BpfProgramInput, Entrypoint, ProgramInput},
    math::Checked,
    util::ResultExt,
};

#[macro_use]
extern crate static_assertions;

pub mod complex;
pub mod data;
pub mod error;
pub mod simple_stake;

pub type TokenAmount = Checked<u64>;
pub type TokenAmountF64 = Checked<U64F64>;

#[derive(Debug, PartialEq, Eq, Clone, parity_scale_codec::Encode, parity_scale_codec::Decode)]
pub enum Method {
    Simple(simple_stake::Method),
}

#[allow(unused)]
pub fn main(mut input: BpfProgramInput) -> ProgramResult {
    let mut data = input.data();
    let method: Method = parity_scale_codec::Decode::decode(&mut data)
        .ok()
        .bpf_expect("could not parse method");

    let result = match method {
        Method::Simple(method) => match method {
            simple_stake::Method::CreatePool(args) => StakePoolEntity::initialize(&mut input, args),
            simple_stake::Method::Stake { amount } => {
                StakePoolEntity::add_stake(&mut input, amount)
            }
            simple_stake::Method::Unstake { amount } => {
                StakePoolEntity::remove_stake(&mut input, amount)
            }
            simple_stake::Method::ClaimReward => StakePoolEntity::claim_reward(&mut input),
            simple_stake::Method::AddReward { amount } => {
                StakePoolEntity::add_reward(&mut input, amount)
            }
        },
    };

    if result.is_err() {
        dbg!(result);
    }

    Ok(())
}

pub struct Program;

impl Entrypoint for Program {
    fn call(input: BpfProgramInput) -> ProgramResult {
        main(input)
    }
}

#[cfg(test)]
mod test {
    use std::mem::size_of;

    use parity_scale_codec::Encode;
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_instruction::create_account,
    };
    use solana_program_test::{builtin_process_instruction, ProgramTest};
    use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};
    use solar::{
        input::wrapped_entrypoint,
        spl::{Mint, Wallet},
        util::minimum_balance,
    };

    use crate::{
        data::AccountType,
        simple_stake::{self, InitializeArgs, StakePool},
        Method,
    };

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        let mut program_test = ProgramTest::default();
        let program_id = Pubkey::new_unique();

        program_test.add_program(
            "x5margin",
            program_id,
            Some(|a, b, c| {
                builtin_process_instruction(wrapped_entrypoint::<super::Program>, a, b, c)
            }),
        );

        let pool_key = Keypair::new();
        let pool_administrator_key = Keypair::new();

        let mut salt: u64 = 0;
        let pool_program_authority = loop {
            let pool_program_authority = Pubkey::create_program_address(
                &[
                    pool_key.pubkey().as_ref(),
                    pool_administrator_key.pubkey().as_ref(),
                    &salt.to_le_bytes(),
                ],
                &program_id,
            );

            match pool_program_authority {
                Ok(s) => break s,
                Err(_) => {
                    salt += 1;
                }
            }
        };

        let (mut client, payer, hash) = program_test.start().await;

        let stake_mint_key = Keypair::new();
        let stake_vault_key = Keypair::new();

        let instrs = vec![
            create_account(
                &payer.pubkey(),
                &stake_mint_key.pubkey(),
                minimum_balance(size_of::<Mint>() as u64),
                size_of::<Mint>() as u64,
                &solar::spl::ID,
            ),
            create_account(
                &payer.pubkey(),
                &stake_vault_key.pubkey(),
                minimum_balance(size_of::<Wallet>() as u64),
                size_of::<Wallet>() as u64,
                &solar::spl::ID,
            ),
            create_account(
                &payer.pubkey(),
                &pool_key.pubkey(),
                StakePool::default_lamports(),
                StakePool::default_size() as u64,
                &program_id,
            ),
            spl_token::instruction::initialize_mint(
                &solar::spl::ID,
                &stake_mint_key.pubkey(),
                &pool_administrator_key.pubkey(),
                None,
                6,
            )
            .unwrap(),
            spl_token::instruction::initialize_account(
                &solar::spl::ID,
                &stake_vault_key.pubkey(),
                &stake_mint_key.pubkey(),
                &pool_program_authority,
            )
            .unwrap(),
            Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new_readonly(pool_administrator_key.pubkey(), false),
                    AccountMeta::new_readonly(pool_program_authority, false),
                    AccountMeta::new(pool_key.pubkey(), false),
                    AccountMeta::new_readonly(stake_mint_key.pubkey(), false),
                    AccountMeta::new_readonly(stake_vault_key.pubkey(), false),
                ],
                data: Method::Simple(simple_stake::Method::CreatePool(InitializeArgs {
                    program_authority_salt: salt,
                    lockup_duration: 0.into(),
                    topup_duration: 0.into(),
                    reward_amount: 0.into(),
                    target_amount: 0.into(),
                }))
                .encode(),
            },
        ];

        let trx = Transaction::new_signed_with_payer(
            &instrs,
            Some(&payer.pubkey()),
            &vec![&payer, &stake_mint_key, &stake_vault_key, &pool_key],
            hash,
        );

        let result = client.process_transaction(trx).await;
        println!("{:?}", result);

        Ok(())
    }
}
