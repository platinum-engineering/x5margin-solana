//! Contains functionality to brige Rust-based WASM modules and Solana's web3 JS API
//!
//!

use std::convert::TryInto;

use wasm_bindgen::{prelude::*, JsCast};

use js_sys::{Object, Uint8Array};
use solana::{Instruction, Pubkey, Transaction};

#[wasm_bindgen]
extern "C" {
    pub type Web3SerializeConfig;

    #[wasm_bindgen(method, setter, js_name = "requireAllSignatures")]
    pub fn set_require_all_signatures(this: &Web3SerializeConfig, value: bool);

    #[wasm_bindgen(method, setter, js_name = "verifySignatures")]
    pub fn set_verify_signatures(this: &Web3SerializeConfig, value: bool);
}

impl Default for Web3SerializeConfig {
    fn default() -> Self {
        let object = Object::default();

        let this: Web3SerializeConfig = object.unchecked_into();
        this.set_require_all_signatures(false);
        this.set_verify_signatures(false);
        this
    }
}

#[wasm_bindgen(module = "@solana/web3.js")]
extern "C" {
    #[wasm_bindgen(js_name = "PublicKey")]
    pub type Web3Pubkey;

    #[wasm_bindgen(constructor, js_class = "PublicKey")]
    pub fn new_raw(arg: Uint8Array) -> Web3Pubkey;

    #[wasm_bindgen(method, js_name = "toBytes")]
    pub fn to_bytes(pk: &Web3Pubkey) -> Uint8Array;
}

#[wasm_bindgen(module = "@solana/web3.js")]
extern "C" {
    #[wasm_bindgen(js_name = "Transaction")]
    pub type Web3Transaction;

    #[wasm_bindgen(static_method_of = Web3Transaction, js_class = "Transaction")]
    pub fn from(buffer: Uint8Array) -> Web3Transaction;

    #[wasm_bindgen(method, js_class = "Transaction")]
    pub fn serialize(this: &Web3Transaction, config: &Web3SerializeConfig) -> Uint8Array;
}

impl From<Pubkey> for Web3Pubkey {
    fn from(other: Pubkey) -> Self {
        let array = Uint8Array::from(other.as_bytes().as_ref());
        Web3Pubkey::new_raw(array)
    }
}

impl From<Web3Pubkey> for Pubkey {
    fn from(other: Web3Pubkey) -> Self {
        let array = other.to_bytes();
        let slice = array.to_vec();
        Pubkey::new(slice.try_into().expect("expected 32 bytes"))
    }
}

impl From<Transaction> for Web3Transaction {
    fn from(other: Transaction) -> Self {
        let encoded = other.encode_bincode();
        Web3Transaction::from(encoded.as_slice().into())
    }
}

impl From<Web3Transaction> for Transaction {
    fn from(other: Web3Transaction) -> Self {
        let encoded = other.serialize(&Default::default());
        let encoded = encoded.to_vec();
        Self::decode_bincode(&encoded).expect("failed conversion from web3 trx")
    }
}
