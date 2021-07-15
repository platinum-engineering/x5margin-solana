use solana_program::pubkey::Pubkey;
use solar::{
    data::try_reinterpret,
    input::ProgramInput,
    spl::{TokenProgram, Wallet},
};

use crate::{
    data::{Entity, EntityAllocator, EntityId, EntityKind, Farm, RequestQueue, StakerRegistry},
    error::Error,
};

use super::Operation;

pub struct CreateFarmArgs {
    pub salt: u64,
}

// TODO: replace asserts with proper error handling
#[inline(never)]
pub fn handle(input: &mut ProgramInput, op: &Operation) -> Result<(), Error> {
    let [
        // TODO: macroify
        token_program,
        administrator_authority,
        program_authority,
        rent,
        farm,
        request_queue,
        stakers,
        active_stake_vault,
        inactive_stake_vault,
        reward_vault
    ] = input.take_accounts::<10>();

    let token_program = TokenProgram::load(token_program)?;

    // TODO(mori): wrap in macro or helper with extra checks
    let args =
        unsafe { try_reinterpret::<CreateFarmArgs>(op.data).expect("couldn't parse arguments") };

    let (derived_program_authority, nonce) = Pubkey::find_program_address(
        &[
            administrator_authority.key().as_ref(),
            &args.salt.to_le_bytes(),
        ],
        input.program_id(),
    );

    assert!(&derived_program_authority == program_authority.key);
    assert!(active_stake_vault.owner == token_program.key);

    assert!(inactive_stake_vault.owner == token_program.key);
    assert!(reward_vault.owner == token_program.key);

    let active_stake_vault_data =
        unsafe { try_reinterpret::<Wallet>(active_stake_vault.data).unwrap() };
    let inactive_stake_vault_data =
        unsafe { try_reinterpret::<Wallet>(inactive_stake_vault.data).unwrap() };
    let reward_vault_data = unsafe { try_reinterpret::<Wallet>(reward_vault.data).unwrap() };

    assert!(active_stake_vault_data.owner() == program_authority.key);
    assert!(inactive_stake_vault_data.owner() == program_authority.key);
    assert!(reward_vault_data.owner() == program_authority.key);
    assert!(active_stake_vault_data.mint() == inactive_stake_vault_data.mint());

    let mut farm = Entity::new(input.program_id(), farm);
    let mut request_queue = Entity::new(input.program_id(), request_queue);
    let mut stakers = Entity::new(input.program_id(), stakers);

    let farm_id = EntityId::new(0);
    let farm_key = *farm.account().key;
    let mut allocator = EntityAllocator::default();
    farm.initialize(
        &mut allocator,
        farm.account().key,
        farm_id,
        EntityKind::Root,
    );

    let farm_data = farm.body_mut::<Farm>();
    farm_data.administrator_authority = *administrator_authority.key;
    farm_data.program_authority = *program_authority.key;
    farm_data.program_authority_nonce = nonce;
    farm_data.program_authority_salt = args.salt;
    farm_data.active_stake_vault = *active_stake_vault.key;
    farm_data.inactive_stake_vault = *inactive_stake_vault.key;
    farm_data.reward_vault = *reward_vault.key;
    farm_data.allocator = allocator;

    let allocator = &mut farm_data.allocator;

    request_queue.initialize(allocator, &farm_key, farm_id, EntityKind::RequestQueue);
    request_queue.body::<RequestQueue>();
    stakers.initialize(allocator, &farm_key, farm_id, EntityKind::StakerRegistry);
    stakers.body::<StakerRegistry>();

    Ok(())
}
