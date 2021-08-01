//test
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;

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
    LegacyBinary(String), // Legacy. Retained for RPC backwards compatibility
    Json(ParsedAccount),
    Binary(String, UiAccountEncoding),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum UiAccountEncoding {
    Binary, // Legacy. Retained for RPC backwards compatibility
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
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
    pub data: Option<Value>,
}

struct AccountSliceConfig {
    offset: u64,
    length: u64,
}

// https://docs.solana.com/developing/clients/jsonrpc-api#filters
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
    // https://docs.solana.com/developing/clients/jsonrpc-api#getaccountinfo
    async fn get_account_info(
        &self,
        account: Pubkey,
        slice: Option<AccountSliceConfig>,
    ) -> Result<Account, Error>;

    // https://docs.solana.com/developing/clients/jsonrpc-api#getprogramaccounts
    async fn get_program_accounts(
        &self, 
        program: Pubkey, 
        slice: Option<AccountSliceConfig>, 
        filters: Option<&[AccountFilter]>
    ) -> Result<Vec<Account>, Error>;
    /*
       // https://docs.solana.com/developing/clients/jsonrpc-api#getmultipleaccounts
       async fn get_multiple_accounts(&self, accounts: &[Pubkey], slice: Option<AccountSliceConfig>) -> Result<Vec<Account>, Error>;

       // https://docs.solana.com/developing/clients/jsonrpc-api#getprogramaccounts
       async fn get_program_accounts(&self, program: Pubkey, slice: Option<AccountSliceConfig>, filters: Option<&[AccountFilter]>) -> Result<Vec<Account>, Error>;

       // https://docs.solana.com/developing/clients/jsonrpc-api#getsignaturestatuses
       async fn get_signature_statuses(&self, signatures: &[Signature], slice: Option<AccountSliceConfig>) -> Result<Vec<Account>, Error>;

       // https://docs.solana.com/developing/clients/jsonrpc-api#getsignaturesforaddress
       async fn get_signatures_for_address(&self, address: &Pubkey) -> Result<Vec<Account>, Error>;

       // https://docs.solana.com/developing/clients/jsonrpc-api#getslot
       async fn get_slot(&self, slice: Option<AccountSliceConfig>) -> u64;

       // https://docs.solana.com/developing/clients/jsonrpc-api#gettransaction
       async fn get_transaction(&self, program: Pubkey, commitment_config: CommitmentConfig, ) -> u64;

       // https://docs.solana.com/developing/clients/jsonrpc-api#requestairdrop
       async fn request_airdrop(&self, pubkey: &Pubkey, lamports: u64) -> u64;

       // https://docs.solana.com/developing/clients/jsonrpc-api#sendtransaction
       async fn send_transaction(&self, transaction: &Transaction) -> u64;

       // https://docs.solana.com/developing/clients/jsonrpc-api#simulatetransaction
       async fn simulate_transaction(&self, transaction: &Transaction,) -> u64;

    */
}

struct RpcClient {}

#[async_trait]
impl Client for RpcClient {
    async fn get_account_info(
        &self,
        account: Pubkey,
        slice: Option<AccountSliceConfig>,
    ) -> Result<Account, Error> {
        let json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [
                "2VWq8XTcDZBvi8v3i8RHonoPP9w74oNDqUeXJortxCZh",
                {
                    "encoding": "jsonParsed"
                }
            ]
        });

        let client = reqwest::Client::new();
        let response: serde_json::Value = client
            .post("https://api.devnet.solana.com")
            .json(&json)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        println!("{:#?}", response["result"]["value"]);

        let acc: Account = serde_json::from_value(response["result"]["value"].clone()).unwrap();

        println!("{:#?}", acc);

        serde_json::from_value(response["result"]["value"].clone()).unwrap()
    }

    async fn get_program_accounts (
        &self,
        account: Pubkey,
        slice: Option<AccountSliceConfig>,
        filters: Option<&[AccountFilter]>,
    ) -> Result<Vec<Account>, Error> {
        let json = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getProgramAccounts",
            "params": ["SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8"]
        });

        let client = reqwest::Client::new();
        let response: serde_json::Value = client
            .post("https://api.devnet.solana.com")
            .json(&json)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        // let res: Result<Vec<Account>, Error> = serde_json::from_value(response["result"].clone())
        //     .expect("some error");

        // for r in res.iter() {
        //     println!("{:?}", r);
        // }

        // return res;

        serde_json::from_value(response["result"].clone()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Client, RpcClient};
    use serde::Serialize;
    use serde_json::Value;
    use solana_sdk::account::Account;

    #[tokio::test]
    async fn get_account_info_test() {
        let rpc_client = RpcClient {};
        let arr = bs58::decode("2VWq8XTcDZBvi8v3i8RHonoPP9w74oNDqUeXJortxCZh")
            .into_vec()
            .unwrap();
        let account = solana_sdk::pubkey::Pubkey::new(&arr);
        let response = rpc_client.get_account_info(account, None).await;
        println!("{:?}", response);
    }

    #[tokio::test]
    async fn get_program_accounts_test() {
        let rpc_client = RpcClient {};
        let arr = bs58::decode("SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8")
            .into_vec()
            .unwrap();
        let account = solana_sdk::pubkey::Pubkey::new(&arr);
        let response = rpc_client.get_program_accounts(account, None, None).await;
        println!("{:#?}", response);
    }
}
