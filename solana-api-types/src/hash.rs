//! The `hash` module provides functions for creating SHA-256 hashes.
use std::convert::TryFrom;

#[cfg(feature = "offchain")]
use std::{fmt, mem, str::FromStr};
#[cfg(feature = "offchain")]
use thiserror::Error;

#[cfg(feature = "crypto")]
pub use hasher::*;

/// Amount of bytes in a hash.
pub const HASH_BYTES: usize = 32;
/// Maximum string length of a base58 encoded hash
pub const MAX_BASE58_LEN: usize = 44;

#[derive(Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "offchain", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Hash(pub [u8; HASH_BYTES]);

#[cfg(feature = "crypto")]
mod hasher {
    use sha2::{Digest, Sha256};

    use super::*;

    #[derive(Clone, Default)]
    pub struct Hasher {
        hasher: Sha256,
    }

    impl Hasher {
        pub fn hash(&mut self, val: &[u8]) {
            self.hasher.update(val);
        }
        pub fn hashv(&mut self, vals: &[&[u8]]) {
            for val in vals {
                self.hash(val);
            }
        }
        pub fn result(self) -> Hash {
            Hash(<[u8; HASH_BYTES]>::try_from(self.hasher.finalize().as_slice()).unwrap())
        }
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

#[cfg(feature = "offchain")]
impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

#[cfg(feature = "offchain")]
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[cfg(feature = "offchain")]
pub enum ParseHashError {
    #[error("string decoded to wrong size for hash")]
    WrongSize,
    #[error("failed to decoded string to hash")]
    Invalid,
}

#[cfg(feature = "offchain")]
impl FromStr for Hash {
    type Err = ParseHashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > MAX_BASE58_LEN {
            return Err(ParseHashError::WrongSize);
        }
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|_| ParseHashError::Invalid)?;
        if bytes.len() != mem::size_of::<Hash>() {
            Err(ParseHashError::WrongSize)
        } else {
            Ok(Hash::new(&bytes))
        }
    }
}

impl Hash {
    pub fn new(hash_slice: &[u8]) -> Self {
        Hash(<[u8; HASH_BYTES]>::try_from(hash_slice).unwrap())
    }

    pub const fn new_from_array(hash_array: [u8; HASH_BYTES]) -> Self {
        Self(hash_array)
    }

    /// unique Hash for tests and benchmarks.
    #[cfg(feature = "offchain")]
    pub fn new_unique() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static I: AtomicU64 = AtomicU64::new(1);

        let mut b = [0u8; HASH_BYTES];
        let i = I.fetch_add(1, Ordering::Relaxed);
        b[0..8].copy_from_slice(&i.to_le_bytes());
        Self::new(&b)
    }

    pub fn to_bytes(self) -> [u8; HASH_BYTES] {
        self.0
    }
}
