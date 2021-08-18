#![allow(stable_features)]
#![feature(min_const_generics)]

use fixed::types::U64F64;
#[cfg(feature = "onchain")]
use simple_stake::StakePoolEntity;
#[cfg(feature = "onchain")]
use solana_api_types::program::ProgramResult;
#[cfg(feature = "onchain")]
use solar::input::{BpfProgramInput, Entrypoint, ProgramInput};
use solar::math::Checked;
#[cfg(feature = "onchain")]
use solar::util::ResultExt;

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

#[cfg(feature = "onchain")]
#[allow(unused)]
pub fn main(mut input: BpfProgramInput) -> ProgramResult {
    use solar::qlog;

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

#[cfg(feature = "onchain")]
impl Entrypoint for Program {
    fn call(input: BpfProgramInput) -> ProgramResult {
        main(input)
    }
}

#[cfg(feature = "onchain")]
#[cfg(test)]
mod test {
    use parity_scale_codec::Encode;
    use solana_program_test::builtin_process_instruction;
    use solar::{
        input::wrapped_entrypoint,
        spl::{create_mint, create_wallet, mint_to},
        util::minimum_balance,
    };

    use solana_api_types::{
        program_test::ProgramTest, system::create_account, AccountMeta, Instruction, Keypair,
        Pubkey, Signer, Transaction,
    };

    use crate::{
        data::AccountType,
        simple_stake::{self, InitializeArgs, StakePool, StakePoolEntity, StakerTicket},
        Method,
    };

    #[tokio::test]
    async fn create_test() -> anyhow::Result<()> {
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
                Some(s) => break s,
                None => {
                    salt += 1;
                }
            }
        };

        let (mut client, payer, hash) = program_test.start().await;

        let stake_mint_key = Keypair::new();
        let stake_vault_key = Keypair::new();
        let aux_wallet_key = Keypair::new();

        let staker_key = Keypair::new();
        let staker_ticket_key = Keypair::new();

        let mut instrs = vec![];
        instrs.extend(create_mint(
            &payer.pubkey(),
            &stake_mint_key.pubkey(),
            &pool_administrator_key.pubkey(),
            6,
        ));
        instrs.extend(create_wallet(
            &payer.pubkey(),
            &stake_vault_key.pubkey(),
            &stake_mint_key.pubkey(),
            &pool_program_authority,
        ));
        instrs.extend(create_wallet(
            &payer.pubkey(),
            &aux_wallet_key.pubkey(),
            &stake_mint_key.pubkey(),
            &pool_administrator_key.pubkey(),
        ));
        instrs.push(mint_to(
            &stake_mint_key.pubkey(),
            &aux_wallet_key.pubkey(),
            &pool_administrator_key.pubkey(),
            1_000_000,
        ));
        instrs.push(create_account(
            &payer.pubkey(),
            &pool_key.pubkey(),
            minimum_balance(StakePool::default_size() as u64),
            StakePool::default_size() as u64,
            &program_id,
        ));
        instrs.push(create_account(
            &payer.pubkey(),
            &staker_ticket_key.pubkey(),
            minimum_balance(StakerTicket::default_size() as u64),
            StakerTicket::default_size() as u64,
            &program_id,
        ));

        instrs.push(Instruction {
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
                lockup_duration: 1000.into(),
                topup_duration: 200.into(),
                reward_amount: 1000.into(),
                target_amount: 10000.into(),
            }))
            .encode(),
        });

        instrs.push(Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new_readonly(*solar::spl::ID, false),
                AccountMeta::new(pool_key.pubkey(), false),
                AccountMeta::new_readonly(staker_key.pubkey(), false),
                AccountMeta::new(staker_ticket_key.pubkey(), false),
                AccountMeta::new(stake_vault_key.pubkey(), false),
                AccountMeta::new_readonly(pool_administrator_key.pubkey(), true),
                AccountMeta::new(aux_wallet_key.pubkey(), false),
            ],
            data: Method::Simple(simple_stake::Method::Stake {
                amount: 10000.into(),
            })
            .encode(),
        });

        let trx = Transaction::new_signed_with_payer(
            &instrs,
            Some(&payer.pubkey()),
            &vec![
                &payer,
                &stake_mint_key,
                &stake_vault_key,
                &pool_key,
                &aux_wallet_key,
                &pool_administrator_key,
                &staker_ticket_key,
            ],
            hash,
        );

        let result = client.process_transaction(trx).await;
        println!("{:?}", result);

        let stake_pool = client
            .get_account(&pool_key.pubkey())
            .await
            .unwrap()
            .unwrap();

        let staker_ticket = client
            .get_account(&staker_ticket_key.pubkey())
            .await
            .unwrap()
            .unwrap();

        let stake_pool = StakePoolEntity::load(&program_id, &stake_pool).unwrap();
        let staker_ticket = stake_pool.load_ticket(&staker_ticket).unwrap();

        assert!(stake_pool.stake_acquired_amount == 10000.into());
        assert!(stake_pool.stake_target_amount == 10000.into());
        assert!(staker_ticket.staked_amount == 10000.into());

        Ok(())
    }

    #[tokio::test]
    async fn stake_test() -> anyhow::Result<()> {
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

        let amount: super::TokenAmount = solar::math::Checked::from(100);

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
                data: Method::Simple(simple_stake::Method::Stake{amount: amount})
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

    #[tokio::test]
    async fn unstake_test() -> anyhow::Result<()> {
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

        let amount: super::TokenAmount = solar::math::Checked::from(100);

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
                data: Method::Simple(simple_stake::Method::Unstake{amount: amount})
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

    #[tokio::test]
    async fn clim_reward_test() -> anyhow::Result<()> {
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
                data: Method::Simple(simple_stake::Method::ClaimReward)
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

    #[tokio::test]
    async fn add_reward_test() -> anyhow::Result<()> {
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

        let amount: super::TokenAmount = solar::math::Checked::from(100);

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
                data: Method::Simple(simple_stake::Method::AddReward{amount: amount})
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
