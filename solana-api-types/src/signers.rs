use crate::{
    pubkey::Pubkey,
    signature::{Signature, SignerError},
    Signer,
};

/// Convenience trait for working with mixed collections of `Signer`s
pub trait Signers {
    fn pubkeys(&self) -> Vec<Pubkey>;
    fn try_pubkeys(&self) -> Result<Vec<Pubkey>, SignerError>;
    fn sign_message(&self, message: &[u8]) -> Vec<Signature>;
    fn try_sign_message(&self, message: &[u8]) -> Result<Vec<Signature>, SignerError>;
}

macro_rules! default_keypairs_impl {
    () => {
        fn pubkeys(&self) -> Vec<Pubkey> {
            self.iter().map(|keypair| keypair.pubkey()).collect()
        }

        fn try_pubkeys(&self) -> Result<Vec<Pubkey>, SignerError> {
            let mut pubkeys = Vec::new();
            for keypair in self.iter() {
                pubkeys.push(keypair.try_pubkey()?);
            }
            Ok(pubkeys)
        }

        fn sign_message(&self, message: &[u8]) -> Vec<Signature> {
            self.iter()
                .map(|keypair| keypair.sign_message(message))
                .collect()
        }

        fn try_sign_message(&self, message: &[u8]) -> Result<Vec<Signature>, SignerError> {
            let mut signatures = Vec::new();
            for keypair in self.iter() {
                signatures.push(keypair.try_sign_message(message)?);
            }
            Ok(signatures)
        }
    };
}

impl<T: Signer> Signers for [T] {
    default_keypairs_impl!();
}

impl Signers for [Box<dyn Signer>] {
    default_keypairs_impl!();
}

impl Signers for Vec<Box<dyn Signer>> {
    default_keypairs_impl!();
}

impl<T: Signer, const N: usize> Signers for [T; N] {
    default_keypairs_impl!();
}

impl<T: Signer> Signers for Vec<T> {
    default_keypairs_impl!();
}

impl<T: Signers> Signers for &T {
    fn pubkeys(&self) -> Vec<Pubkey> {
        <T as Signers>::pubkeys(self)
    }

    fn try_pubkeys(&self) -> Result<Vec<Pubkey>, SignerError> {
        <T as Signers>::try_pubkeys(self)
    }

    fn sign_message(&self, message: &[u8]) -> Vec<Signature> {
        <T as Signers>::sign_message(self, message)
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Vec<Signature>, SignerError> {
        <T as Signers>::try_sign_message(self, message)
    }
}
