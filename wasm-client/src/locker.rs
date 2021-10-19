use js_sys::Promise;
use log::{debug, error, info};
use parity_scale_codec::Encode;
use solana::{transaction, Instruction, Keypair, Pubkey, Signer, Transaction};
use solar::{entity::AccountType, offchain::client::SolanaClient};
use token_locker::{data::TokenLockEntity, UnlockDate};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::console::debug;

use crate::ResultExt;
use crate::{
    web3::Web3Pubkey,
    web_wallet::{is_installed, sign_transaction},
};

#[wasm_bindgen]
#[derive(Clone)]
pub struct LockerClient {
    client: SolanaClient,
}

#[wasm_bindgen]
impl LockerClient {
    #[wasm_bindgen(constructor)]
    pub fn new(client: &super::Client) -> LockerClient {
        Self {
            client: client.inner.clone(),
        }
    }

    #[wasm_bindgen(method)]
    pub fn create_token_locker(
        &self,
        program_id: Web3Pubkey,
        payer: Web3Pubkey,
        funding_wallet: Web3Pubkey,
        lp_mint: Web3Pubkey,
        amount: u64,
        unlock_date: i64,
    ) -> Promise {
        let client = self.client.clone();
        future_to_promise(async move {
            if !is_installed() {
                return Err("a wallet with signing capabilities must be installed".into());
            }

            let program_id: Pubkey = program_id.into();
            let funding_wallet: Pubkey = funding_wallet.into();
            let payer: Pubkey = payer.into();
            let lp_mint: Pubkey = lp_mint.into();

            let locker = Keypair::new();
            let vault = Keypair::new();

            let mut instructions = vec![];
            let (program_authority, nonce) = token_locker::data::find_locker_program_authority(
                &program_id,
                &locker.pubkey(),
                &payer,
                0,
            );
            instructions.extend_from_slice(&solar::spl::create_wallet(
                &payer,
                &vault.pubkey(),
                &lp_mint,
                &program_authority,
            ));

            instructions.push(TokenLockEntity::create_default(
                &program_id,
                &payer,
                &locker.pubkey(),
            ));

            let instruction = token_locker::instructions::CreateArgs::new(
                solar::spl::ID,
                &locker.pubkey(),
                &funding_wallet,
                &payer,
                &vault.pubkey(),
                &program_authority,
            );

            let accounts = instruction.metas();
            let data = token_locker::Method::CreateLock {
                amount: amount.into(),
                unlock_date: UnlockDate::Absolute(unlock_date.into()),
                nonce,
            }
            .encode();
            instructions.push(Instruction {
                program_id,
                accounts,
                data,
            });

            let recent_blockhash = client.recent_blockhash();
            let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer));
            transaction
                .try_partial_sign([&locker, &vault], recent_blockhash)
                .into_js_result()?;
            // transaction
            //     .try_partial_sign([&vault], recent_blockhash)
            //     .into_js_result()?;

            let transaction = sign_transaction(transaction).await?;

            debug!("instructions: {:#?}", &instructions);

            debug!("payer: {}", &payer);
            debug!("lp_mint: {}", &lp_mint);
            debug!("vault: {}", &vault.pubkey());
            debug!("locker: {}", &locker.pubkey());
            debug!("funding_wallet: {}", &funding_wallet);
            debug!("program_authority: {}", &program_authority);

            debug!("trx(rust): {:#?}", &transaction);

            if let Err(error) = transaction.verify() {
                error!("transaction verification failed: {}", error);
                panic!();
            }

            client
                .process_transaction(&transaction)
                .await
                .into_js_result()?;

            info!("create locker transaction successfully processed!");

            Ok(JsValue::undefined())
        })
    }
}
