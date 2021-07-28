use async_trait::async_trait;
use serde_json::Value;
use solana_sdk::{
    account::Account, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};

use serde::{Deserialize, Serialize};

pub type Epoch = u64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParsedAccount {
    pub program: String,
    pub parsed: Value,
    pub space: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UiAccount {
    pub lamports: u64,
    pub data: UiAccountData,
    pub owner: String,
    pub executable: bool,
    pub rent_epoch: Epoch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum UiAccountData {
    /// Legacy. Retained for RPC backwards compatibility
    LegacyBinary(String),
    Json(ParsedAccount),
    Binary(String, UiAccountEncoding),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UiAccountEncoding {
    /// Legacy. Retained for RPC backwards compatibility
    Binary,
    Base58,
    Base64,
    JsonParsed,
    #[serde(rename = "base64+zstd")]
    Base64Zstd,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    ServerError(i64),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CommitmentConfig {
    Finalized,
    Confirmed,
    Processed,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    pub data: Option<Value>,
}

struct AccountSliceConfig {
    offset: u64,
    length: u64,
}

/// https://docs.solana.com/developing/clients/jsonrpc-api#filters
struct Memcmp {
    offset: u64,
    bytes: String,
}

struct AccountFilter {
    data_size: u64,
    memcmp: Memcmp,
}

#[async_trait]
trait Client {
    /// https://docs.solana.com/developing/clients/jsonrpc-api#getaccountinfo
    async fn get_account_info(
        &self,
        account: Pubkey,
        slice: Option<AccountSliceConfig>,
    ) -> Result<Account, Error>;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#getprogramaccounts
    async fn get_program_accounts(
        &self,
        program: Pubkey,
        slice: Option<AccountSliceConfig>,
        filters: Option<&[AccountFilter]>,
    ) -> Result<Vec<Account>, Error>;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#getmultipleaccounts
    async fn get_multiple_accounts(
        &self,
        accounts: &[Pubkey],
        slice: Option<AccountSliceConfig>,
    ) -> Result<Vec<Account>, Error>;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#getsignaturestatuses
    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
        slice: Option<AccountSliceConfig>,
    ) -> Result<Vec<Account>, Error>;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#getsignaturesforaddress
    async fn get_signatures_for_address(&self, address: &Pubkey) -> Result<Vec<Account>, Error>;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#getslot
    async fn get_slot(&self, slice: Option<AccountSliceConfig>) -> u64;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#gettransaction
    async fn get_transaction(
        &self,
        signature: Signature,
        commitment_config: CommitmentConfig,
    ) -> u64;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#requestairdrop
    async fn request_airdrop(&self, pubkey: &Pubkey, lamports: u64) -> u64;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#sendtransaction
    async fn send_transaction(&self, transaction: &Transaction) -> u64;

    /// https://docs.solana.com/developing/clients/jsonrpc-api#simulatetransaction
    async fn simulate_transaction(&self, transaction: &Transaction) -> u64;
}
