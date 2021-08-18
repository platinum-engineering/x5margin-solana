use solana_api_types::Account;

use super::{AccountBackend, AccountFields, AccountFieldsMut, Environment};

pub struct Offchain;

impl Environment for Offchain {
    fn supports_syscalls() -> bool {
        false
    }

    fn is_native() -> bool {
        true
    }
}

impl AccountFields for Account {
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
        self.data.as_slice()
    }
}

impl AccountFieldsMut for Account {
    fn set_lamports(&mut self, value: u64) {
        self.lamports = value;
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self.data.as_mut_slice()
    }
}

// `Box` here b/c we want to allow some indirection to implement
// trait but cannot afford to have lifetimes -- wasm-bindgen can't do that yet.
impl AccountBackend for Box<Account> {
    type Impl = Account;
    type Env = Offchain;

    fn backend(&self) -> &Self::Impl {
        self
    }

    fn backend_mut(&mut self) -> &mut Self::Impl {
        self
    }
}

