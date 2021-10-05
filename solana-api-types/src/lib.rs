#![allow(clippy::nonstandard_macro_braces)]

#[cfg(feature = "offchain")]
#[macro_use]
extern crate serde;

#[cfg(feature = "debug")]
#[macro_use]
extern crate thiserror;

// Modules that are available under all features.

pub mod entrypoint;
pub mod hash;
pub mod instruction;
pub mod program;
pub mod pubkey;
pub mod syscalls;
pub mod system;
pub mod sysvar;

pub use hash::Hash;
pub use instruction::{Instruction, InstructionError};
pub use pubkey::Pubkey;

// Modules that are only available when using the Solana SDK bridge.

#[cfg(feature = "runtime-test")]
pub mod program_test;
#[cfg(feature = "runtime-test")]
pub mod sdk_proxy;

#[cfg(feature = "crypto")]
pub use key::Keypair;

// Modules that are only available when executing in an offchain environment.

#[cfg(feature = "offchain")]
pub mod client;
#[cfg(feature = "offchain")]
pub mod error;
#[cfg(feature = "offchain")]
pub mod key;
#[cfg(feature = "offchain")]
pub mod message;
#[cfg(feature = "offchain")]
pub mod short_vec;
#[cfg(feature = "offchain")]
pub mod signature;
#[cfg(feature = "offchain")]
pub mod signers;
#[cfg(feature = "offchain")]
pub mod transaction;
#[cfg(feature = "offchain")]
pub use error::{ClientError, JsonValueParseError};
#[cfg(feature = "offchain")]
pub use instruction::CompiledInstruction;
#[cfg(feature = "offchain")]
pub use key::Signer;
#[cfg(feature = "offchain")]
pub use message::Message;
#[cfg(feature = "offchain")]
pub use signature::{Signature, SignerError};
#[cfg(feature = "offchain")]
pub use signers::Signers;
#[cfg(feature = "offchain")]
pub use transaction::{
    ConfirmedTransaction, ConfirmedTransactionMetadata, Transaction, TransactionError,
    TransactionStatus, TransactionSummary,
};

/// Epoch is a unit of time a given leader schedule is honored,
///  some number of Slots.
pub type Epoch = u64;

/// Slot is a unit of time given to a leader for encoding,
///  is some some number of Ticks long.
pub type Slot = u64;

/// UnixTimestamp is an approximate measure of real-world time,
/// expressed as Unix time (ie. seconds since the Unix epoch)
pub type UnixTimestamp = i64;

#[cfg(feature = "offchain")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// An attribute of a slot. It describes how finalized a block is at some point in time. For example, a slot
/// is said to be at the max level immediately after the cluster recognizes the block at that slot as
/// finalized. When querying the ledger state, use lower levels of commitment to report progress and higher
/// levels to ensure state changes will not be rolled back.
pub enum CommitmentLevel {
    /// The highest slot of the heaviest fork processed by the node. Ledger state at this slot is
    /// not derived from a confirmed or finalized block, but if multiple forks are present, is from
    /// the fork the validator believes is most likely to finalize.
    Processed,

    /// The highest slot that has been voted on by supermajority of the cluster, ie. is confirmed.
    /// Confirmation incorporates votes from gossip and replay. It does not count votes on
    /// descendants of a block, only direct votes on that block, and upholds "optimistic
    /// confirmation" guarantees in release 1.3 and onwards.
    Confirmed,

    /// The highest slot having reached max vote lockout, as recognized by a supermajority of the
    /// cluster.
    Finalized,
}

#[cfg(feature = "offchain")]
impl Default for CommitmentLevel {
    fn default() -> Self {
        Self::Finalized
    }
}

#[cfg(feature = "offchain")]
impl CommitmentLevel {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "processed" => Self::Processed,
            "confirmed" => Self::Confirmed,
            "finalized" => Self::Finalized,
            _ => return None,
        })
    }
}

/// Account metadata used to define Instructions
#[derive(PartialEq, Clone)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[cfg_attr(feature = "offchain", derive(Serialize, Deserialize))]
pub struct AccountMeta {
    /// An account's public key
    pub pubkey: Pubkey,
    /// True if an Instruction requires a Transaction signature matching `pubkey`.
    pub is_signer: bool,
    /// True if the `pubkey` can be loaded as a read-write account.
    pub is_writable: bool,
}

impl AccountMeta {
    pub fn new(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }

    pub fn new_readonly(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }
}

/// An off-chain representation of a Solana account.
///
/// A Solana account is a container of data associated with a specific public key (address).
///
/// An account is created as soon as lamports are deposited to a specific address, after which the account will be stored in the ledger.
///
/// Accounts can be expanded to serve as containers of arbitrary binary data by using the System program instructions.
#[derive(PartialEq, Eq, Clone, Default)]
#[cfg(feature = "offchain")]
pub struct Account {
    /// Lamport balance of the account. 1 lamport = 10^-9 SOL.
    ///
    /// Lamports are used to pay "rent" once per epoch. The size of the rent is dependent on the size of the account.
    ///
    /// There exists a rent-exemption balance threshold, which, if exceeded, stops the account from owing rent. Currently this threshold is 7.12 SOL per Mebibyte, or 6709 lamports per byte.
    pub lamports: u64,
    /// Arbitrary associated account data. May be empty.
    pub data: Vec<u8>,
    /// The program that owns this account. If this account is an executable program, the loader that executes this program.
    ///
    /// Only the program that owns this account is allowed to modify its data, and debit its balance.
    /// Non-owner programs have read-only access.
    ///
    /// When an account is initially created, its owner is set to the System program.
    /// The System program can then transfer ownership of the account to another program, granting it write permissions for that account.
    pub owner: Pubkey,
    /// Whether this account is an executable program.
    pub executable: bool,
    /// The epoch at which this account will next owe rent.
    pub rent_epoch: Epoch,
    /// The public key of the account.
    pub pubkey: Pubkey,
}
