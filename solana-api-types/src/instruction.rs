use crate::{AccountMeta, Pubkey};

/// Various errors that can occur during instruction execution.
#[derive(PartialEq, Eq, Clone)]
#[cfg_attr(feature = "offchain", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "debug", derive(Debug, Error))]

pub enum InstructionError {
    /// *DEPRECATED* The program instruction returned an error.
    #[cfg_attr(feature = "debug", error("generic instruction error"))]
    GenericError,

    /// The arguments provided to a program were invalid
    #[cfg_attr(feature = "debug", error("invalid program argument"))]
    InvalidArgument,

    /// An instruction's data contents were invalid
    #[cfg_attr(feature = "debug", error("invalid instruction data"))]
    InvalidInstructionData,

    /// An account's data contents was invalid
    #[cfg_attr(feature = "debug", error("invalid account data for instruction"))]
    InvalidAccountData,

    /// An account's data was too small
    #[cfg_attr(feature = "debug", error("account data too small for instruction"))]
    AccountDataTooSmall,

    /// An account's balance was too small to complete the instruction
    #[cfg_attr(feature = "debug", error("insufficient funds for instruction"))]
    InsufficientFunds,

    /// The account did not have the expected program id
    #[cfg_attr(feature = "debug", error("incorrect program id for instruction"))]
    IncorrectProgramId,

    /// A signature was required but not found
    #[cfg_attr(feature = "debug", error("missing required signature for instruction"))]
    MissingRequiredSignature,

    /// An initialize instruction was sent to an account that has already been initialized.
    #[cfg_attr(
        feature = "debug",
        error("instruction requires an uninitialized account")
    )]
    AccountAlreadyInitialized,

    /// An attempt to operate on an account that hasn't been initialized.
    #[cfg_attr(
        feature = "debug",
        error("instruction requires an initialized account")
    )]
    UninitializedAccount,

    /// Program's instruction lamport balance does not equal the balance after the instruction
    #[cfg_attr(
        feature = "debug",
        error("sum of account balances before and after instruction do not match")
    )]
    UnbalancedInstruction,

    /// Program modified an account's program id
    #[cfg_attr(
        feature = "debug",
        error("instruction modified the program id of an account")
    )]
    ModifiedProgramId,

    /// Program spent the lamports of an account that doesn't belong to it
    #[cfg_attr(
        feature = "debug",
        error("instruction spent from the balance of an account it does not own")
    )]
    ExternalAccountLamportSpend,

    /// Program modified the data of an account that doesn't belong to it
    #[cfg_attr(
        feature = "debug",
        error("instruction modified data of an account it does not own")
    )]
    ExternalAccountDataModified,

    /// Read-only account's lamports modified
    #[cfg_attr(
        feature = "debug",
        error("instruction changed the balance of a read-only account")
    )]
    ReadonlyLamportChange,

    /// Read-only account's data was modified
    #[cfg_attr(
        feature = "debug",
        error("instruction modified data of a read-only account")
    )]
    ReadonlyDataModified,

    /// An account was referenced more than once in a single instruction
    // Deprecated, instructions can now contain duplicate accounts
    #[cfg_attr(feature = "debug", error("instruction contains duplicate accounts"))]
    DuplicateAccountIndex,

    /// Executable bit on account changed, but shouldn't have
    #[cfg_attr(
        feature = "debug",
        error("instruction changed executable bit of an account")
    )]
    ExecutableModified,

    /// Rent_epoch account changed, but shouldn't have
    #[cfg_attr(
        feature = "debug",
        error("instruction modified rent epoch of an account")
    )]
    RentEpochModified,

    /// The instruction expected additional account keys
    #[cfg_attr(feature = "debug", error("insufficient account keys for instruction"))]
    NotEnoughAccountKeys,

    /// A non-system program changed the size of the account data
    #[cfg_attr(
        feature = "debug",
        error("non-system instruction changed account size")
    )]
    AccountDataSizeChanged,

    /// The instruction expected an executable account
    #[cfg_attr(feature = "debug", error("instruction expected an executable account"))]
    AccountNotExecutable,

    /// Failed to borrow a reference to account data, already borrowed
    #[cfg_attr(
        feature = "debug",
        error("instruction tries to borrow reference for an account which is already borrowed")
    )]
    AccountBorrowFailed,

    /// Account data has an outstanding reference after a program's execution
    #[cfg_attr(
        feature = "debug",
        error("instruction left account with an outstanding borrowed reference")
    )]
    AccountBorrowOutstanding,

    /// The same account was multiply passed to an on-chain program's entrypoint, but the program
    /// modified them differently.  A program can only modify one instance of the account because
    /// the runtime cannot determine which changes to pick or how to merge them if both are modified
    #[cfg_attr(
        feature = "debug",
        error("instruction modifications of multiply-passed account differ")
    )]
    DuplicateAccountOutOfSync,

    /// Allows on-chain programs to implement program-specific error types and see them returned
    /// by the Solana runtime. A program-specific error may be any type that is represented as
    /// or serialized to a u32 integer.
    #[cfg_attr(feature = "debug", error("custom program error: {0:#x}"))]
    Custom(u32),

    /// The return value from the program was invalid.  Valid errors are either a defined builtin
    /// error value or a user-defined error in the lower 32 bits.
    #[cfg_attr(feature = "debug", error("program returned invalid error code"))]
    InvalidError,

    /// Executable account's data was modified
    #[cfg_attr(
        feature = "debug",
        error("instruction changed executable accounts data")
    )]
    ExecutableDataModified,

    /// Executable account's lamports modified
    #[cfg_attr(
        feature = "debug",
        error("instruction changed the balance of a executable account")
    )]
    ExecutableLamportChange,

    /// Executable accounts must be rent exempt
    #[cfg_attr(feature = "debug", error("executable accounts must be rent exempt"))]
    ExecutableAccountNotRentExempt,

    /// Unsupported program id
    #[cfg_attr(feature = "debug", error("Unsupported program id"))]
    UnsupportedProgramId,

    /// Cross-program invocation call depth too deep
    #[cfg_attr(
        feature = "debug",
        error("Cross-program invocation call depth too deep")
    )]
    CallDepth,

    /// An account required by the instruction is missing
    #[cfg_attr(
        feature = "debug",
        error("An account required by the instruction is missing")
    )]
    MissingAccount,

    /// Cross-program invocation reentrancy not allowed for this instruction
    #[cfg_attr(
        feature = "debug",
        error("Cross-program invocation reentrancy not allowed for this instruction")
    )]
    ReentrancyNotAllowed,

    /// Length of the seed is too long for address generation
    #[cfg_attr(
        feature = "debug",
        error("Length of the seed is too long for address generation")
    )]
    MaxSeedLengthExceeded,

    /// Provided seeds do not result in a valid address
    #[cfg_attr(
        feature = "debug",
        error("Provided seeds do not result in a valid address")
    )]
    InvalidSeeds,

    /// Failed to reallocate account data of this length
    #[cfg_attr(feature = "debug", error("Failed to reallocate account data"))]
    InvalidRealloc,

    /// Computational budget exceeded
    #[cfg_attr(feature = "debug", error("Computational budget exceeded"))]
    ComputationalBudgetExceeded,

    /// Cross-program invocation with unauthorized signer or writable account
    #[cfg_attr(
        feature = "debug",
        error("Cross-program invocation with unauthorized signer or writable account")
    )]
    PrivilegeEscalation,

    /// Failed to create program execution AccountBackend
    #[cfg_attr(
        feature = "debug",
        error("Failed to create program execution AccountBackend")
    )]
    ProgramAccountBackendSetupFailure,

    /// Program failed to complete
    #[cfg_attr(feature = "debug", error("Program failed to complete"))]
    ProgramFailedToComplete,

    /// Program failed to compile
    #[cfg_attr(feature = "debug", error("Program failed to compile"))]
    ProgramFailedToCompile,

    /// Account is immutable
    #[cfg_attr(feature = "debug", error("Account is immutable"))]
    Immutable,

    /// Incorrect authority provided
    #[cfg_attr(feature = "debug", error("Incorrect authority provided"))]
    IncorrectAuthority,

    /// Failed to serialize or deserialize account data
    #[cfg_attr(
        feature = "debug",
        error("Failed to serialize or deserialize account data: {0}")
    )]
    BorshIoError(String),

    /// An account does not have enough lamports to be rent-exempt
    #[cfg_attr(
        feature = "debug",
        error("An account does not have enough lamports to be rent-exempt")
    )]
    AccountNotRentExempt,

    /// Invalid account owner
    #[cfg_attr(feature = "debug", error("Invalid account owner"))]
    InvalidAccountOwner,

    /// Program arithmetic overflowed
    #[cfg_attr(feature = "debug", error("Program arithmetic overflowed"))]
    ArithmeticOverflow,

    /// Unsupported sysvar
    #[cfg_attr(feature = "debug", error("Unsupported sysvar"))]
    UnsupportedSysvar,

    /// Illegal account owner
    #[cfg_attr(feature = "debug", error("Provided owner is not allowed"))]
    IllegalOwner,
}

/// An instruction for an on-chain Solana program.
///
/// An instruction encapsulates the input parameters of a program invocation,
/// like the environment and arguments passed to a `main` handler in native programs.
///
/// Programs will typically implement some form of dispatch for sub-instructions.
#[derive(PartialEq, Clone)]
#[cfg_attr(feature = "debug", derive(Debug))]
#[cfg_attr(feature = "offchain", derive(Serialize, Deserialize))]
pub struct Instruction {
    /// Pubkey of the program that will be executed.
    pub program_id: Pubkey,
    /// List of accounts to be passed to the program, with read/write/signer information.
    pub accounts: Vec<AccountMeta>,
    /// Raw byte buffer given as input to the program. Different programs will use their own formats for this field.
    pub data: Vec<u8>,
}

impl Instruction {
    #[cfg(feature = "offchain")]
    pub fn new_with_bincode<T: serde::Serialize>(
        program_id: Pubkey,
        data: &T,
        accounts: Vec<AccountMeta>,
    ) -> Self {
        let data = bincode::serialize(data).unwrap();
        Self {
            program_id,
            accounts,
            data,
        }
    }

    pub fn new_with_bytes(program_id: Pubkey, data: &[u8], accounts: Vec<AccountMeta>) -> Self {
        Self {
            program_id,
            accounts,
            data: data.to_vec(),
        }
    }
}

/// A 'compiled' form of an instruction, as it appears within a transaction.
///
/// You usually won't need to use this.
#[cfg(feature = "offchain")]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompiledInstruction {
    /// Index into the transaction keys array indicating the program account that executes this instruction
    pub program_id_index: u8,
    /// Ordered indices into the transaction keys array indicating which accounts to pass to the program
    #[serde(with = "crate::short_vec")]
    pub accounts: Vec<u8>,
    /// The program input data
    #[serde(with = "crate::short_vec")]
    pub data: Vec<u8>,
}
