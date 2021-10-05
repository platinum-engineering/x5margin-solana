use std::{
    convert::{TryFrom, TryInto},
    str::FromStr,
};

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use log::debug;
use parking_lot::RwLock;
use reqwest::{IntoUrl, Url};
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer,
};
use serde_json::{from_value, json, to_value, Map, Value};
use solana_api_types::{client::*, *};

/// Partially-parsed and weakly-typed Solaa JSON-RPC response.
pub struct RpcResponse {
    /// Specified the method that this response is responding to.
    ///
    /// Serves no purpose on the HTTP JSON-RPC service.
    ///
    /// On WebSocket PubSub service allows to match responses with requests.
    pub method: Option<String>,
    /// The id specified in the initial client request.
    ///
    /// Serves no purpose on the HTTP JSON-RPC service.
    ///
    /// On WebSocket PubSub service, allows to match responses with requests.
    pub id: u64,
    /// `result` field payload on the response. Used in HTTP JSON-RPC responses, and in WS JSON-RPC subscription responses (not notifications!)
    ///
    /// Not present in WS JSON-RPC notifications.
    pub result: Value,
    /// `params` field payload on the response. Only present in WS JSON-RPC subscription notifications.
    pub params: Value,
}

/// Make a JSON-RPC request with the specified id, method and request params.
pub fn make_rpc_request(id: u64, method: &str, params: Option<Value>) -> Value {
    let mut request = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
    });

    if let Some(params) = params {
        request
            .as_object_mut()
            .unwrap()
            .insert("params".into(), params);
    }

    request
}

/// Parse a JSON-RPC request from the provided untyped JSON Value.
pub fn parse_rpc_response(mut value: Value) -> RpcResponse {
    let method = value["method"].as_str().map(|s| s.into());

    let id = value["id"].as_u64().unwrap_or(0);

    let result = (&mut value["result"]).take();
    let params = (&mut value["params"]).take();

    RpcResponse {
        method,
        id,
        result,
        params,
    }
}

/// An implementation of [`solana_api_types::client::Client`] that interfaces with the Solana HTTP JSON-RPC service.
pub struct SolanaApiClient {
    client: reqwest::Client,
    url: Url,
    default_commitment: RwLock<CommitmentLevel>,
}

impl Clone for SolanaApiClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            url: self.url.clone(),
            default_commitment: RwLock::new(*self.default_commitment.read()),
        }
    }
}

impl SolanaApiClient {
    /// Create a new client that will connect to the provided endpoint.
    ///
    /// Doesn't perform any requests.
    pub fn new<T: IntoUrl>(url: T) -> anyhow::Result<Self> {
        let url = url.into_url().context("invalid url")?;
        let client = reqwest::Client::new();

        Ok(Self {
            client,
            url,
            default_commitment: RwLock::new(CommitmentLevel::Confirmed),
        })
    }

    /// Create a new client connected to the Solana Devnet ([https://api.devnet.solana.com])
    pub fn devnet() -> anyhow::Result<Self> {
        Self::new("https://api.devnet.solana.com")
    }

    /// Helper method to construct a JSON-RPC call.
    async fn jsonrpc_call(
        &self,
        method: &str,
        mut params: Value,
    ) -> Result<RpcResponse, ClientError> {
        // Clean-up empty configuration objects
        {
            let params = params.as_array_mut().unwrap();

            params.retain(|v| v.as_object().map(|m| !m.is_empty()).unwrap_or(true));
        }

        let request_json = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": method,
            "params": params,
        });

        let request_json = serde_json::to_string(&request_json)
            .expect("conversion of json value to json string should be infallible");

        debug!("sending rpc request: {}", request_json);

        let client = self.client.clone();
        let request = client
            .post(self.url.clone())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .body(request_json)
            .send()
            .await
            .map_err(ClientError::transport)?;

        let body = request.bytes().await.map_err(ClientError::transport)?;
        let body = std::str::from_utf8(&body).map_err(ClientError::parsing)?;

        debug!("received rpc response: {}", body);

        let body: serde_json::Value = serde_json::from_str(body).map_err(ClientError::parsing)?;

        Ok(parse_rpc_response(body))
    }

    /// Adds a commitment level to the params array if specified, otherwise adds the default commitment.
    fn add_commitment(&self, params: &mut Map<String, Value>, commitment: Option<CommitmentLevel>) {
        let commitment = if let Some(commitment) = commitment {
            commitment.to_str()
        } else {
            self.default_commitment.read().to_str()
        };

        params["commitment"] = json!(commitment);
    }
}

fn deserialize_base64_data<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u8>, D::Error> {
    struct V;

    impl<'de> Visitor<'de> for V {
        type Value = &'de str;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "string")
        }

        fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }
    }

    let s = deserializer.deserialize_str(V)?;
    base64::decode(s).map_err(|_| D::Error::custom("invalid base64 encoding"))
}

fn deserialize_base64_pubkey<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Pubkey, D::Error> {
    let buf = deserialize_base64_data(deserializer)?;

    if buf.len() != 32 {
        Err(D::Error::invalid_length(buf.len(), &"32"))
    } else {
        Ok(Pubkey::new(buf.try_into().expect("infallible")))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiAccountPartial {
    lamports: u64,
    #[serde(deserialize_with = "deserialize_base64_pubkey")]
    owner: Pubkey,
    #[serde(deserialize_with = "deserialize_base64_data")]
    data: Vec<u8>,
    executable: bool,
    rent_epoch: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiAccountFull {
    #[serde(deserialize_with = "deserialize_base64_pubkey")]
    pubkey: Pubkey,
    account: UiAccountPartial,
}

#[async_trait(?Send)]
impl Client for SolanaApiClient {
    async fn get_account_info(
        &self,
        pubkey: &Pubkey,
        slice: Option<AccountSlice>,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<Account>, ClientError> {
        let mut cfg = Map::new();

        cfg["encoding"] = json!("base64");

        if let Some(slice) = slice {
            cfg["dataSlice"] = json!({"offset" : slice.offset, "length": slice.length});
        }

        self.add_commitment(&mut cfg, commitment);

        let response = self
            .jsonrpc_call("getAccountInfo", json!([pubkey.to_string(), cfg]))
            .await?;

        if response.result["value"].is_null() {
            Ok(None)
        } else {
            let account =
                serde_json::from_value::<UiAccountPartial>(response.result["value"].clone())
                    .map_err(ClientError::parsing)?;

            let account = Account {
                lamports: account.lamports,
                owner: account.owner,
                data: account.data,
                executable: account.executable,
                rent_epoch: account.rent_epoch,
                pubkey: *pubkey,
            };

            Ok(Some(account))
        }
    }

    async fn get_program_accounts_ex(
        &self,
        program: &Pubkey,
        filters: Option<&[AccountFilter]>,
        slice: Option<AccountSlice>,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Vec<Account>, ClientError> {
        let mut cfg = Map::new();

        cfg["encoding"] = json!("base64");

        if let Some(slice) = slice {
            cfg["dataSlice"] = json!({"offset" : slice.offset, "length": slice.length});
        }

        if let Some(filters) = filters {
            cfg["filters"] = json!(filters
                .iter()
                .map(|f| f.to_json_value())
                .collect::<Vec<_>>());
        }

        self.add_commitment(&mut cfg, commitment);

        let response = self
            .jsonrpc_call("getProgramAccounts", json!([program.to_string(), cfg]))
            .await?;

        let accounts =
            from_value::<Vec<UiAccountFull>>(response.result).map_err(ClientError::parsing)?;

        Ok(accounts
            .into_iter()
            .map(|account| Account {
                lamports: account.account.lamports,
                owner: account.account.owner,
                data: account.account.data,
                executable: account.account.executable,
                rent_epoch: account.account.rent_epoch,
                pubkey: account.pubkey,
            })
            .collect())
    }

    async fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
        slice: Option<AccountSlice>,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Vec<Option<Account>>, ClientError> {
        let mut cfg = Map::new();

        cfg["encoding"] = json!("base64");

        if let Some(slice) = slice {
            cfg["dataSlice"] = json!({"offset" : slice.offset, "length": slice.length});
        }

        self.add_commitment(&mut cfg, commitment);
        let pubkeys = pubkeys.iter().map(|p| p.to_string()).collect::<Vec<_>>();

        let response = self
            .jsonrpc_call("getMultipleAccounts", json!([pubkeys, cfg]))
            .await?;

        let accounts = from_value::<Vec<Option<UiAccountFull>>>(response.result)
            .map_err(ClientError::parsing)?;

        Ok(accounts
            .into_iter()
            .map(|account| {
                account.map(|account| Account {
                    lamports: account.account.lamports,
                    owner: account.account.owner,
                    data: account.account.data,
                    executable: account.account.executable,
                    rent_epoch: account.account.rent_epoch,
                    pubkey: account.pubkey,
                })
            })
            .collect())
    }

    async fn get_transaction_statuses(
        &self,
        signatures: &[Signature],
        search_history: bool,
    ) -> Result<Vec<Option<TransactionStatus>>, ClientError> {
        let mut cfg = Map::new();

        if search_history {
            cfg["searchTransactionHistory"] = json!(true);
        }

        let signatures = signatures.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let response = self
            .jsonrpc_call("getSignatureStatuses", json!([signatures, cfg]))
            .await?;

        let statuses = from_value::<Vec<Value>>(response.result).map_err(ClientError::parsing)?;

        Ok(statuses
            .into_iter()
            .map(|status| {
                if status.is_null() {
                    None
                } else {
                    TransactionStatus::try_from(&status).ok()
                }
            })
            .collect())
    }

    async fn get_transactions_for_account(
        &self,
        account: &Pubkey,
        before: Option<&Signature>,
        until: Option<&Signature>,
        limit: u64,
    ) -> Result<Vec<TransactionSummary>, ClientError> {
        todo!()
    }

    async fn get_slot(&self, commitment: Option<CommitmentLevel>) -> Result<Slot, ClientError> {
        let mut cfg = Map::new();
        self.add_commitment(&mut cfg, commitment);
        let response = self.jsonrpc_call("getSlot", json!([cfg])).await?;

        Ok(from_value::<u64>(response.result).map_err(ClientError::parsing)?)
    }

    async fn get_transaction(
        &self,
        signature: Signature,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Option<ConfirmedTransaction>, ClientError> {
        todo!()
    }

    async fn get_recent_blockhash(
        &self,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Hash, ClientError> {
        let mut cfg = Map::new();
        self.add_commitment(&mut cfg, commitment);
        let response = self
            .jsonrpc_call("getRecentBlockhash", json!([cfg]))
            .await?;
        let hash = from_value::<String>(response.result["value"]["blockhash"].clone())
            .map_err(ClientError::parsing)?;
        let hash = Hash::from_str(&hash).map_err(ClientError::parsing)?;

        Ok(hash)
    }

    async fn request_airdrop(
        &self,
        pubkey: &Pubkey,
        lamports: u64,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Signature, ClientError> {
        todo!()
    }

    async fn send_transaction_ex(
        &self,
        transaction: &Transaction,
        skip_preflight: bool,
        preflight_commitment: Option<CommitmentLevel>,
    ) -> Result<Signature, ClientError> {
        todo!()
    }

    // async fn get_account_info(
    //     &self,
    //     account: solana_api_types::Pubkey,
    //     cfg: Option<solana_api_types::RpcAccountInfoConfig>,
    // ) -> Result<solana_api_types::Account, solana_api_types::ClientError> {
    //     let r: RpcResponse<Option<UiAccount>> = self
    //         .jsonrpc_call(Request {
    //             method: "getAccountInfo",
    //             params: json!([account.to_string(), serde_json::to_value(&cfg)?,]),
    //         })
    //         .await?;

    //     let account = r
    //         .value
    //         .ok_or_else(|| RpcError::ForUser("account not found".into()))?
    //         .decode(account)
    //         .ok_or_else(|| RpcError::ParseError("failed to decode account".to_string()))?;

    //     Ok(account)
    // }

    // async fn get_program_accounts(
    //     &self,
    //     program: solana_api_types::Pubkey,
    //     cfg: Option<solana_api_types::RpcProgramAccountsConfig>,
    // ) -> Result<Vec<solana_api_types::Account>, solana_api_types::ClientError> {
    //     let r: Vec<RpcKeyedAccount> = self
    //         .jsonrpc_call(Request {
    //             method: "getProgramAccounts",
    //             params: json!([program.to_string(), serde_json::to_value(&cfg)?,]),
    //         })
    //         .await?;

    //     let r = r
    //         .into_iter()
    //         .filter_map(|a| {
    //             let pubkey = Pubkey::from_str(a.pubkey.as_str()).ok()?;
    //             a.account.decode(pubkey)
    //         })
    //         .collect();

    //     Ok(r)
    // }

    // async fn get_multiple_accounts(
    //     &self,
    //     accounts: &[solana_api_types::Pubkey],
    //     cfg: Option<solana_api_types::RpcAccountInfoConfig>,
    // ) -> Result<Vec<solana_api_types::Account>, solana_api_types::ClientError> {
    //     let accounts_as_str: Vec<String> = accounts.iter().map(|a| a.to_string()).collect();

    //     let r: RpcResponse<Vec<Option<UiAccount>>> = self
    //         .jsonrpc_call(Request {
    //             method: "getMultipleAccounts",
    //             params: json!([accounts_as_str, serde_json::to_value(&cfg)?,]),
    //         })
    //         .await?;

    //     let r = r
    //         .value
    //         .into_iter()
    //         .zip(accounts)
    //         .filter_map(|(acc, key)| acc?.decode(*key))
    //         .collect();

    //     Ok(r)
    // }

    // async fn get_signature_statuses(
    //     &self,
    //     signatures: &[Signature],
    //     cfg: Option<solana_api_types::RpcSignatureStatusConfig>,
    // ) -> Result<Vec<Option<TransactionStatus>>, solana_api_types::ClientError> {
    //     let signatures: Vec<String> = signatures.iter().map(|s| s.to_string()).collect();

    //     let r: RpcResponse<Vec<Option<TransactionStatus>>> = self
    //         .jsonrpc_call(Request {
    //             method: "getSignatureStatuses",
    //             params: json!([signatures, serde_json::to_value(&cfg)?,]),
    //         })
    //         .await?;

    //     Ok(r.value)
    // }

    // async fn get_signatures_for_address(
    //     &self,
    //     address: &solana_api_types::Pubkey,
    //     cfg: Option<solana_api_types::RpcSignaturesForAddressConfig>,
    // ) -> Result<Vec<solana_api_types::SignatureInfo>, solana_api_types::ClientError> {
    //     let r: RpcResponse<Vec<solana_api_types::SignatureInfo>> = self
    //         .jsonrpc_call(Request {
    //             method: "getSignaturesForAddress",
    //             params: json!([address.to_string(), serde_json::to_value(&cfg)?,]),
    //         })
    //         .await?;

    //     Ok(r.value)
    // }

    // async fn get_slot(
    //     &self,
    //     cfg: Option<solana_api_types::RpcSlotConfig>,
    // ) -> Result<solana_api_types::Slot, solana_api_types::ClientError> {
    //     let r: solana_api_types::Slot = self
    //         .jsonrpc_call(Request {
    //             method: "getSlot",
    //             params: json!([serde_json::to_value(&cfg)?,]),
    //         })
    //         .await?;

    //     Ok(r)
    // }

    // async fn get_transaction(
    //     &self,
    //     signature: Signature,
    //     cfg: Option<solana_api_types::RpcTransactionConfig>,
    // ) -> Result<Option<solana_api_types::EncodedConfirmedTransaction>, solana_api_types::ClientError>
    // {
    //     let r: Option<solana_api_types::EncodedConfirmedTransaction> = self
    //         .jsonrpc_call(Request {
    //             method: "getTransaction",
    //             params: json!([signature.to_string(), serde_json::to_value(&cfg)?,]),
    //         })
    //         .await?;

    //     Ok(r)
    // }

    // async fn get_recent_blockhash(&self) -> Result<Hash, solana_api_types::ClientError> {
    //     let r: RpcResponse<Blockhash> = self
    //         .jsonrpc_call(Request {
    //             method: "getRecentBlockhash",
    //             params: json!([]),
    //         })
    //         .await?;

    //     let hash = Hash::from_str(&r.value.blockhash).unwrap();

    //     Ok(hash)
    // }

    // async fn request_airdrop(
    //     &self,
    //     pubkey: &solana_api_types::Pubkey,
    //     lamports: u64,
    //     _commitment: Option<solana_api_types::CommitmentConfig>,
    // ) -> Result<Signature, solana_api_types::ClientError> {
    //     let r: String = self
    //         .jsonrpc_call(Request {
    //             method: "requestAirdrop",
    //             params: json!([pubkey.to_string(), lamports]),
    //         })
    //         .await?;

    //     let signature = Signature::from_str(&r).unwrap();

    //     Ok(signature)
    // }

    // async fn send_transaction_ex(
    //     &self,
    //     transaction: &solana_api_types::Transaction,
    //     skip_preflight: bool,
    //     preflight_commitment: CommitmentLevel,
    // ) -> Result<Signature, solana_api_types::ClientError> {
    //     let transaction = transaction.encode(TransactionEncoding::Base64)?;

    //     let r: String = self
    //         .jsonrpc_call(Request {
    //             method: "sendTransaction",
    //             params: json!([transaction, {
    //                 "skipPreflight": skip_preflight,
    //                 "preflightCommitment": preflight_commitment,
    //                 "encoding": "base64",
    //             }]),
    //         })
    //         .await?;

    //     let signature = Signature::from_str(&r).unwrap();

    //     Ok(signature)
    // }

    // async fn simulate_transaction(
    //     &self,
    //     _transaction: &solana_api_types::Transaction,
    //     _sig_verify: bool,
    //     _commitment: CommitmentLevel,
    //     _replace_recent_blockhash: bool,
    // ) -> Result<solana_api_types::RpcSimulateTransactionResult, solana_api_types::ClientError> {
    //     todo!()
    // }

    fn default_commitment_level(&self) -> CommitmentLevel {
        *self.default_commitment.read()
    }

    fn set_default_commitment_level(&self, level: CommitmentLevel) {
        *self.default_commitment.write() = level;
    }
}
