use solana_program::pubkey::Pubkey;

pub mod offchain;
pub mod onchain;

/// A trait for abstracting over the underlying account storage type, which can be
/// different for on-chain and off-chain logic.
pub trait AccountFields {
    fn key(&self) -> &Pubkey;
    fn owner(&self) -> &Pubkey;
    fn is_signer(&self) -> bool;
    fn is_writable(&self) -> bool;
    fn is_executable(&self) -> bool;
    fn lamports(&self) -> u64;
    fn rent_epoch(&self) -> u64;
    fn data(&self) -> &[u8];
}

pub trait AccountFieldsMut: AccountFields {
    fn set_lamports(&mut self, value: u64);
    fn data_mut(&mut self) -> &mut [u8];
}

pub trait AccountBackend {
    type Impl: AccountFields;

    fn backend(&self) -> &Self::Impl;

    fn backend_mut(&mut self) -> &mut Self::Impl;
}

impl<T> AccountFields for T
where
    T: AccountBackend,
{
    fn key(&self) -> &Pubkey {
        self.backend().key()
    }

    fn owner(&self) -> &Pubkey {
        self.backend().owner()
    }

    fn is_signer(&self) -> bool {
        self.backend().is_signer()
    }

    fn is_writable(&self) -> bool {
        self.backend().is_writable()
    }

    fn is_executable(&self) -> bool {
        self.backend().is_executable()
    }

    fn lamports(&self) -> u64 {
        self.backend().lamports()
    }

    fn rent_epoch(&self) -> u64 {
        self.backend().rent_epoch()
    }

    fn data(&self) -> &[u8] {
        self.backend().data()
    }
}

impl<T> AccountFieldsMut for T
where
    T: AccountBackend,
    T::Impl: AccountFieldsMut,
{
    fn set_lamports(&mut self, value: u64) {
        self.backend_mut().set_lamports(value)
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self.backend_mut().data_mut()
    }
}

#[macro_export]
macro_rules! forward_account_backend {
    ($t:ident, $f:ident) => {
        impl<B> AccountBackend for $t<B>
        where
            B: AccountBackend,
        {
            type Impl = <B as AccountBackend>::Impl;

            fn backend(&self) -> &Self::Impl {
                self.$f.backend()
            }

            fn backend_mut(&mut self) -> &mut Self::Impl {
                self.$f.backend_mut()
            }
        }
    };
}
