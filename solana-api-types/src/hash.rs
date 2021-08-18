//! The `hash` module provides functions for creating SHA-256 hashes.
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt, mem, str::FromStr};
use thiserror::Error;

#[cfg(feature = "crypto")]
pub use hasher::*;

pub const HASH_BYTES: usize = 32;
/// Maximum string length of a base58 encoded hash
const MAX_BASE58_LEN: usize = 44;
#[derive(Serialize, Deserialize, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
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
            // At the time of this writing, the sha2 library is stuck on an old version
            // of generic_array (0.9.0). Decouple ourselves with a clone to our version.
            Hash(<[u8; HASH_BYTES]>::try_from(self.hasher.finalize().as_slice()).unwrap())
        }
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseHashError {
    #[error("string decoded to wrong size for hash")]
    WrongSize,
    #[error("failed to decoded string to hash")]
    Invalid,
}

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
