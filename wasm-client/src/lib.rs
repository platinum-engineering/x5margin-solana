#![allow(clippy::new_ret_no_self)]

pub mod locker;
pub mod web3;
pub mod web_wallet;

use std::{convert::TryInto, fmt::Display, sync::Arc};

use log::Level;
use solana_rpc_client::SolanaApiClient;
use solar::offchain::client::SolanaClient;
use wasm_bindgen::prelude::*;

extern crate solana_api_types as solana;

pub trait ResultExt<T> {
    fn into_js_result(self) -> Result<T, JsValue>;
}

pub trait ErrorExt {
    fn into_js_error(self) -> JsValue
    where
        Self: Sized;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Display,
{
    fn into_js_result(self) -> Result<T, JsValue> {
        self.map_err(|err| JsValue::from_str(&format!("{}", err)))
    }
}

impl<E> ErrorExt for E
where
    E: Display,
{
    fn into_js_error(self) -> JsValue {
        JsValue::from_str(&format!("{}", self))
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Client {
    pub(crate) inner: SolanaClient,
}

#[wasm_bindgen]
impl Client {
    pub async fn new(http_url: String, ws_url: String) -> Result<Client, JsValue> {
        let ws_url = ws_url.as_str();

        let rpc_client = SolanaApiClient::new(http_url).into_js_result()?;
        let rpc_client = Arc::new(rpc_client);
        let solana_client = SolanaClient::start(rpc_client, ws_url.try_into().into_js_result()?)
            .await
            .into_js_result()?;

        let client = Client {
            inner: solana_client,
        };

        Ok(client)
    }
}

// #[wasm_bindgen]
// #[derive(Clone, Copy)]
// pub struct Pubkey(solana::Pubkey);

// #[wasm_bindgen]
// impl Pubkey {
//     pub fn new(key: &str) -> Result<Pubkey, JsValue> {
//         Ok(Self(solana::Pubkey::from_str(key).into_js_result()?))
//     }
// }

// impl AsRef<solana::Pubkey> for Pubkey {
//     fn as_ref(&self) -> &solana::Pubkey {
//         &self.0
//     }
// }

// #[wasm_bindgen]
// pub struct Signature(solana::Signature);

// #[wasm_bindgen]
// impl Signature {
//     pub fn new(signature: &str) -> Result<Signature, JsValue> {
//         Ok(Self(
//             solana::Signature::from_str(signature).into_js_result()?,
//         ))
//     }
// }

// #[wasm_bindgen]
// #[derive(Serialize)]
// pub struct MintAccount {
//     account: solar::spl::MintAccount<Box<solana::Account>>,
// }

// impl MintAccount {
//     pub fn any(account: solana::Account) -> Result<Self, solar::error::SolarError> {
//         let account = Box::new(account);
//         let account = solar::spl::MintAccount::any(account)?;

//         Ok(MintAccount { account })
//     }

//     pub fn wallet(
//         &self,
//         account: solana::Account,
//     ) -> Result<WalletAccount, solar::error::SolarError> {
//         let account = Box::new(account);
//         let account = self.account.wallet(account)?;

//         Ok(WalletAccount { account })
//     }
// }

// #[wasm_bindgen]
// #[derive(Serialize)]
// pub struct WalletAccount {
//     account: solar::spl::WalletAccount<Box<solana::Account>>,
// }

// impl WalletAccount {
//     pub fn any(account: solana::Account) -> Result<Self, solar::error::SolarError> {
//         let account = Box::new(account);
//         let account = solar::spl::WalletAccount::any(account)?;

//         Ok(WalletAccount { account })
//     }
// }

// #[wasm_bindgen]
// #[derive(Clone)]
// pub struct Instruction(solana::Instruction);

// impl From<solana::Instruction> for Instruction {
//     fn from(i: solana::Instruction) -> Self {
//         Self(i)
//     }
// }

// #[wasm_bindgen]
// pub struct Instructions {
//     inner: Vec<solana::Instruction>,
// }

// impl<const N: usize> From<[solana::Instruction; N]> for Instructions {
//     fn from(src: [solana::Instruction; N]) -> Self {
//         Self {
//             inner: src.to_vec(),
//         }
//     }
// }

// impl AsRef<[solana::Instruction]> for Instructions {
//     fn as_ref(&self) -> &[solana::Instruction] {
//         self.inner.as_ref()
//     }
// }

// #[wasm_bindgen]
// impl Instructions {
//     pub fn push(&mut self, i: Instruction) {
//         self.inner.push(i.0);
//     }
// }

// #[wasm_bindgen]
// pub fn create_mint(payer: Pubkey, mint: Pubkey, authority: Pubkey, decimals: u8) -> Instructions {
//     solar::spl::create_mint(payer.as_ref(), mint.as_ref(), authority.as_ref(), decimals).into()
// }

// #[wasm_bindgen]
// pub fn create_wallet(
//     payer: Pubkey,
//     wallet: Pubkey,
//     mint: Pubkey,
//     authority: Pubkey,
// ) -> Instructions {
//     solar::spl::create_wallet(
//         payer.as_ref(),
//         wallet.as_ref(),
//         mint.as_ref(),
//         authority.as_ref(),
//     )
//     .into()
// }

// #[wasm_bindgen]
// pub fn mint_to(mint: Pubkey, wallet: Pubkey, authority: Pubkey, amount: u64) -> Instruction {
//     solar::spl::mint_to(mint.as_ref(), wallet.as_ref(), authority.as_ref(), amount).into()
// }

// #[wasm_bindgen]
// pub fn create_account(
//     from_pubkey: Pubkey,
//     to_pubkey: Pubkey,
//     lamports: u64,
//     space: u64,
//     owner: Pubkey,
// ) -> Instruction {
//     solana::system::create_account(
//         from_pubkey.as_ref(),
//         to_pubkey.as_ref(),
//         lamports,
//         space,
//         owner.as_ref(),
//     )
//     .into()
// }

// #[wasm_bindgen]
// pub struct Hash(solana::Hash);

// #[wasm_bindgen]
// pub struct Transaction(solana::Transaction);

// impl From<solana::Transaction> for Transaction {
//     fn from(tx: solana::Transaction) -> Self {
//         Self(tx)
//     }
// }

// impl AsRef<solana::Transaction> for Transaction {
//     fn as_ref(&self) -> &solana::Transaction {
//         &self.0
//     }
// }

#[wasm_bindgen(start)]
pub fn main() {
    console_log::init_with_level(Level::Debug).unwrap();
}
