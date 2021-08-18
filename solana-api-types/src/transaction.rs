use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    error::ClientErrorKind, short_vec, signature::SignerError, ClientError, CompiledInstruction,
    Instruction, InstructionError, Message, Pubkey, Signature, Signers, Slot,
    UiTransactionEncoding,
};

use super::Hash;

/// Reasons a transaction might be rejected.
#[derive(Error, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum TransactionError {
    /// An account is already being processed in another transaction in a way
    /// that does not support parallelism
    #[error("Account in use")]
    AccountInUse,

    /// A `Pubkey` appears twice in the transaction's `account_keys`.  Instructions can reference
    /// `Pubkey`s more than once but the message must contain a list with no duplicate keys
    #[error("Account loaded twice")]
    AccountLoadedTwice,

    /// Attempt to debit an account but found no record of a prior credit.
    #[error("Attempt to debit an account but found no record of a prior credit.")]
    AccountNotFound,

    /// Attempt to load a program that does not exist
    #[error("Attempt to load a program that does not exist")]
    ProgramAccountNotFound,

    /// The from `Pubkey` does not have sufficient balance to pay the fee to schedule the transaction
    #[error("Insufficient funds for fee")]
    InsufficientFundsForFee,

    /// This account may not be used to pay transaction fees
    #[error("This account may not be used to pay transaction fees")]
    InvalidAccountForFee,

    /// The bank has seen this transaction before. This can occur under normal operation
    /// when a UDP packet is duplicated, as a user error from a client not updating
    /// its `recent_blockhash`, or as a double-spend attack.
    #[error("This transaction has already been processed")]
    AlreadyProcessed,

    /// The bank has not seen the given `recent_blockhash` or the transaction is too old and
    /// the `recent_blockhash` has been discarded.
    #[error("Blockhash not found")]
    BlockhashNotFound,

    /// An error occurred while processing an instruction. The first element of the tuple
    /// indicates the instruction index in which the error occurred.
    #[error("Error processing Instruction {0}: {1}")]
    InstructionError(u8, InstructionError),

    /// Loader call chain is too deep
    #[error("Loader call chain is too deep")]
    CallChainTooDeep,

    /// Transaction requires a fee but has no signature present
    #[error("Transaction requires a fee but has no signature present")]
    MissingSignatureForFee,

    /// Transaction contains an invalid account reference
    #[error("Transaction contains an invalid account reference")]
    InvalidAccountIndex,

    /// Transaction did not pass signature verification
    #[error("Transaction did not pass signature verification")]
    SignatureFailure,

    /// This program may not be used for executing instructions
    #[error("This program may not be used for executing instructions")]
    InvalidProgramForExecution,

    /// Transaction failed to sanitize accounts offsets correctly
    /// implies that account locks are not taken for this TX, and should
    /// not be unlocked.
    #[error("Transaction failed to sanitize accounts offsets correctly")]
    SanitizeFailure,

    #[error("Transactions are currently disabled due to cluster maintenance")]
    ClusterMaintenance,

    /// Transaction processing left an account with an outstanding borrowed reference
    #[error("Transaction processing left an account with an outstanding borrowed reference")]
    AccountBorrowOutstanding,
}

pub type Result<T> = std::result::Result<T, TransactionError>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransactionConfirmationStatus {
    Processed,
    Confirmed,
    Finalized,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStatus {
    pub slot: Slot,
    pub confirmations: Option<usize>, // None = rooted
    pub status: Result<()>,           // legacy field
    pub err: Option<TransactionError>,
    pub confirmation_status: Option<TransactionConfirmationStatus>,
}

/// An atomic transaction
#[derive(Debug, PartialEq, Default, Eq, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// A set of digital signatures of `account_keys`, `program_ids`, `recent_blockhash`, and `instructions`, signed by the first
    /// signatures.len() keys of account_keys
    /// NOTE: Serialization-related changes must be paired with the direct read at sigverify.
    #[serde(with = "short_vec")]
    pub signatures: Vec<Signature>,

    /// The message to sign.
    pub message: Message,
}

impl Transaction {
    pub fn encode(
        &self,
        encoding: UiTransactionEncoding,
    ) -> std::result::Result<String, ClientError> {
        let serialized = bincode::serialize(self).map_err(|e| {
            ClientErrorKind::Custom(format!("transaction serialization failed: {}", e))
        })?;
        let encoded = match encoding {
            UiTransactionEncoding::Base58 => bs58::encode(serialized).into_string(),
            UiTransactionEncoding::Base64 => base64::encode(serialized),
            _ => {
                return Err(ClientErrorKind::Custom(format!(
                    "unsupported transaction encoding: {}. Supported encodings: base58, base64",
                    encoding
                ))
                .into())
            }
        };
        Ok(encoded)
    }

    pub fn new_unsigned(message: Message) -> Self {
        Self {
            signatures: vec![Signature::default(); message.header.num_required_signatures as usize],
            message,
        }
    }

    pub fn new_with_payer(instructions: &[Instruction], payer: Option<&Pubkey>) -> Self {
        let message = Message::new(instructions, payer);
        Self::new_unsigned(message)
    }

    /// Return a message containing all data that should be signed.
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Return the serialized message data to sign.
    pub fn message_data(&self) -> Vec<u8> {
        self.message().serialize()
    }

    pub fn is_signed(&self) -> bool {
        self.signatures
            .iter()
            .all(|signature| *signature != Signature::default())
    }

    /// Create a signed transaction with the given payer.
    ///
    /// # Panics
    ///
    /// Panics when signing fails.
    pub fn new_signed_with_payer<T: Signers>(
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signing_keypairs: &T,
        recent_blockhash: Hash,
    ) -> Self {
        let message = Message::new(instructions, payer);
        Self::new(signing_keypairs, message, recent_blockhash)
    }

    /// Create a signed transaction.
    ///
    /// # Panics
    ///
    /// Panics when signing fails.
    pub fn new<T: Signers>(
        from_keypairs: &T,
        message: Message,
        recent_blockhash: Hash,
    ) -> Transaction {
        let mut tx = Self::new_unsigned(message);
        tx.sign(from_keypairs, recent_blockhash);
        tx
    }

    /// Create a signed transaction
    /// * `from_keypairs` - The keys used to sign the transaction.
    /// * `keys` - The keys for the transaction.  These are the program state
    ///    instances or lamport recipient keys.
    /// * `recent_blockhash` - The PoH hash.
    /// * `program_ids` - The keys that identify programs used in the `instruction` vector.
    /// * `instructions` - Instructions that will be executed atomically.
    ///
    /// # Panics
    ///
    /// Panics when signing fails.
    pub fn new_with_compiled_instructions<T: Signers>(
        from_keypairs: &T,
        keys: &[Pubkey],
        recent_blockhash: Hash,
        program_ids: Vec<Pubkey>,
        instructions: Vec<CompiledInstruction>,
    ) -> Self {
        let mut account_keys = from_keypairs.pubkeys();
        let from_keypairs_len = account_keys.len();
        account_keys.extend_from_slice(keys);
        account_keys.extend(&program_ids);
        let message = Message::new_with_compiled_instructions(
            from_keypairs_len as u8,
            0,
            program_ids.len() as u8,
            account_keys,
            Hash::default(),
            instructions,
        );
        Transaction::new(from_keypairs, message, recent_blockhash)
    }

    /// Check keys and keypair lengths, then sign this transaction.
    ///
    /// # Panics
    ///
    /// Panics when signing fails, use [`Transaction::try_sign`] to handle the error.
    pub fn sign<T: Signers>(&mut self, keypairs: &T, recent_blockhash: Hash) {
        if let Err(e) = self.try_sign(keypairs, recent_blockhash) {
            panic!("Transaction::sign failed with error {:?}", e);
        }
    }

    /// Sign using some subset of required keys
    ///  if recent_blockhash is not the same as currently in the transaction,
    ///  clear any prior signatures and update recent_blockhash
    ///
    /// # Panics
    ///
    /// Panics when signing fails, use [`Transaction::try_partial_sign`] to handle the error.
    pub fn partial_sign<T: Signers>(&mut self, keypairs: &T, recent_blockhash: Hash) {
        if let Err(e) = self.try_partial_sign(keypairs, recent_blockhash) {
            panic!("Transaction::partial_sign failed with error {:?}", e);
        }
    }

    /// Sign the transaction and place the signatures in their associated positions in `signatures`
    /// without checking that the positions are correct.
    ///
    /// # Panics
    ///
    /// Panics when signing fails, use [`Transaction::try_partial_sign_unchecked`] to handle the error.
    pub fn partial_sign_unchecked<T: Signers>(
        &mut self,
        keypairs: &T,
        positions: Vec<usize>,
        recent_blockhash: Hash,
    ) {
        if let Err(e) = self.try_partial_sign_unchecked(keypairs, positions, recent_blockhash) {
            panic!(
                "Transaction::partial_sign_unchecked failed with error {:?}",
                e
            );
        }
    }

    /// Check keys and keypair lengths, then sign this transaction, returning any signing errors
    /// encountered
    pub fn try_sign<T: Signers>(
        &mut self,
        keypairs: &T,
        recent_blockhash: Hash,
    ) -> std::result::Result<(), SignerError> {
        self.try_partial_sign(keypairs, recent_blockhash)?;

        if !self.is_signed() {
            Err(SignerError::NotEnoughSigners)
        } else {
            Ok(())
        }
    }

    ///  Sign using some subset of required keys, returning any signing errors encountered. If
    ///  recent_blockhash is not the same as currently in the transaction, clear any prior
    ///  signatures and update recent_blockhash
    pub fn try_partial_sign<T: Signers>(
        &mut self,
        keypairs: &T,
        recent_blockhash: Hash,
    ) -> std::result::Result<(), SignerError> {
        let positions = self.get_signing_keypair_positions(&keypairs.pubkeys())?;
        if positions.iter().any(|pos| pos.is_none()) {
            return Err(SignerError::KeypairPubkeyMismatch);
        }
        let positions: Vec<usize> = positions.iter().map(|pos| pos.unwrap()).collect();
        self.try_partial_sign_unchecked(keypairs, positions, recent_blockhash)
    }

    /// Sign the transaction, returning any signing errors encountered, and place the
    /// signatures in their associated positions in `signatures` without checking that the
    /// positions are correct.
    pub fn try_partial_sign_unchecked<T: Signers>(
        &mut self,
        keypairs: &T,
        positions: Vec<usize>,
        recent_blockhash: Hash,
    ) -> std::result::Result<(), SignerError> {
        // if you change the blockhash, you're re-signing...
        if recent_blockhash != self.message.recent_blockhash {
            self.message.recent_blockhash = recent_blockhash;
            self.signatures
                .iter_mut()
                .for_each(|signature| *signature = Signature::default());
        }

        let signatures = keypairs.try_sign_message(&self.message_data())?;
        for i in 0..positions.len() {
            self.signatures[positions[i]] = signatures[i];
        }
        Ok(())
    }

    /// Get the positions of the pubkeys in `account_keys` associated with signing keypairs
    pub fn get_signing_keypair_positions(&self, pubkeys: &[Pubkey]) -> Result<Vec<Option<usize>>> {
        if self.message.account_keys.len() < self.message.header.num_required_signatures as usize {
            return Err(TransactionError::InvalidAccountIndex);
        }
        let signed_keys =
            &self.message.account_keys[0..self.message.header.num_required_signatures as usize];

        Ok(pubkeys
            .iter()
            .map(|pubkey| signed_keys.iter().position(|x| x == pubkey))
            .collect())
    }

    /// Verify the transaction
    pub fn verify(&self) -> Result<()> {
        let message_bytes = self.message_data();
        if !self
            ._verify_with_results(&message_bytes)
            .iter()
            .all(|verify_result| *verify_result)
        {
            Err(TransactionError::SignatureFailure)
        } else {
            Ok(())
        }
    }

    pub fn verify_with_results(&self) -> Vec<bool> {
        self._verify_with_results(&self.message_data())
    }

    pub(crate) fn _verify_with_results(&self, message_bytes: &[u8]) -> Vec<bool> {
        self.signatures
            .iter()
            .zip(&self.message.account_keys)
            .map(|(signature, pubkey)| signature.verify(pubkey.as_ref(), message_bytes))
            .collect()
    }
}
