use super::{AccountBackend, AccountFields, Environment};

pub struct Offchain;

impl Environment for Offchain {
    fn supports_syscalls() -> bool {
        false
    }

    fn is_native() -> bool {
        true
    }
}

impl AccountFields for solana_api_types::Account {
    fn key(&self) -> &solana_api_types::Pubkey {
        &self.pubkey
    }

    fn owner(&self) -> &solana_api_types::Pubkey {
        &self.owner
    }

    fn is_signer(&self) -> bool {
        false
    }

    fn is_writable(&self) -> bool {
        false
    }

    fn is_executable(&self) -> bool {
        self.executable
    }

    fn lamports(&self) -> u64 {
        self.lamports
    }

    fn rent_epoch(&self) -> u64 {
        self.rent_epoch
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}

impl AccountBackend for &solana_api_types::Account {
    type Impl = solana_api_types::Account;
    type Env = Offchain;

    fn backend(&self) -> &Self::Impl {
        self
    }

    fn backend_mut(&mut self) -> &mut Self::Impl {
        unimplemented!()
    }
}

