use std::{
    convert::{TryFrom, TryInto},
    fmt,
    str::FromStr,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// maximum length of derived `Pubkey` seed
pub const MAX_SEED_LEN: usize = 32;
/// Maximum number of seeds
pub const MAX_SEEDS: usize = 16;

const PDA_MARKER: &[u8; 21] = b"ProgramDerivedAddress";

#[repr(transparent)]
#[derive(Serialize, Deserialize, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    pub const fn new(pubkey: [u8; 32]) -> Self {
        Self(pubkey)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[cfg(feature = "extended")]
    pub fn new_unique() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static I: AtomicU64 = AtomicU64::new(1);

        let mut b = [0u8; 32];
        let i = I.fetch_add(1, Ordering::Relaxed);
        b[0..8].copy_from_slice(&i.to_le_bytes());
        Self::new(b)
    }

    #[cfg(any(feature = "extended", target_arch = "bpf"))]
    pub fn create_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Option<Pubkey> {
        if seeds.len() > MAX_SEEDS {
            return None;
        }

        for seed in seeds.iter() {
            if seed.len() > MAX_SEED_LEN {
                return None;
            }
        }

        #[cfg(not(target_arch = "bpf"))]
        {
            let mut hasher = crate::hash::Hasher::default();
            for seed in seeds.iter() {
                hasher.hash(seed);
            }
            hasher.hashv(&[program_id.as_ref(), PDA_MARKER]);
            let hash = hasher.result();
            let pk = Pubkey(hash.0);

            if pk.is_on_curve() {
                return None;
            }

            Some(Pubkey(hash.0))
        }

        #[cfg(target_arch = "bpf")]
        {
            extern "C" {
                fn sol_create_program_address(
                    seeds_addr: *const u8,
                    seeds_len: u64,
                    program_id_addr: *const u8,
                    address_bytes_addr: *const u8,
                ) -> u64;
            }
            let mut bytes = [0; 32];
            let result = unsafe {
                sol_create_program_address(
                    seeds as *const _ as *const u8,
                    seeds.len() as u64,
                    program_id as *const _ as *const u8,
                    &mut bytes as *mut _ as *mut u8,
                )
            };
            match result {
                crate::entrypoint::SUCCESS => Some(Pubkey(bytes)),
                _ => None,
            }
        }
    }

    #[cfg(feature = "extended")]
    pub fn is_on_curve(&self) -> bool {
        curve25519_dalek::edwards::CompressedEdwardsY::from_slice(self.0.as_ref())
            .decompress()
            .is_some()
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
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
            Ok(Pubkey::new(pubkey_vec.try_into().unwrap()))
        }
    }
}

impl TryFrom<&str> for Pubkey {
    type Error = ParsePubkeyError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Pubkey::from_str(s)
    }
}
