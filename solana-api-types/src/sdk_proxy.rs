use crate::{
    program::ProgramError,
    sysvar::{clock::Clock, rent::Rent},
    AccountMeta, CompiledInstruction, Hash, Instruction, Message, Pubkey, Signature, Transaction,
};

pub trait ToSdk {
    type Original;

    fn to_sdk(&self) -> Self::Original;
}

pub trait FromSdk {
    type Original;

    fn from_sdk(sdk: &Self::Original) -> Self;
}

impl ToSdk for Pubkey {
    type Original = solana_program::pubkey::Pubkey;

    fn to_sdk(&self) -> Self::Original {
        solana_program::pubkey::Pubkey::new_from_array(*self.as_bytes())
    }
}

impl ToSdk for Signature {
    type Original = solana_sdk::signature::Signature;

    fn to_sdk(&self) -> Self::Original {
        solana_sdk::signature::Signature::new(self.as_ref())
    }
}

impl ToSdk for Hash {
    type Original = solana_sdk::hash::Hash;

    fn to_sdk(&self) -> Self::Original {
        solana_sdk::hash::Hash::new_from_array(self.0)
    }
}

impl ToSdk for AccountMeta {
    type Original = solana_sdk::instruction::AccountMeta;

    fn to_sdk(&self) -> Self::Original {
        if self.is_writable {
            solana_sdk::instruction::AccountMeta::new(self.pubkey.to_sdk(), self.is_signer)
        } else {
            solana_sdk::instruction::AccountMeta::new_readonly(self.pubkey.to_sdk(), self.is_signer)
        }
    }
}

impl ToSdk for Instruction {
    type Original = solana_sdk::instruction::Instruction;

    fn to_sdk(&self) -> Self::Original {
        solana_sdk::instruction::Instruction {
            program_id: self.program_id.to_sdk(),
            accounts: self.accounts.iter().map(|s| s.to_sdk()).collect(),
            data: self.data.clone(),
        }
    }
}

impl ToSdk for CompiledInstruction {
    type Original = solana_sdk::instruction::CompiledInstruction;

    fn to_sdk(&self) -> Self::Original {
        solana_sdk::instruction::CompiledInstruction {
            program_id_index: self.program_id_index,
            accounts: self.accounts.clone(),
            data: self.data.clone(),
        }
    }
}

impl ToSdk for Message {
    type Original = solana_sdk::message::Message;

    fn to_sdk(&self) -> Self::Original {
        solana_sdk::message::Message::new_with_compiled_instructions(
            self.header.num_required_signatures,
            self.header.num_readonly_signed_accounts,
            self.header.num_readonly_unsigned_accounts,
            self.account_keys.iter().map(Pubkey::to_sdk).collect(),
            self.recent_blockhash.to_sdk(),
            self.instructions.iter().map(|s| s.to_sdk()).collect(),
        )
    }
}

impl ToSdk for Transaction {
    type Original = solana_sdk::transaction::Transaction;

    fn to_sdk(&self) -> Self::Original {
        solana_sdk::transaction::Transaction {
            signatures: self
                .signatures
                .iter()
                .map(|s| solana_sdk::signature::Signature::new(s.as_ref()))
                .collect(),
            message: self.message.to_sdk(),
        }
    }
}

impl FromSdk for Clock {
    type Original = solana_program::sysvar::clock::Clock;

    fn from_sdk(sdk: &Self::Original) -> Self {
        Self {
            slot: sdk.slot,
            epoch_start_timestamp: sdk.epoch_start_timestamp,
            epoch: sdk.epoch,
            leader_schedule_epoch: sdk.leader_schedule_epoch,
            unix_timestamp: sdk.unix_timestamp,
        }
    }
}

impl FromSdk for Rent {
    type Original = solana_program::sysvar::rent::Rent;

    fn from_sdk(sdk: &Self::Original) -> Self {
        Self {
            lamports_per_byte_year: sdk.lamports_per_byte_year,
            exemption_threshold: sdk.exemption_threshold,
            burn_percent: sdk.burn_percent,
        }
    }
}

impl FromSdk for ProgramError {
    type Original = solana_program::program_error::ProgramError;

    fn from_sdk(sdk: &Self::Original) -> Self {
        Self::from(u64::from(sdk.clone()))
    }
}
