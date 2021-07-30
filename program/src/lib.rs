#![allow(stable_features)]
#![feature(min_const_generics)]

use solana_program::entrypoint::ProgramResult;
use solar::{
    bytecode_marker,
    input::{Entrypoint, ProgramInput},
    util::is_zeroed,
};

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate borsh;

pub mod complex;
pub mod data;
pub mod error;
pub mod ops;
pub mod simple_stake;

#[allow(unused)]
pub fn main(mut input: ProgramInput) -> ProgramResult {
    bytecode_marker!(start);
    assert!(is_zeroed(input.data()));
    bytecode_marker!(end);

    Ok(())
}

pub struct Program;

impl Entrypoint for Program {
    fn call(input: ProgramInput) -> ProgramResult {
        main(input)
    }
}

#[cfg(test)]
mod test {
    use solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    };
    use solana_program_test::{builtin_process_instruction, ProgramTest};
    use solana_sdk::{signer::Signer, transaction::Transaction};
    use solar::input::wrapped_entrypoint;

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

        let (mut client, payer, hash) = program_test.start().await;

        let instr = Instruction {
            program_id,
            accounts: vec![AccountMeta::new(solar::spl::ID, false)],
            data: vec![],
        };

        let trx =
            Transaction::new_signed_with_payer(&[instr], Some(&payer.pubkey()), &[&payer], hash);

        let result = client.process_transaction(trx).await;
        println!("{:?}", result);

        Ok(())
    }
}
