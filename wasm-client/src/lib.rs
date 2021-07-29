use std::{
    convert::TryFrom,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use wasm_bindgen::prelude::*;

use solana_api_types::{Client, ClientError, Pubkey};

struct SolanaApiClient {
    client: reqwest::Client,
    current_id: AtomicUsize,
    solana_api_url: &'static str,
}

struct Request {
    method: &'static str,
    params: serde_json::Value,
}

impl SolanaApiClient {
    async fn mk_request(&self, r: Request) -> Result<serde_json::Value, ClientError> {
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
        let body = serde_json::from_slice(&body)?;

        Ok(body)
    }
}

#[async_trait(?Send)]
impl Client for SolanaApiClient {
    async fn get_account_info(
        &self,
        account: solana_api_types::Pubkey,
        cfg: Option<solana_api_types::RpcAccountInfoConfig>,
    ) -> Result<solana_api_types::Account, solana_api_types::ClientError> {
        let r = self
            .mk_request(Request {
                method: "getAccountInfo",
                params: serde_json::json!([
                    // serde_json::to_value(&account)?,
                    "4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA",
                    serde_json::to_value(&cfg)?,
                ]),
            })
            .await?;

        let account = serde_json::from_value(r)?;

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
    ) -> Result<String, solana_api_types::ClientError> {
        todo!()
    }

    async fn send_transaction(
        &self,
        transaction: &solana_api_types::Transaction,
        cfg: solana_api_types::RpcSendTransactionConfig,
    ) -> Result<String, solana_api_types::ClientError> {
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
    let client = SolanaApiClient {
        client: reqwest::Client::new(),
        current_id: AtomicUsize::new(0),
        solana_api_url: "https://api.devnet.solana.com",
    };

    let pubkey = Pubkey::try_from("4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA").unwrap();
    let r = client.get_account_info(pubkey, None).await;
    let r = match r {
        Ok(a) => String::from("account"),
        Err(err) => format!("{:?}", err),
    };

    Ok(JsValue::from_serde(&r).unwrap())
}
