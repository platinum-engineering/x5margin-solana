use std::{convert::TryFrom, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[repr(transparent)]
#[derive(Serialize, Deserialize, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    pub fn new(pubkey_vec: &[u8]) -> Self {
        Self(
            <[u8; 32]>::try_from(<&[u8]>::clone(&pubkey_vec))
                .expect("Slice must be the same length as a Pubkey"),
        )
    }
}

impl std::fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

#[derive(Error, Debug, Serialize, Clone, PartialEq)]
pub enum ParsePubkeyError {
    #[error("String is the wrong size")]
    WrongSize,
    #[error("Invalid Base58 string")]
    Invalid,
}

/// Maximum string length of a base58 encoded pubkey
const MAX_BASE58_LEN: usize = 44;

impl FromStr for Pubkey {
    type Err = ParsePubkeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > MAX_BASE58_LEN {
            return Err(ParsePubkeyError::WrongSize);
        }
        let pubkey_vec = bs58::decode(s)
            .into_vec()
            .map_err(|_| ParsePubkeyError::Invalid)?;
        if pubkey_vec.len() != std::mem::size_of::<Pubkey>() {
            Err(ParsePubkeyError::WrongSize)
        } else {
            Ok(Pubkey::new(&pubkey_vec))
        }
    }
}

impl TryFrom<&str> for Pubkey {
    type Error = ParsePubkeyError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Pubkey::from_str(s)
    }
}
