use solana_api_types::Pubkey;

use super::{AccountBackend, AccountFields, Environment};

/// [`Environment`] implementation for [`PubkeyAccount`]. Supports nothing.
pub struct PubkeyEnvironment;

impl Environment for PubkeyEnvironment {
    fn supports_syscalls() -> bool {
        false
    }

    fn is_native() -> bool {
        false
    }
}

/// Simple AccountBackend implementation which only knows about its own Pubkey, and nothing else.
///
/// Useful for off-chain use-cases where loading the entire account isn't necessary, and for on-chain Authority checks.
pub struct PubkeyAccount {
    pubkey: Pubkey,
}

impl From<Pubkey> for PubkeyAccount {
    fn from(pubkey: Pubkey) -> Self {
        Self { pubkey }
    }
}

impl AccountFields for Pubkey {
    fn key(&self) -> &Pubkey {
        self
    }

    fn owner(&self) -> &Pubkey {
        unimplemented!("PubkeyAccount does not support `owner()`")
    }

    fn is_signer(&self) -> bool {
        false
    }

    fn is_writable(&self) -> bool {
        false
    }

    fn is_executable(&self) -> bool {
        false
    }

    fn lamports(&self) -> u64 {
        unimplemented!("PubkeyAccount does not support `lamports()`")
    }

    fn rent_epoch(&self) -> u64 {
        unimplemented!("PubkeyAccount does not support `rent_epoch()`")
    }

    fn data(&self) -> &[u8] {
        unimplemented!("PubkeyAccount does not support `data()`")
    }
}

impl AccountBackend for PubkeyAccount {
    type Impl = Pubkey;

    type Env = PubkeyEnvironment;

    fn backend(&self) -> &Self::Impl {
        &self.pubkey
    }

    fn backend_mut(&mut self) -> &mut Self::Impl {
        &mut self.pubkey
    }
}
