use crate::web3::*;
use js_sys::{global, Object, Promise, Reflect};
use solana::{Pubkey, Transaction};
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(getter, js_name = "signTransaction", js_namespace = ["window", "solana"])]
    fn __sign_transaction(transaction: Web3Transaction) -> Promise;
}

pub fn solana_object() -> Option<Object> {
    if let Ok(window) = Reflect::get(&global().into(), &"window".into()) {
        if let Ok(solana) = Reflect::get(&window, &"solana".into()) {
            if let Ok(solana) = solana.dyn_into() {
                return Some(solana);
            }
        }
    }

    None
}

pub fn public_key() -> Option<Pubkey> {
    solana_object()
        .and_then(|solana| Reflect::get(&solana.into(), &"publicKey".into()).ok())
        .and_then(|value| value.dyn_into::<Web3Pubkey>().ok())
        .map(|pk| pk.into())
}

pub fn is_connected() -> bool {
    solana_object()
        .and_then(|solana| Reflect::get(&solana.into(), &"isConnected".into()).ok())
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

pub async fn sign_transaction(transaction: Transaction) -> Result<Transaction, JsValue> {
    let w3tx: Web3Transaction = transaction.clone().into();
    let w3tx = __sign_transaction(w3tx);
    let w3tx = JsFuture::from(w3tx)
        .await
        .map_err(|err| JsValue::from(format!("failed to sign transaction: {:?}", err)))?;
    let w3tx = w3tx.dyn_into::<Web3Transaction>()?;
    let transaction = w3tx.into();

    Ok(transaction)
}

pub fn is_installed() -> bool {
    let global = global();

    if let Ok(window) = Reflect::get(&global.into(), &"window".into()) {
        if let Ok(solana) = Reflect::get(&window, &"solana".into()) {
            if solana.is_object() {
                return true;
            }
        }
    }

    false
}
