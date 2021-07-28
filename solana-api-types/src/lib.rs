pub type Epoch = u64;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub program: String,
    pub parsed: Value,
    pub space: u64,
}

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
