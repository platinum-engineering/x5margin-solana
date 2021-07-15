use solana_program::pubkey::Pubkey;

pub mod offchain;
pub mod onchain;

/// A trait for abstracting over the underlying account storage type, which will be
/// different for on-chain and off-chain logic.
pub trait AccountBackend {
    fn key(&self) -> &Pubkey;
    fn owner(&self) -> &Pubkey;
    fn is_signer(&self) -> bool;
    fn is_writable(&self) -> bool;
    fn is_executable(&self) -> bool;
    fn lamports(&self) -> u64;
    fn rent_epoch(&self) -> u64;
    fn data(&self) -> &[u8];
}

impl<T: AccountBackend> AccountBackend for &T {
    fn key(&self) -> &Pubkey {
        (*self).key()
    }

    fn owner(&self) -> &Pubkey {
        (*self).owner()
    }

    fn is_signer(&self) -> bool {
        (*self).is_signer()
    }

    fn is_writable(&self) -> bool {
        (*self).is_writable()
    }

    fn is_executable(&self) -> bool {
        (*self).is_executable()
    }

    fn lamports(&self) -> u64 {
        (*self).lamports()
    }

    fn rent_epoch(&self) -> u64 {
        (*self).rent_epoch()
    }

    fn data(&self) -> &[u8] {
        (*self).data()
    }
}

impl<T: AccountBackend> AccountBackend for &mut T {
    fn key(&self) -> &Pubkey {
        (**self).key()
    }

    fn owner(&self) -> &Pubkey {
        (**self).owner()
    }

    fn is_signer(&self) -> bool {
        (**self).is_signer()
    }

    fn is_writable(&self) -> bool {
        (**self).is_writable()
    }

    fn is_executable(&self) -> bool {
        (**self).is_executable()
    }

    fn lamports(&self) -> u64 {
        (**self).lamports()
    }

    fn rent_epoch(&self) -> u64 {
        (**self).rent_epoch()
    }

    fn data(&self) -> &[u8] {
        (**self).data()
    }
}

pub trait AccountBackendMut: AccountBackend {
    fn set_lamports(&mut self, value: u64);
    fn data_mut(&mut self) -> &mut [u8];
}

impl<T: AccountBackendMut> AccountBackendMut for &mut T {
    fn set_lamports(&mut self, value: u64) {
        (*self).set_lamports(value);
    }

    fn data_mut(&mut self) -> &mut [u8] {
        (*self).data_mut()
    }
}
