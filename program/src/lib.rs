#![allow(stable_features)]
#![feature(min_const_generics)]

use ops::Operation;
use solana_program::entrypoint::ProgramResult;
use solar::input::ProgramInput;

#[macro_use]
extern crate static_assertions;

pub mod data;
pub mod error;
pub mod ops;

#[allow(unused)]
pub fn main(mut input: ProgramInput) -> ProgramResult {
    let op = Operation::from_buf(input.data());

    // // TODO(mori): macro-ify this
    // match op.header.kind {
    //     OperationKind::Poll => ops::poll::handle(&mut input, &op)?,
    //     OperationKind::CreateFarm => ops::create_farm::handle(&mut input, &op)?,
    //     OperationKind::Stake => ops::stake::handle(&mut input, &op)?,
    //     OperationKind::Unstake => todo!(),
    //     OperationKind::ClaimRewards => todo!(),
    //     OperationKind::Unknown => todo!(),
    // }

    Ok(())
}
