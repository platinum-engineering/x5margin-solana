use std::{
    convert::TryFrom,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use futures::{Future, TryFutureExt};
use js_sys::Promise;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::{convert::FromWasmAbi, prelude::*};
use wasm_bindgen_futures::{future_to_promise, JsFuture};

use solana_api_types::{
    Client, ClientError, CommitmentConfig, EncodedConfirmedTransaction, Message, Pubkey,
    RpcAccountInfoConfig, RpcError, RpcKeyedAccount, RpcResponse, RpcSendTransactionConfig,
    RpcSignatureStatusConfig, RpcSignaturesForAddressConfig, RpcSimulateTransactionConfig,
    RpcSimulateTransactionResult, RpcSlotConfig, RpcTransactionConfig, Signature, SignatureInfo,
    Slot, Transaction, TransactionStatus, UiAccount, UiAccountEncoding, UiTransactionEncoding,
};

struct SolanaApiClient {
    client: reqwest::Client,
    current_id: AtomicUsize,
    solana_api_url: &'static str,
}

impl SolanaApiClient {
    fn new(solana_api_url: &'static str) -> Self {
        Self {
            client: reqwest::Client::new(),
            current_id: AtomicUsize::new(0),
            solana_api_url,
        }
    }

    fn devnet() -> Self {
        Self::new("https://api.devnet.solana.com")
    }

    fn dupe(&self) -> Self {
        let id = self.current_id.fetch_add(1, Ordering::SeqCst);
        Self {
            client: self.client.clone(),
            current_id: AtomicUsize::new(id),
            solana_api_url: self.solana_api_url,
        }
    }
}

struct Request {
    method: &'static str,
    params: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: i64,
    result: T,
}

impl SolanaApiClient {
    async fn mk_request<T: DeserializeOwned>(&self, r: Request) -> Result<T, ClientError> {
        let id = self.current_id.fetch_add(1, Ordering::SeqCst);

        log::info!("{}", id);

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
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        log::info!("{}", body);
        let body: JsonRpcResponse<T> = serde_json::from_value(body)?;

        Ok(body.result)
    }
}

#[async_trait(?Send)]
impl Client for SolanaApiClient {
    async fn get_account_info(
        &self,
        account: Pubkey,
        cfg: Option<solana_api_types::RpcAccountInfoConfig>,
    ) -> Result<solana_api_types::Account, solana_api_types::ClientError> {
        let r: RpcResponse<UiAccount> = self
            .mk_request(Request {
                method: "getAccountInfo",
                params: serde_json::json!([account.to_string(), serde_json::to_value(&cfg)?,]),
            })
            .await?;

        let account = r
            .value
            .decode(account)
            .ok_or_else(|| RpcError::ParseError("failed to decode account".to_string()))?;

        Ok(account)
    }

    async fn get_program_accounts(
        &self,
        program: Pubkey,
        cfg: Option<solana_api_types::RpcProgramAccountsConfig>,
    ) -> Result<Vec<solana_api_types::Account>, solana_api_types::ClientError> {
        let r: Vec<RpcKeyedAccount> = self
            .mk_request(Request {
                method: "getProgramAccounts",
                params: serde_json::json!([program.to_string(), serde_json::to_value(&cfg)?,]),
            })
            .await?;

        let r = r
            .into_iter()
            .filter_map(|a| {
                let pubkey = Pubkey::from_str(a.pubkey.as_str()).ok()?;
                a.account.decode(pubkey)
            })
            .collect();

        Ok(r)
    }

    async fn get_multiple_accounts(
        &self,
        accounts: &[Pubkey],
        cfg: Option<solana_api_types::RpcAccountInfoConfig>,
    ) -> Result<Vec<solana_api_types::Account>, solana_api_types::ClientError> {
        let accounts_as_str: Vec<String> = accounts.iter().map(|a| a.to_string()).collect();
        let r: RpcResponse<Vec<Option<UiAccount>>> = self
            .mk_request(Request {
                method: "getMultipleAccounts",
                params: serde_json::json!([accounts_as_str, serde_json::to_value(&cfg)?,]),
            })
            .await?;

        let r = r
            .value
            .into_iter()
            .zip(accounts)
            .filter_map(|(acc, key)| acc?.decode(*key))
            .collect();

        Ok(r)
    }

    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
        cfg: Option<solana_api_types::RpcSignatureStatusConfig>,
    ) -> Result<Vec<Option<TransactionStatus>>, solana_api_types::ClientError> {
        let signatures: Vec<String> = signatures.iter().map(|s| s.to_string()).collect();
        let r: RpcResponse<Vec<Option<TransactionStatus>>> = self
            .mk_request(Request {
                method: "getSignatureStatuses",
                params: serde_json::json!([signatures, serde_json::to_value(&cfg)?,]),
            })
            .await?;

        Ok(r.value)
    }

    async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        cfg: Option<RpcSignaturesForAddressConfig>,
    ) -> Result<Vec<SignatureInfo>, solana_api_types::ClientError> {
        let r: Vec<SignatureInfo> = self
            .mk_request(Request {
                method: "getSignaturesForAddress",
                params: serde_json::json!([address.to_string(), serde_json::to_value(&cfg)?]),
            })
            .await?;

        Ok(r)
    }

    async fn get_slot(
        &self,
        cfg: Option<solana_api_types::RpcSlotConfig>,
    ) -> Result<Slot, solana_api_types::ClientError> {
        let r: Slot = self
            .mk_request(Request {
                method: "getSlot",
                params: serde_json::json!([serde_json::to_value(&cfg)?]),
            })
            .await?;

        Ok(r)
    }

    async fn get_transaction(
        &self,
        signature: Signature,
        cfg: Option<solana_api_types::RpcTransactionConfig>,
    ) -> Result<Option<EncodedConfirmedTransaction>, solana_api_types::ClientError> {
        let r: Option<EncodedConfirmedTransaction> = self
            .mk_request(Request {
                method: "getTransaction",
                params: serde_json::json!([signature.to_string(), serde_json::to_value(&cfg)?]),
            })
            .await?;

        Ok(r)
    }

    async fn request_airdrop(
        &self,
        pubkey: &Pubkey,
        lamports: u64,
        commitment: Option<solana_api_types::CommitmentConfig>,
    ) -> Result<Signature, solana_api_types::ClientError> {
        let r: String = self
            .mk_request(Request {
                method: "requestAirdrop",
                params: serde_json::json!([
                    pubkey.to_string(),
                    lamports,
                    serde_json::to_value(&commitment)?
                ]),
            })
            .await?;

        let signature =
            Signature::from_str(&r).map_err(|err| RpcError::ParseError(err.to_string()))?;

        Ok(signature)
    }

    async fn send_transaction(
        &self,
        transaction: &solana_api_types::Transaction,
        cfg: RpcSendTransactionConfig,
    ) -> Result<Signature, solana_api_types::ClientError> {
        let encoding = cfg.encoding.unwrap_or_default();
        let preflight_commitment = cfg.preflight_commitment.unwrap_or_default();

        let cfg = RpcSendTransactionConfig {
            preflight_commitment: Some(preflight_commitment),
            encoding: Some(encoding),
            ..cfg
        };

        let transaction = transaction.encode(encoding)?;

        let r: String = self
            .mk_request(Request {
                method: "sendTransaction",
                params: serde_json::json!([transaction, serde_json::to_value(&cfg)?]),
            })
            .await?;

        let signature =
            Signature::from_str(&r).map_err(|err| RpcError::ParseError(err.to_string()))?;

        Ok(signature)
    }

    async fn simulate_transaction(
        &self,
        transaction: &solana_api_types::Transaction,
        cfg: solana_api_types::RpcSimulateTransactionConfig,
    ) -> Result<solana_api_types::RpcSimulateTransactionResult, solana_api_types::ClientError> {
        let encoding = cfg.encoding.unwrap_or_default();
        let commitment = cfg.commitment.unwrap_or_default();
        let cfg = RpcSimulateTransactionConfig {
            commitment: Some(commitment),
            encoding: Some(encoding),
            ..cfg
        };

        let transaction = transaction.encode(encoding)?;
        let r: RpcResponse<RpcSimulateTransactionResult> = self
            .mk_request(Request {
                method: "simulateTransaction",
                params: serde_json::json!([transaction, serde_json::to_value(&cfg)?]),
            })
            .await?;

        Ok(r.value)
    }
}

#[wasm_bindgen]
pub struct ApiClient {
    inner: SolanaApiClient,
}

fn return_promise(fut: impl Future<Output = Result<JsValue, ClientError>> + 'static) -> Promise {
    future_to_promise(fut.map_err(|err| err.to_string().into()))
}

#[wasm_bindgen]
impl ApiClient {
    pub fn devnet() -> Self {
        Self {
            inner: SolanaApiClient::devnet(),
        }
    }

    pub fn get_account_info(&self, account: String, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let account = Pubkey::from_str(&account)?;
            let cfg = cfg.into_serde()?;
            let r = client.get_account_info(account, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_program_accounts(&self, program: String, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let program = Pubkey::from_str(&program)?;
            let cfg = cfg.into_serde()?;
            let r = client.get_program_accounts(program, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_multiple_accounts(&self, accounts: Box<[JsValue]>, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let accounts: Vec<Pubkey> = accounts
                .into_iter()
                .filter_map(|a| {
                    let s = a.as_string()?;
                    Pubkey::from_str(&s).ok()
                })
                .collect();
            let cfg = cfg.into_serde()?;
            let r = client.get_multiple_accounts(&accounts, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_signature_statuses(&self, signatures: Box<[JsValue]>, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let signatures: Vec<Signature> = signatures
                .into_iter()
                .filter_map(|s| {
                    let s = s.as_string()?;
                    Signature::from_str(&s).ok()
                })
                .collect();
            let cfg = cfg.into_serde()?;
            let r = client.get_signature_statuses(&signatures, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_signatures_for_address(&self, address: String, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let address = Pubkey::from_str(&address)?;
            let cfg = cfg.into_serde()?;
            let r = client.get_signatures_for_address(&address, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_slot(&self, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let cfg = cfg.into_serde()?;
            let r = client.get_slot(cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_transaction(&self, signature: String, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let signature = Signature::from_str(&signature)?;
            let cfg = cfg.into_serde()?;
            let r = client.get_transaction(signature, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn request_airdrop(&self, pubkey: String, lamports: u64, commitment: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let pubkey = Pubkey::from_str(&pubkey)?;
            let commitment = commitment.into_serde()?;
            let r = client
                .request_airdrop(&pubkey, lamports, commitment)
                .await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn send_transaction(&self, transaction: JsValue, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let transaction = transaction.into_serde()?;
            let cfg = cfg.into_serde()?;
            let r = client.send_transaction(&transaction, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn simulate_transaction(&self, transaction: JsValue, cfg: JsValue) -> Promise {
        let client = self.inner.dupe();

        let fut = async move {
            let transaction = transaction.into_serde()?;
            let cfg = cfg.into_serde()?;
            let r = client.simulate_transaction(&transaction, cfg).await?;
            let r = JsValue::from_serde(&r)?;

            Ok(r)
        };

        return_promise(fut)
    }
}

#[wasm_bindgen]
pub fn init_rust_logs() {
    console_log::init().unwrap();
}

#[wasm_bindgen]
pub async fn run() -> Result<JsValue, JsValue> {
    let api = "https://api.devnet.solana.com";
    let client = SolanaApiClient::new(api);

    let pubkey = Pubkey::try_from("4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA").unwrap();
    let r = client
        .get_account_info(pubkey, None)
        .await
        .map_err(|err| err.to_string())?;

    let pubkey = Pubkey::try_from("4Nd1mBQtrMJVYVfKf2PJy9NZUZdTAsp7D4xWLs4gDB4T").unwrap();
    let r = client
        .get_program_accounts(pubkey, None)
        .await
        .map_err(|err| err.to_string())?;

    let accounts = &[
        Pubkey::try_from("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg").unwrap(),
        Pubkey::try_from("4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA").unwrap(),
    ];
    let r = client
        .get_multiple_accounts(
            accounts,
            Some(RpcAccountInfoConfig {
                encoding: Some(UiAccountEncoding::Base58),
                data_slice: None,
                commitment: None,
            }),
        )
        .await
        .map_err(|err| err.to_string())?;

    let signatures = &[
        Signature::try_from("5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW").unwrap(),
        Signature::try_from("5j7s6NiJS3JAkvgkoc18WVAsiSaci2pxB2A6ueCJP4tprA2TFg9wSyTLeYouxPBJEMzJinENTkpA52YStRW5Dia7").unwrap(),
    ];
    let r = client
        .get_signature_statuses(
            signatures,
            Some(RpcSignatureStatusConfig {
                search_transaction_history: true,
            }),
        )
        .await
        .map_err(|err| err.to_string())?;

    let address = Pubkey::try_from("Vote111111111111111111111111111111111111111").unwrap();
    let r = client
        .get_signatures_for_address(
            &address,
            Some(RpcSignaturesForAddressConfig {
                limit: Some(1),
                ..Default::default()
            }),
        )
        .await
        .map_err(|err| err.to_string())?;

    let r = client.get_slot(None).await.map_err(|err| err.to_string())?;

    let signature = Signature::from_str(
        "2nBhEBYYvfaAe16UMNqRHre4YNSskvuYgx3M6E4JP1oDYvZEJHvoPzyUidNgNX5r9sTyN1J9UxtbCXy2rqYcuyuv",
    )
    .unwrap();
    let r = client
        .get_transaction(
            signature,
            Some(RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Json),
                ..Default::default()
            }),
        )
        .await
        .map_err(|err| err.to_string())?;

    let pubkey = Pubkey::from_str("83astBRguLMdt2h5U1Tpdq5tjFoJ6noeGwaY3mDLVcri").unwrap();
    let r = client
        .request_airdrop(&pubkey, 1000000000, None)
        .await
        .map_err(|err| err.to_string())?;

    // TODO: send_transaction
    let transaction = Transaction {
        signatures: vec![],
        message: Message::default(),
    };
    let r = client
        .simulate_transaction(
            &transaction,
            RpcSimulateTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base58),
                ..Default::default()
            },
        )
        .await
        .map_err(|err| err.to_string())?;

    let r = JsValue::from_serde(&r).unwrap();
    Ok(r)
}
