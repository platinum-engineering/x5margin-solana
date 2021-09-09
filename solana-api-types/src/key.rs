use crate::{signature::SignerError, Pubkey, Signature};

#[cfg(feature = "crypto")]
mod crypto_imports {
    pub use ed25519_dalek::Signer as _;
    pub use rand::{rngs::OsRng, CryptoRng, RngCore};
    pub use std::convert::TryInto;
}

#[cfg(feature = "crypto")]
use crypto_imports::*;

#[cfg(feature = "crypto")]
/// A vanilla Ed25519 key pair
#[derive(Debug)]
pub struct Keypair(ed25519_dalek::Keypair);

/// The `Signer` trait declares operations that all digital signature providers
/// must support. It is the primary interface by which signers are specified in
/// `Transaction` signing interfaces
pub trait Signer {
    /// Infallibly gets the implementor's public key. Returns the all-zeros
    /// `Pubkey` if the implementor has none.
    fn pubkey(&self) -> Pubkey {
        self.try_pubkey().unwrap_or_default()
    }
    /// Fallibly gets the implementor's public key
    fn try_pubkey(&self) -> Result<Pubkey, SignerError>;
    /// Infallibly produces an Ed25519 signature over the provided `message`
    /// bytes. Returns the all-zeros `Signature` if signing is not possible.
    fn sign_message(&self, message: &[u8]) -> Signature {
        self.try_sign_message(message).unwrap_or_default()
    }
    /// Fallibly produces an Ed25519 signature over the provided `message` bytes.
    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError>;
}

#[cfg(feature = "crypto")]
impl Keypair {
    /// Constructs a new, random `Keypair` using a caller-proveded RNG
    pub fn generate<R>(csprng: &mut R) -> Self
    where
        R: CryptoRng + RngCore,
    {
        Self(ed25519_dalek::Keypair::generate(csprng))
    }

    /// Constructs a new, random `Keypair` using `OsRng`
    pub fn new() -> Self {
        let mut rng = OsRng::default();
        Self::generate(&mut rng)
    }

    /// Recovers a `Keypair` from a byte array
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ed25519_dalek::SignatureError> {
        ed25519_dalek::Keypair::from_bytes(bytes).map(Self)
    }

    /// Returns this `Keypair` as a byte array
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0.to_bytes()
    }

    /// Recovers a `Keypair` from a base58-encoded string
    pub fn from_base58_string(s: &str) -> Self {
        Self::from_bytes(&bs58::decode(s).into_vec().unwrap()).unwrap()
    }

    /// Returns this `Keypair` as a base58-encoded string
    pub fn to_base58_string(&self) -> String {
        bs58::encode(&self.0.to_bytes()).into_string()
    }

    /// Gets this `Keypair`'s SecretKey
    pub fn secret(&self) -> &ed25519_dalek::SecretKey {
        &self.0.secret
    }
}

#[cfg(feature = "crypto")]
impl Signer for Keypair {
    fn pubkey(&self) -> Pubkey {
        Pubkey::new(self.0.public.as_ref().try_into().expect("infallible"))
    }

    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        Ok(self.pubkey())
    }

    fn sign_message(&self, message: &[u8]) -> Signature {
        Signature::new(self.0.sign(message).to_bytes())
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        Ok(self.sign_message(message))
    }
}

#[cfg(feature = "crypto")]
impl<T: AsRef<Keypair>> Signer for T {
    fn pubkey(&self) -> Pubkey {
        self.as_ref().pubkey()
    }

    fn try_pubkey(&self) -> Result<Pubkey, SignerError> {
        self.as_ref().try_pubkey()
    }

    fn sign_message(&self, message: &[u8]) -> Signature {
        self.as_ref().sign_message(message)
    }

    fn try_sign_message(&self, message: &[u8]) -> Result<Signature, SignerError> {
        self.as_ref().try_sign_message(message)
    }
}
