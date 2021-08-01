use std::{
    convert::TryFrom,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Deserialize};

use solana_api_types::{
    Client, ClientError, Pubkey, RpcAccountInfoConfig, RpcError, RpcKeyedAccount, RpcResponse,
    RpcSignatureStatusConfig, Signature, TransactionStatus, UiAccount, UiAccountEncoding,
};

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
    result: T,
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
        let body: serde_json::Value = serde_json::from_slice(&body)?;
        println!("{}", body);
        let body: JsonRpcResponse<T> = serde_json::from_value(body)?;

        Ok(body.result)
    }
}

#[async_trait(?Send)]
impl Client for SolanaApiClient {
    async fn get_account_info(
        &self,
        account: solana_api_types::Pubkey,
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
        program: solana_api_types::Pubkey,
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
        accounts: &[solana_api_types::Pubkey],
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
        signature: Signature,
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
    ) -> Result<Signature, solana_api_types::ClientError> {
        todo!()
    }

    async fn send_transaction(
        &self,
        transaction: &solana_api_types::Transaction,
        cfg: solana_api_types::RpcSendTransactionConfig,
    ) -> Result<Signature, solana_api_types::ClientError> {
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

#[cfg(test)]
mod tests {
    use std::{
        convert::TryFrom,
        str::FromStr,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::{SolanaApiClient, Client};

    #[tokio::test]
    async fn get_account_info_test() {

        let client = SolanaApiClient {
            client: reqwest::Client::new(),
            current_id: AtomicUsize::new(0),
            solana_api_url: "https://api.devnet.solana.com",
        };

        let pubkey = solana_api_types::Pubkey::try_from("13LeFbG6m2EP1fqCj9k66fcXsoTHMMtgr7c78AivUrYD").unwrap();
       
        let r = client
        .get_account_info(pubkey, None)
        .await
        .map_err(|err| err.to_string());

        println!("{:?}", r);
    }

    #[tokio::test]
    async fn get_program_accounts_test() {

        let client = SolanaApiClient {
            client: reqwest::Client::new(),
            current_id: AtomicUsize::new(0),
            solana_api_url: "https://api.devnet.solana.com",
        };

        let pubkey = solana_api_types::Pubkey::try_from("6TvznH3B2e3p2mbhufNBpgSrLx6UkgvxtVQvopEZ2kuH").unwrap();
        let acccount_cfg = solana_api_types::RpcAccountInfoConfig {
            encoding: Some(solana_api_types::UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
        };
        let cfg = solana_api_types::RpcProgramAccountsConfig {
            filters: None,
            account_config: acccount_cfg,
            with_context: None,
        };

        let r = client
            .get_program_accounts(pubkey, Some(cfg))
            .await
            .map_err(|err| err.to_string());

        println!("{:?}", r);
    }

    #[tokio::test]
    async fn get_multiple_accounts_test() {

        let client = SolanaApiClient {
            client: reqwest::Client::new(),
            current_id: AtomicUsize::new(0),
            solana_api_url: "https://api.devnet.solana.com",
        };

        let accounts = &[
            solana_api_types::Pubkey::try_from("9B5XszUGdMaxCZ7uSQhPzdks5ZQSmWxrmzCSvtJ6Ns6g").unwrap(),
            solana_api_types::Pubkey::try_from("13LeFbG6m2EP1fqCj9k66fcXsoTHMMtgr7c78AivUrYD").unwrap(),
    ];
    let r = client
        .get_multiple_accounts(
            accounts,
            Some(solana_api_types::RpcAccountInfoConfig {
                encoding: Some(solana_api_types::UiAccountEncoding::Base64),
                data_slice: None,
                commitment: None,
            }),
        )
        .await
        .map_err(|err| err.to_string());

        println!("{:?}", r);
    }

    #[tokio::test]
    async fn get_signature_statuses_test () {

        let client = SolanaApiClient {
            client: reqwest::Client::new(),
            current_id: AtomicUsize::new(0),
            solana_api_url: "https://api.devnet.solana.com",
        };

        let signatures = &[
            solana_api_types::Signature::try_from("5eCvikyPBwCKDvyKAdrAfLh9RgmKKvu8x5KpVeuBAVugvnzqcfdFe9DWpSaqJUh4ncdU6VU3Nt7p2YWyoscivtRu").unwrap(),
            solana_api_types::Signature::try_from("44pGayfTYPSMT31zdzsdRWovCzRv3AeMEJZ4Z83XzNbDmHyzVGN2LV6SGkqbkPQbgNWQmV9fVEtVV6nZCEgpa7E6").unwrap(),
        ];

        let r = client
            .get_signature_statuses(
                signatures,
                Some(solana_api_types::RpcSignatureStatusConfig {
                    search_transaction_history: false,
                }),
            )
            .await
            .map_err(|err| err.to_string());

        println!("{:?}", r);
    }
}