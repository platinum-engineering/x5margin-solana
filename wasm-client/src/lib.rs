use std::{
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use futures::{Future, TryFutureExt};
use js_sys::Promise;
use parity_scale_codec::Encode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

use solana_api_types::{
    Account, AccountMeta, Client, ClientError, ClientErrorKind, EncodedConfirmedTransaction,
    Instruction, Pubkey, RpcAccountInfoConfig, RpcError, RpcKeyedAccount, RpcResponse,
    RpcSendTransactionConfig, RpcSignaturesForAddressConfig, RpcSimulateTransactionConfig,
    RpcSimulateTransactionResult, Signature, SignatureInfo, Signer, SignerError, Slot, Transaction,
    TransactionStatus, UiAccount,
};

pub trait ResultExt<T> {
    fn into_js_value(self) -> Result<T, JsValue>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    ClientError: From<E>,
{
    fn into_js_value(self) -> Result<T, JsValue> {
        self.map_err(|err| ClientError::from(err).into())
    }
}

#[wasm_bindgen]
#[derive(Clone, Copy)]
pub struct Pk(Pubkey);

#[wasm_bindgen]
impl Pk {
    pub fn new(key: &str) -> Result<Pk, JsValue> {
        Ok(Self(Pubkey::from_str(key).into_js_value()?))
    }
}

impl Pk {
    fn to_pubkey(self) -> Pubkey {
        self.0
    }
}

impl AsRef<Pubkey> for Pk {
    fn as_ref(&self) -> &Pubkey {
        &self.0
    }
}

#[wasm_bindgen]
pub struct Sig(Signature);

#[wasm_bindgen]
impl Sig {
    pub fn new(signature: &str) -> Result<Sig, JsValue> {
        Ok(Self(Signature::from_str(signature).into_js_value()?))
    }
}

impl Sig {
    fn into_inner(self) -> Signature {
        self.0
    }
}

struct RawApiClient {
    client: reqwest::Client,
    current_id: AtomicUsize,
    solana_api_url: &'static str,
}

impl Clone for RawApiClient {
    fn clone(&self) -> Self {
        let id = self.current_id.fetch_add(1, Ordering::SeqCst);
        Self {
            client: self.client.clone(),
            current_id: AtomicUsize::new(id),
            solana_api_url: self.solana_api_url,
        }
    }
}

impl RawApiClient {
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

impl RawApiClient {
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
impl Client for RawApiClient {
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

fn return_promise<T>(fut: impl Future<Output = Result<T, ClientError>> + 'static) -> Promise
where
    T: serde::Serialize,
{
    let fut = async move {
        let r = fut.await?;
        Ok::<JsValue, ClientError>(JsValue::from_serde(&r)?)
    };

    future_to_promise(fut.map_err(|err| err.into()))
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct ApiClient {
    inner: RawApiClient,
}

#[wasm_bindgen]
impl ApiClient {
    pub fn devnet() -> Self {
        Self {
            inner: RawApiClient::devnet(),
        }
    }

    pub fn get_account_info(&self, account: Pk, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let account = account.to_pubkey();
            let cfg = cfg.into_serde()?;
            let r = client.get_account_info(account, cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_program_accounts(&self, program: Pk, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let program = program.to_pubkey();
            let cfg = cfg.into_serde()?;
            let r = client.get_program_accounts(program, cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_multiple_accounts(&self, accounts: Box<[JsValue]>, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let accounts: Vec<Pubkey> = accounts
                .iter()
                .filter_map(|a| {
                    let s = a.as_string()?;
                    Pubkey::from_str(&s).ok()
                })
                .collect();
            let cfg = cfg.into_serde()?;
            let r = client.get_multiple_accounts(&accounts, cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_signature_statuses(&self, signatures: Box<[JsValue]>, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let signatures: Vec<Signature> = signatures
                .iter()
                .filter_map(|s| {
                    let s = s.as_string()?;
                    Signature::from_str(&s).ok()
                })
                .collect();
            let cfg = cfg.into_serde()?;
            let r = client.get_signature_statuses(&signatures, cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_signatures_for_address(&self, address: Pk, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let address = address.to_pubkey();
            let cfg = cfg.into_serde()?;
            let r = client.get_signatures_for_address(&address, cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_slot(&self, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let cfg = cfg.into_serde()?;
            let r = client.get_slot(cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn get_transaction(&self, signature: Sig, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let signature = signature.into_inner();
            let cfg = cfg.into_serde()?;
            let r = client.get_transaction(signature, cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn request_airdrop(&self, pubkey: Pk, lamports: u64, commitment: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let pubkey = pubkey.to_pubkey();
            let commitment = commitment.into_serde()?;
            let r = client
                .request_airdrop(&pubkey, lamports, commitment)
                .await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn send_transaction(&self, transaction: Tx, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let cfg = cfg.into_serde()?;
            let r = client.send_transaction(transaction.as_ref(), cfg).await?;

            Ok(r)
        };

        return_promise(fut)
    }

    pub fn simulate_transaction(&self, transaction: Tx, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let cfg = cfg.into_serde()?;
            let r = client
                .simulate_transaction(transaction.as_ref(), cfg)
                .await?;

            Ok(r)
        };

        return_promise(fut)
    }
}

#[wasm_bindgen]
#[derive(Serialize)]
pub struct MintAccount {
    account: solar::spl::MintAccount<Box<Account>>,
}

impl MintAccount {
    fn any(account: Account) -> Result<Self, solar::spl::SplReadError> {
        let account = Box::new(account);
        let account = solar::spl::MintAccount::any(account)?;

        Ok(MintAccount { account })
    }

    fn wallet(&self, account: Account) -> Result<WalletAccount, solar::spl::SplReadError> {
        let account = Box::new(account);
        let account = self.account.wallet(account)?;

        Ok(WalletAccount { account })
    }
}

#[wasm_bindgen]
#[derive(Serialize)]
pub struct WalletAccount {
    account: solar::spl::WalletAccount<Box<Account>>,
}

impl WalletAccount {
    fn any(account: Account) -> Result<Self, solar::spl::SplReadError> {
        let account = Box::new(account);
        let account = solar::spl::WalletAccount::any(account)?;

        Ok(WalletAccount { account })
    }
}

#[derive(Clone)]
pub struct RawPoolClient {
    inner: RawApiClient,
}

impl RawPoolClient {
    async fn load_wallet_account(
        &self,
        pubkey: Pubkey,
        cfg: Option<RpcAccountInfoConfig>,
    ) -> Result<WalletAccount, ClientError> {
        let account = self.inner.get_account_info(pubkey, cfg).await?;
        let account = WalletAccount::any(account)
            .map_err(|err| ClientError::from(ClientErrorKind::Custom(err.to_string())))?;

        Ok(account)
    }

    async fn load_mint_account(
        &self,
        pubkey: Pubkey,
        cfg: Option<RpcAccountInfoConfig>,
    ) -> Result<MintAccount, ClientError> {
        let account = self.inner.get_account_info(pubkey, cfg).await?;
        let account = MintAccount::any(account)
            .map_err(|err| ClientError::from(ClientErrorKind::Custom(err.to_string())))?;

        Ok(account)
    }

    async fn load_stake_pool(
        &self,
        program: Pubkey,
        stake_pool: Pubkey,
    ) -> Result<StakePoolEntity, ClientError> {
        let stake_pool = self.inner.get_account_info(stake_pool, None).await?;
        let stake_pool = StakePoolEntity::load(&program, Box::new(stake_pool))
            .map_err(|err| ClientError::from(ClientErrorKind::Custom(err.to_string())))?;

        Ok(stake_pool)
    }

    async fn load_staker_ticket(
        &self,
        ticket: Pubkey,
        stake_pool: &StakePoolEntity,
    ) -> Result<StakerTicketEntity, ClientError> {
        let ticket = self.inner.get_account_info(ticket, None).await?;
        let ticket = stake_pool
            .load_ticket(Box::new(ticket))
            .map_err(|err| ClientError::from(ClientErrorKind::Custom(err.to_string())))?;

        Ok(ticket)
    }
}

#[wasm_bindgen]
pub struct PoolClient {
    inner: RawPoolClient,
}

#[wasm_bindgen]
impl PoolClient {
    pub fn load_wallet_account(&self, pubkey: Pk, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let pubkey = pubkey.to_pubkey();
            let cfg = cfg.into_serde()?;

            let wallet_account = client.load_wallet_account(pubkey, cfg).await?;
            Ok(wallet_account)
        };

        return_promise(fut)
    }

    pub fn load_mint_account(&self, pubkey: Pk, cfg: JsValue) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let pubkey = pubkey.to_pubkey();
            let cfg = cfg.into_serde()?;

            let wallet_account = client.load_mint_account(pubkey, cfg).await?;
            Ok(wallet_account)
        };

        return_promise(fut)
    }

    pub fn load_stake_pool(&self, program: Pk, stake_pool: Pk) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let program = program.to_pubkey();
            let stake_pool = stake_pool.to_pubkey();
            let stake_pool = client.load_stake_pool(program, stake_pool).await?;
            Ok(stake_pool)
        };

        return_promise(fut)
    }

    pub fn load_staker_ticket(&self, ticket: Pk, stake_pool: StakePoolEntity) -> Promise {
        let client = self.inner.clone();

        let fut = async move {
            let ticket = ticket.to_pubkey();
            let ticket = client.load_staker_ticket(ticket, &stake_pool).await?;
            Ok(ticket)
        };

        return_promise(fut)
    }
}

#[wasm_bindgen]
pub struct StakePoolEntity {
    entity: x5margin_program::simple_stake::StakePoolEntity<Box<Account>>,
}

impl Serialize for StakePoolEntity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.entity.account().serialize(serializer)
    }
}

impl StakePoolEntity {
    pub fn load(
        program: &Pubkey,
        pool: Box<Account>,
    ) -> Result<Self, x5margin_program::error::Error> {
        let stake_pool = x5margin_program::simple_stake::StakePoolEntity::load(program, pool)?;

        Ok(Self { entity: stake_pool })
    }

    pub fn load_ticket(
        &self,
        ticket: Box<Account>,
    ) -> Result<StakerTicketEntity, x5margin_program::error::Error> {
        self.entity
            .load_ticket(ticket)
            .map(|entity| StakerTicketEntity { entity })
    }
}

#[wasm_bindgen]
pub struct StakerTicketEntity {
    entity: x5margin_program::simple_stake::StakerTicketEntity<Box<Account>>,
}

impl Serialize for StakerTicketEntity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.entity.account().serialize(serializer)
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Instr(Instruction);

impl From<Instruction> for Instr {
    fn from(i: Instruction) -> Self {
        Self(i)
    }
}

#[wasm_bindgen]
pub struct Instructions {
    inner: Vec<Instruction>,
}

impl<const N: usize> From<[Instruction; N]> for Instructions {
    fn from(src: [Instruction; N]) -> Self {
        Self {
            inner: src.to_vec(),
        }
    }
}

impl AsRef<[Instruction]> for Instructions {
    fn as_ref(&self) -> &[Instruction] {
        self.inner.as_ref()
    }
}

#[wasm_bindgen]
impl Instructions {
    pub fn push(&mut self, i: Instr) {
        self.inner.push(i.0);
    }
}

#[wasm_bindgen]
pub fn create_mint(payer: Pk, mint: Pk, authority: Pk, decimals: u8) -> Instructions {
    solar::spl::create_mint(payer.as_ref(), mint.as_ref(), authority.as_ref(), decimals).into()
}

#[wasm_bindgen]
pub fn create_wallet(payer: Pk, wallet: Pk, mint: Pk, authority: Pk) -> Instructions {
    solar::spl::create_wallet(
        payer.as_ref(),
        wallet.as_ref(),
        mint.as_ref(),
        authority.as_ref(),
    )
    .into()
}

#[wasm_bindgen]
pub fn mint_to(mint: Pk, wallet: Pk, authority: Pk, amount: u64) -> Instr {
    solar::spl::mint_to(mint.as_ref(), wallet.as_ref(), authority.as_ref(), amount).into()
}

#[wasm_bindgen]
pub fn create_account(
    from_pubkey: Pk,
    to_pubkey: Pk,
    lamports: u64,
    space: u64,
    owner: Pk,
) -> Instr {
    solana_api_types::system::create_account(
        from_pubkey.as_ref(),
        to_pubkey.as_ref(),
        lamports,
        space,
        owner.as_ref(),
    )
    .into()
}

#[wasm_bindgen]
pub struct ProgramAuthority {
    salt: u64,
    pk: Pk,
}

#[wasm_bindgen]
impl ProgramAuthority {
    pub fn new(key: Pk, administrator_key: Pk, program_id: Pk) -> Self {
        let mut salt: u64 = 0;
        let pk = loop {
            let pk = Pubkey::create_program_address(
                &[
                    key.as_ref().as_ref(),
                    administrator_key.as_ref().as_ref(),
                    &salt.to_le_bytes(),
                ],
                program_id.as_ref(),
            );

            match pk {
                Some(s) => break s,
                None => {
                    salt += 1;
                }
            }
        };

        Self { salt, pk: Pk(pk) }
    }
}

#[wasm_bindgen]
pub struct CreatePoolArgs {
    lockup_duration: i64,
    topup_duration: i64,
    reward_amount: u64,
    target_amount: u64,
}

#[wasm_bindgen]
impl CreatePoolArgs {
    pub fn new(
        lockup_duration: i64,
        topup_duration: i64,
        reward_amount: u64,
        target_amount: u64,
    ) -> Self {
        Self {
            lockup_duration,
            topup_duration,
            reward_amount,
            target_amount,
        }
    }
}

#[wasm_bindgen]
pub struct PoolInstructionBuilder {
    pool_key: Pk,
    administrator_key: Pk,
    program_id: Pk,
    stake_mint_key: Pk,
    stake_vault_key: Pk,
    authority: ProgramAuthority,
}

#[wasm_bindgen]
impl PoolInstructionBuilder {
    pub fn new(
        pool_key: Pk,
        administrator_key: Pk,
        stake_mint_key: Pk,
        stake_vault_key: Pk,
        program_id: Pk,
    ) -> Self {
        Self {
            pool_key,
            administrator_key,
            program_id,
            stake_mint_key,
            stake_vault_key,
            authority: ProgramAuthority::new(pool_key, administrator_key, program_id),
        }
    }

    pub fn create_pool(&self, args: CreatePoolArgs) -> Instr {
        Instruction {
            program_id: self.program_id.to_pubkey(),
            accounts: vec![
                AccountMeta::new_readonly(self.administrator_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.authority.pk.to_pubkey(), false),
                AccountMeta::new(self.pool_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_mint_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_vault_key.to_pubkey(), false),
            ],
            data: x5margin_program::Method::Simple(
                x5margin_program::simple_stake::Method::CreatePool(
                    x5margin_program::simple_stake::InitializeArgs {
                        program_authority_salt: self.authority.salt,
                        lockup_duration: args.lockup_duration.into(),
                        topup_duration: args.topup_duration.into(),
                        reward_amount: args.reward_amount.into(),
                        target_amount: args.target_amount.into(),
                    },
                ),
            )
            .encode(),
        }
        .into()
    }

    pub fn stake(
        &self,
        amount: u64,
        staker_key: Pk,
        staker_ticket_key: Pk,
        aux_wallet_key: Pk,
    ) -> Instr {
        Instruction {
            program_id: self.program_id.to_pubkey(),
            accounts: vec![
                AccountMeta::new_readonly(*solar::spl::ID, false),
                AccountMeta::new(self.pool_key.to_pubkey(), false),
                AccountMeta::new_readonly(staker_key.to_pubkey(), false),
                AccountMeta::new(staker_ticket_key.to_pubkey(), false),
                AccountMeta::new(self.stake_vault_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.administrator_key.to_pubkey(), true),
                AccountMeta::new(aux_wallet_key.to_pubkey(), false),
            ],
            data: x5margin_program::Method::Simple(x5margin_program::simple_stake::Method::Stake {
                amount: amount.into(),
            })
            .encode(),
        }
        .into()
    }

    pub fn unstake(&self, amount: u64) -> Instr {
        Instruction {
            program_id: self.program_id.to_pubkey(),
            accounts: vec![
                AccountMeta::new_readonly(self.administrator_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.authority.pk.to_pubkey(), false),
                AccountMeta::new(self.pool_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_mint_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_vault_key.to_pubkey(), false),
            ],
            data: x5margin_program::Method::Simple(
                x5margin_program::simple_stake::Method::Unstake {
                    amount: amount.into(),
                },
            )
            .encode(),
        }
        .into()
    }

    pub fn claim_reward(&self) -> Instr {
        Instruction {
            program_id: self.program_id.to_pubkey(),
            accounts: vec![
                AccountMeta::new_readonly(self.administrator_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.authority.pk.to_pubkey(), false),
                AccountMeta::new(self.pool_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_mint_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_vault_key.to_pubkey(), false),
            ],
            data: x5margin_program::Method::Simple(
                x5margin_program::simple_stake::Method::ClaimReward,
            )
            .encode(),
        }
        .into()
    }

    pub fn add_reward(&self, amount: u64) -> Instr {
        Instruction {
            program_id: self.program_id.to_pubkey(),
            accounts: vec![
                AccountMeta::new_readonly(self.administrator_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.authority.pk.to_pubkey(), false),
                AccountMeta::new(self.pool_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_mint_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.stake_vault_key.to_pubkey(), false),
            ],
            data: x5margin_program::Method::Simple(
                x5margin_program::simple_stake::Method::AddReward {
                    amount: amount.into(),
                },
            )
            .encode(),
        }
        .into()
    }
}

#[wasm_bindgen]
pub struct LockerInstructionBuilder {
    program_id: Pk,
    locker: Pk,
    administrator_key: Pk,
    authority: ProgramAuthority,
}

#[wasm_bindgen]
impl LockerInstructionBuilder {
    pub fn new(program_id: Pk, locker: Pk, administrator_key: Pk) -> Self {
        Self {
            program_id,
            locker,
            administrator_key,
            authority: ProgramAuthority::new(locker, administrator_key, program_id),
        }
    }

    pub fn create_token_lock(&self, unlock_date: i64, amount: u64) -> Instr {
        Instruction {
            program_id: self.program_id.to_pubkey(),
            accounts: vec![
                AccountMeta::new_readonly(self.administrator_key.to_pubkey(), false),
                AccountMeta::new_readonly(self.locker.to_pubkey(), false),
                AccountMeta::new(self.locker.to_pubkey(), false),
                todo!(),
            ],
            data: todo!(),
        }
        .into()
    }
}

#[wasm_bindgen]
pub struct Hash(solana_api_types::Hash);

impl Hash {
    fn into_inner(self) -> solana_api_types::Hash {
        self.0
    }
}

#[wasm_bindgen]
pub struct Keypair(solana_api_types::Keypair);

impl AsRef<solana_api_types::Keypair> for Keypair {
    fn as_ref(&self) -> &solana_api_types::Keypair {
        &self.0
    }
}

#[wasm_bindgen]
pub struct Signers(Vec<Keypair>);

#[wasm_bindgen]
pub struct Tx(solana_api_types::Transaction);

impl From<solana_api_types::Transaction> for Tx {
    fn from(tx: solana_api_types::Transaction) -> Self {
        Self(tx)
    }
}

impl AsRef<solana_api_types::Transaction> for Tx {
    fn as_ref(&self) -> &solana_api_types::Transaction {
        &self.0
    }
}

#[wasm_bindgen]
pub fn transaction_signed_with_payer(
    instructions: Instructions,
    payer: Pk,
    signers: &Signers,
    recent_blockhash: Hash,
) -> Tx {
    Transaction::new_signed_with_payer(
        instructions.as_ref(),
        Some(payer.as_ref()),
        &signers.0,
        recent_blockhash.into_inner(),
    )
    .into()
}

#[wasm_bindgen]
pub fn init_rust_logs() {
    console_log::init().unwrap();
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use solana_api_types::{
        RpcAccountInfoConfig, RpcSignatureStatusConfig, RpcTransactionConfig, UiAccountEncoding,
        UiTransactionEncoding,
    };

    use super::*;

    fn init_client() -> RawApiClient {
        RawApiClient::new("http://api.devnet.solana.com")
    }

    #[tokio::test]
    async fn test_get_account_info() {
        let client = init_client();
        let pubkey = Pubkey::try_from("4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA").unwrap();
        let _r = client.get_account_info(pubkey, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_program_accounts() {
        let client = init_client();
        let pubkey = Pubkey::try_from("4Nd1mBQtrMJVYVfKf2PJy9NZUZdTAsp7D4xWLs4gDB4T").unwrap();
        let _r = client.get_program_accounts(pubkey, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_multiple_accounts() {
        let client = init_client();
        let accounts = &[
            Pubkey::try_from("vines1vzrYbzLMRdu58ou5XTby4qAqVRLmqo36NKPTg").unwrap(),
            Pubkey::try_from("4fYNw3dojWmQ4dXtSGE9epjRGy9pFSx62YypT7avPYvA").unwrap(),
        ];
        let _r = client
            .get_multiple_accounts(
                accounts,
                Some(RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base58),
                    data_slice: None,
                    commitment: None,
                }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_signature_statuses() {
        let client = init_client();
        let signatures = &[
            Signature::try_from("5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW").unwrap(),
            Signature::try_from("5j7s6NiJS3JAkvgkoc18WVAsiSaci2pxB2A6ueCJP4tprA2TFg9wSyTLeYouxPBJEMzJinENTkpA52YStRW5Dia7").unwrap(),
        ];
        let _r = client
            .get_signature_statuses(
                signatures,
                Some(RpcSignatureStatusConfig {
                    search_transaction_history: true,
                }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_signatures_for_address() {
        let client = init_client();
        let address = Pubkey::try_from("Vote111111111111111111111111111111111111111").unwrap();
        let _r = client
            .get_signatures_for_address(
                &address,
                Some(RpcSignaturesForAddressConfig {
                    limit: Some(1),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_slot() {
        let client = init_client();
        let _r = client.get_slot(None).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_transaction() {
        let client = init_client();
        let signature = Signature::from_str(
            "2nBhEBYYvfaAe16UMNqRHre4YNSskvuYgx3M6E4JP1oDYvZEJHvoPzyUidNgNX5r9sTyN1J9UxtbCXy2rqYcuyuv",
        )
        .unwrap();
        let _r = client
            .get_transaction(
                signature,
                Some(RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::Json),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_request_airdrop() {
        let client = init_client();
        let pubkey = Pubkey::from_str("83astBRguLMdt2h5U1Tpdq5tjFoJ6noeGwaY3mDLVcri").unwrap();
        let _r = client
            .request_airdrop(&pubkey, 1000000000, None)
            .await
            .unwrap();
    }

    // TODO: send_transaction
    // TODO: simulate_transaction
}
