use solana_api_types::{AccountMeta, Instruction, Pubkey};

pub fn find_associated_wallet(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let mut seed: u8 = std::u8::MAX;
    loop {
        if let Some(pubkey) = Pubkey::create_program_address(
            &[
                owner.as_ref(),
                solar::spl::ID.as_ref(),
                mint.as_ref(),
                &[seed],
            ],
            solar::spl::ASSOCIATED_TOKEN_ID,
        ) {
            return pubkey;
        }

        seed -= 1;
    }
}

pub fn initialize_associated_wallet(payer: &Pubkey, owner: &Pubkey, mint: &Pubkey) -> Instruction {
    let address = find_associated_wallet(owner, mint);

    Instruction {
        program_id: *solar::spl::ASSOCIATED_TOKEN_ID,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(address, false),
            AccountMeta::new_readonly(*owner, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*solana_api_types::system::ID, false),
            AccountMeta::new_readonly(*solar::spl::ID, false),
            AccountMeta::new_readonly(*solana_api_types::sysvar::rent::ID, false),
        ],
        data: vec![],
    }
}
