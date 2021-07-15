use solana_program::{clock::Clock, entrypoint::ProgramResult, program_pack::Pack, sysvar::Sysvar};
use solar::{bytecode_marker, data::try_reinterpret, input::ProgramInput, qlog, util::pubkey_eq};

use crate::{
    data::{Entity, Farm, Request, RequestQueue, Staker, StakerRegistry},
    ops::stake,
};

use super::Operation;

pub struct StakeArgs {
    pub amount: u64,
}

#[inline(never)]
pub fn handle(input: &mut ProgramInput, op: &Operation) -> ProgramResult {
    let [
        // TODO: macroify
        token_program,
        source_wallet,
        staker,
        farm,
        request_queue,
        stakers,
        inactive_stake_vault,
    ] = input.take_accounts::<7>();

    bytecode_marker!(marker_a);
    qlog!(staker.key.as_ref()[0], " is the first bye of staker");
    bytecode_marker!(marker_b);

    assert!(pubkey_eq(token_program.key, &spl_token::ID));
    assert!(pubkey_eq(source_wallet.owner, &spl_token::ID));

    let source_wallet_data = spl_token::state::Account::unpack(source_wallet.data).unwrap();
    assert!(&source_wallet_data.owner == staker.key);
    assert!(staker.is_signer);

    let args = unsafe { try_reinterpret::<StakeArgs>(op.data).unwrap() };
    let mut farm = Entity::new(input.program_id(), farm);
    let mut request_queue = Entity::new(input.program_id(), request_queue);
    let mut stakers = Entity::new(input.program_id(), stakers);

    // TODO: transfer tokens from source wallet to inactive stake vault
    // TODO: instead of creating a new Staker for every Stake operation, allow specifying an idx into an existing staker registry
    // TODO: debit the staker for storage rent (= lamports(size(Staker)))
    let staker_data = Staker {
        authority: *staker.key,
        active_stake: 0,
        inactive_stake: args.amount,
        unclaimed_reward: 0,
    };

    let clock = Clock::get().unwrap();

    let request = Request {
        slot: clock.slot,
        kind: crate::data::RequestKind::AddStake {
            staker: *staker.key,
            amount: args.amount,
        },
    };

    let farm_data = farm.body_mut::<Farm>();
    let request_queue_data = request_queue.body_mut::<RequestQueue>();
    let stakers_data = stakers.body_mut::<StakerRegistry>();

    farm_data.inactive_stake += args.amount;
    request_queue_data.requests.push(request);
    stakers_data.stakers.push(staker_data);

    Ok(())
}
