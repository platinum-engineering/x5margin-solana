use std::{
    convert::TryFrom,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize};
use wasm_bindgen::prelude::*;

use solana_api_types::{Client, ClientError, Pubkey, RpcError, RpcResponse, UiAccount};

struct SolanaApiClient {
    client: reqwest::Client,
    current_id: AtomicUsize,
    solana_api_url: &'static str,
}

struct Request {
    method: &'static str,
    params: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: i64,
    result: RpcResponse<T>,
}

impl SolanaApiClient {
    async fn mk_request<T: DeserializeOwned>(&self, r: Request) -> Result<T, ClientError> {
        let id = self.current_id.fetch_add(1, Ordering::SeqCst);

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": r.method,
            "params": r.params,
        });
        let request = serde_json::to_vec(&request)?;

        let r = self
            .client
            .post(self.solana_api_url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(request)
            .send()
            .await?;

        let body = r.bytes().await?;
        let body: JsonRpcResponse<T> = serde_json::from_slice(&body)?;

        Ok(body.result.value)
    }
}

#[async_trait(?Send)]
impl Client for SolanaApiClient {
    async fn get_account_info(
        &self,
        account: solana_api_types::Pubkey,
        cfg: Option<solana_api_types::RpcAccountInfoConfig>,
    ) -> Result<solana_api_types::Account, solana_api_types::ClientError> {
        let account: UiAccount = self
            .mk_request(Request {
                method: "getAccountInfo",
                params: serde_json::json!([account.to_string(), serde_json::to_value(&cfg)?,]),
            })
            .await?;

        let account = account
            .decode()
            .ok_or_else(|| RpcError::ParseError("failed to decode account".to_string()))?;

        Ok(account)
    }

    async fn get_program_accounts(
        &self,
        program: solana_api_types::Pubkey,
        cfg: Option<solana_api_types::RpcProgramAccountsConfig>,
    ) -> Result<Vec<solana_api_types::RpcKeyedAccount>, solana_api_types::ClientError> {
        todo!()
    }

    async fn get_multiple_accounts(
        &self,
        accounts: &[solana_api_types::Pubkey],
        cfg: Option<solana_api_types::RpcAccountInfoConfig>,
    ) -> Result<Vec<solana_api_types::Account>, solana_api_types::ClientError> {
        todo!()
    }

    async fn get_signature_statuses(
        &self,
        signatures: &[solana_api_types::Signature],
        cfg: solana_api_types::RpcSignatureStatusConfig,
    ) -> Result<Vec<solana_api_types::Account>, solana_api_types::ClientError> {
        todo!()
    }

    async fn get_signatures_for_address(
        &self,
        address: &solana_api_types::Pubkey,
        cfg: solana_api_types::RpcSignaturesForAddressConfig,
    ) -> Result<Vec<solana_api_types::SignatureInfo>, solana_api_types::ClientError> {
        todo!()
    }

    async fn get_slot(
        &self,
        cfg: Option<solana_api_types::RpcSlotConfig>,
    ) -> Result<solana_api_types::Slot, solana_api_types::ClientError> {
        todo!()
    }

    async fn get_transaction(
        &self,
        signature: solana_api_types::Signature,
        cfg: Option<solana_api_types::RpcTransactionConfig>,
    ) -> Result<Option<solana_api_types::EncodedConfirmedTransaction>, solana_api_types::ClientError>
    {
        todo!()
    }

    async fn request_airdrop(
        &self,
        pubkey: &solana_api_types::Pubkey,
        lamports: u64,
        commitment: Option<solana_api_types::CommitmentConfig>,
    ) -> Result<solana_api_types::Signature, solana_api_types::ClientError> {
        todo!()
    }

    async fn send_transaction(
        &self,
        transaction: &solana_api_types::Transaction,
        cfg: solana_api_types::RpcSendTransactionConfig,
    ) -> Result<solana_api_types::Signature, solana_api_types::ClientError> {
        todo!()
    }

    async fn simulate_transaction(
        &self,
        transaction: &solana_api_types::Transaction,
        cfg: solana_api_types::RpcSimulateTransactionConfig,
    ) -> Result<solana_api_types::RpcSimulateTransactionResult, solana_api_types::ClientError> {
        todo!()
    }
}

#[wasm_bindgen]
pub async fn run() -> Result<JsValue, JsValue> {
    console_log::init().unwrap();

    let client = SolanaApiClient {
        client: reqwest::Client::new(),
        current_id: AtomicUsize::new(0),
        solana_api_url: "https://api.devnet.solana.com",
    };

    let pubkey = Pubkey::try_from("4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA").unwrap();
    let r = client.get_account_info(pubkey, None).await;
    let r = match r {
        Ok(a) => JsValue::from_serde(&a).unwrap(),
        Err(err) => {
            let err = format!("{:?}", err);
            JsValue::from_str(&err)
        }
    };

    Ok(r)
}
