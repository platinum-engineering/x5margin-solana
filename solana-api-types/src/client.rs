use super::*;
use async_trait::async_trait;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccountSlice {
    pub offset: usize,
    pub length: usize,
}

impl AccountSlice {
    pub fn to_json_value(&self) -> Value {
        json!({"offset": self.offset, "length": self.length})
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemcmpFilter<'a> {
    /// Offset to match the provided bytes from.
    pub offset: usize,
    /// Bytes, encoded with specified encoding, or default Binary
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccountFilter<'a> {
    /// Filters accounts by their total size.
    DataSize(u64),
    /// Filters accounts by a region of their data.
    Memcmp(MemcmpFilter<'a>),
}

impl<'a> AccountFilter<'a> {
    pub fn to_json_value(&self) -> Value {
        match self {
            AccountFilter::DataSize(length) => json!({ "dataSize": length }),
            AccountFilter::Memcmp(MemcmpFilter { offset, bytes }) => {
                json!({"memcmp": {"offset": offset, "bytes": bs58::encode(bytes).into_string()}})
            }
        }
    }
}

#[ async_trait(?Send)]
pub trait Client {
    /// Get the default commitment level configured for this [`Client`] instance.
    fn default_commitment_level(&self) -> CommitmentLevel;

    /// Set the default commitment level for this [`Client`] instance. This commitment will be used when a `None` commitment is provided to a method.
    fn set_default_commitment_level(&self, level: CommitmentLevel);

    /// Get the provided account, optionally resized to the provided offset and length.
    async fn get_account_info(
        &self,
        account: &Pubkey,
        slice: Option<AccountSlice>,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<Account>, ClientError>;

    /// Get all accounts owned by the provided program, optionally filtered and sliced.
    async fn get_program_accounts_ex(
        &self,
        program: &Pubkey,
        filters: Option<&[AccountFilter]>,
        slice: Option<AccountSlice>,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Vec<Account>, ClientError>;

    /// Get multiple accounts, optionally resized to the profided offset and length.
    async fn get_multiple_accounts(
        &self,
        accounts: &[Pubkey],
        slice: Option<AccountSlice>,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Vec<Option<Account>>, ClientError>;

    /// Gets the statuses of multiple transactions, finding them by their signatures.
    ///
    /// If the `search_history` parameter is `true`, requests the RPC node to search through transactions that are not in the current transaction cache.
    async fn get_transaction_statuses(
        &self,
        signatures: &[Signature],
        search_history: bool,
    ) -> Result<Vec<Option<TransactionStatus>>, ClientError>;

    /// Gets all transactions in which the specified account appeared.
    ///
    /// The options `before` and `until` respectively specify the start and the end of the search query.
    ///
    /// `limit` limits the amount of transactions returned.
    async fn get_transactions_for_account(
        &self,
        account: &Pubkey,
        before: Option<&Signature>,
        until: Option<&Signature>,
        limit: u64,
    ) -> Result<Vec<TransactionSummary>, ClientError>;

    /// Gets the current slot for the provided (or default) commitment level.
    async fn get_slot(&self, commitment: Option<CommitmentLevel>) -> Result<Slot, ClientError>;

    /// Gets the information about a specific signature. The transaction has to be in "confirmed" or "finalized" confirmation level to be visible in this endpoint.
    async fn get_transaction(
        &self,
        signature: Signature,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<ConfirmedTransaction>, ClientError>;

    /// Gets a recent blockhash, usable in transactions for the `recent_blockhash` value.
    async fn get_recent_blockhash(
        &self,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Hash, ClientError>;

    /// Requests an airdrop of lamports to the specified account. Only works if the cluster has a faucet for delivering SOL - this is either a local dev cluster, devnet or testnet. Not available on mainnet.
    async fn request_airdrop(
        &self,
        pubkey: &Pubkey,
        lamports: u64,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Signature, ClientError>;

    /// Sends the transaction to the cluster. Doesn't wait for confirmation of the transaction. Doesn't skip preflight checks.
    async fn send_transaction(&self, transaction: &Transaction) -> Result<Signature, ClientError> {
        self.send_transaction_ex(transaction, false, None).await
    }

    /// Sends the transaction to the cluster. Doesn't wait for confirmation of the transaction.
    ///
    /// "Preflight", if enabled, tests the transaction against the RPC node's current ledger state at the specified commitment level, before submitting the transaction to the network.
    ///
    /// This is useful to avoid wasting SOL on transaction fees if the transaction would immediately fail. Note that even if the transaction passes the preflight check, it might still fail on the network because of diverging ledger state. Contrariwise, some transactions might fail on the preflight check, but succeed if actually submitted to the network.
    ///
    /// Rule of thumb: enable preflight checks for BPF programs, disable them for built-ins (System program, BPF Loader).
    ///
    /// Also note that if the transaction fails the preflight check, it won't appear in the explorer, which might complicate debugging on non-local clusters.
    async fn send_transaction_ex(
        &self,
        transaction: &Transaction,
        skip_preflight: bool,
        preflight_commitment: Option<CommitmentLevel>,
    ) -> Result<Signature, ClientError>;

    // Simulates the transaction on the RPC node's ledger, without submitting it to the network and without spending any SOL on transaction fees. The result of the transaction is discarded and not commited to the ledger state.
    //
    // `sig_verify`, if false, will skip signature verification on the transaction.
    //
    // `replace_recnet_blockhash`, if true, will replace the `recent_blockhash` field on the transaction before executing it.
    //
    // async fn simulate_transaction(
    //     &self,
    //     transaction: &Transaction,
    //     sig_verify: bool,
    //     commitment: Option<CommitmentLevel>,
    //     replace_recent_blockhash: bool,
    // ) -> Result<RpcSimulateTransactionResult, ClientError>;
}
