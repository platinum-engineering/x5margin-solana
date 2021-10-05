use std::{convert::TryInto, fmt, str::FromStr};

use serde::{
    de::{Error, Visitor},
    ser::SerializeTuple,
    Deserialize, Serialize,
};
use thiserror::Error;

use crate::TransactionError;

/// Maximum string length of a base58 encoded signature
const MAX_BASE58_SIGNATURE_LEN: usize = 88;

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// An Ed25519 digital signature.
///
/// In the Solana runtime, signatures signal to the runtime that some action was authorized for a specific account by the holder of its private key.
///
/// The presence of a signature in a transaction for a specific account results in the `is_signer` field being set during program execution for that account, letting the program know that the account owner
/// "signed off" on this operation.
pub struct Signature([u8; 64]);

impl Default for Signature {
    fn default() -> Self {
        Self([0; 64])
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut array = serializer.serialize_tuple(64)?;
        for i in self.0.iter() {
            array.serialize_element(i)?;
        }
        array.end()
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ArrayVisitor;

        impl<'de> Visitor<'de> for ArrayVisitor {
            type Value = [u8; 64];

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "sequence of 64 byte values")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                if let Some(len) = seq.size_hint() {
                    if len != 64 {
                        return Err(A::Error::invalid_length(len, &"64"));
                    }
                }

                let mut array = [0u8; 64];
                for (len, i) in array.iter_mut().enumerate() {
                    *i = seq
                        .next_element::<u8>()?
                        .ok_or_else(|| A::Error::invalid_length(len, &"64"))?;
                }

                Ok(array)
            }
        }

        let array = deserializer.deserialize_tuple(64, ArrayVisitor)?;
        Ok(Self(array))
    }
}

impl Signature {
    pub const fn new(bytes: [u8; 64]) -> Signature {
        Signature(bytes)
    }

    pub const fn as_array(&self) -> &[u8; 64] {
        &self.0
    }

    #[cfg(feature = "crypto")]
    pub(self) fn verify_verbose(
        &self,
        pubkey_bytes: &[u8],
        message_bytes: &[u8],
    ) -> Result<(), ed25519_dalek::SignatureError> {
        let publickey = ed25519_dalek::PublicKey::from_bytes(pubkey_bytes)?;
        let signature = self.0.into();
        publickey.verify_strict(message_bytes, &signature)
    }

    #[cfg(feature = "crypto")]
    pub fn verify(&self, pubkey_bytes: &[u8], message_bytes: &[u8]) -> bool {
        self.verify_verbose(pubkey_bytes, message_bytes).is_ok()
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Signature({})", bs58::encode(self.0).into_string())
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseSignatureError {
    #[error("string decoded to wrong size for signature")]
    WrongSize,
    #[error("failed to decode string to signature")]
    Invalid,
}

impl FromStr for Signature {
    type Err = ParseSignatureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > MAX_BASE58_SIGNATURE_LEN {
            return Err(ParseSignatureError::WrongSize);
        }
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|_| ParseSignatureError::Invalid)?;
        if bytes.len() != 64 {
            Err(ParseSignatureError::WrongSize)
        } else {
            Ok(Signature::new(bytes.try_into().expect("infallible")))
        }
    }
}

impl std::convert::TryFrom<&str> for Signature {
    type Error = ParseSignatureError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Signature::from_str(s)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum PresignerError {
    #[error("pre-generated signature cannot verify data")]
    VerificationFailure,
}

#[derive(Debug, Error, PartialEq)]
pub enum SignerError {
    #[error("keypair-pubkey mismatch")]
    KeypairPubkeyMismatch,

    #[error("not enough signers")]
    NotEnoughSigners,

    #[error("transaction error")]
    TransactionError(#[from] TransactionError),

    #[error("custom error: {0}")]
    Custom(String),

    // Presigner-specific Errors
    #[error("presigner error")]
    PresignerError(#[from] PresignerError),

    // Remote Keypair-specific Errors
    #[error("connection error: {0}")]
    Connection(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("no device found")]
    NoDeviceFound,

    #[error("{0}")]
    Protocol(String),

    #[error("{0}")]
    UserCancel(String),
}
