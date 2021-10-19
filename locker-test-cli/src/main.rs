use std::{collections::HashMap, convert::TryInto, sync::Arc, time::Duration};

use anyhow::{anyhow, Context};
use log::info;
use parity_scale_codec::Encode;
use solana_api_types::{
    client::Client, system, CommitmentLevel, Instruction, Keypair, Pubkey, Signer, Transaction,
};
use solar::entity::AccountType;
use solar::{
    offchain::client::SolanaClient,
    spl::{create_mint, create_wallet, mint_to, WalletAccount},
};
use solar_macros::parse_pubkey;
use structopt::StructOpt;
use token_locker::{data::TokenLockEntity, UnlockDate};

use crate::{
    predefined::{
        btc_mint_keypair, default_authority_keypair, default_payer_keypair, ray_mint_keypair,
        usdt_mint_keypair,
    },
    util::{find_associated_wallet, initialize_associated_wallet},
};

#[macro_use]
extern crate serde;

#[macro_use]
extern crate indoc;

pub mod predefined;
pub mod util;
use predefined::*;

#[derive(StructOpt)]
enum Command {
    CreateMint {
        tag: String,
    },
    CreateWallet {
        mint: String,
        tag: String,
    },
    MintTokens {
        wallet: String,
        amount: u64,
    },

    CreateLocker {
        mint: String,
        source_wallet: String,
        tag: String,
    },
    GenerateLpJson,
    Withdraw,
    Increment,
    Init,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ClientStore {
    mints: HashMap<String, Vec<u8>>,
    wallets: HashMap<String, Vec<u8>>,
    associated_wallets: HashMap<String, Vec<u8>>,
    lockers: HashMap<String, Vec<u8>>,
    locker_owners: HashMap<String, Vec<u8>>,
}

impl ClientStore {
    pub fn mint(&self, tag: &str) -> Keypair {
        Keypair::from_bytes(self.mints.get(tag).expect("missing mint")).unwrap()
    }

    pub fn mint_by_pubkey(&self, pubkey: &Pubkey) -> Keypair {
        self.mints
            .iter()
            .filter_map(|(_, key)| {
                let keypair = Keypair::from_bytes(key).unwrap();
                if &keypair.pubkey() == pubkey {
                    Some(keypair)
                } else {
                    None
                }
            })
            .next()
            .expect("missing mint")
    }

    pub fn associated_wallet(&self, tag: &str) -> Pubkey {
        Pubkey::new(
            self.associated_wallets
                .get(tag)
                .expect("missing wallet")
                .as_slice()
                .try_into()
                .unwrap(),
        )
    }

    pub fn wallet(&self, tag: &str) -> Keypair {
        Keypair::from_bytes(self.wallets.get(tag).expect("missing wallet")).unwrap()
    }

    pub fn locker(&self, tag: &str) -> Keypair {
        Keypair::from_bytes(self.lockers.get(tag).expect("missing locker")).unwrap()
    }

    pub fn locker_owner(&self, tag: &str) -> Keypair {
        Keypair::from_bytes(self.locker_owners.get(tag).expect("missing locker owner")).unwrap()
    }
}

pub fn load_settings() -> ClientStore {
    let file = std::fs::read("store.json").expect("couldn't read settings");
    serde_json::from_slice::<ClientStore>(&file).expect("couldn't parse settings")
}

pub fn store_settings(settings: &ClientStore) {
    let data = serde_json::to_vec(settings).expect("couldn't serialize settings");
    std::fs::write("store.json", &data).expect("couldn't write settings");
}

const LOCKER_PROGRAM_ID: Pubkey = parse_pubkey!("8HQopi9Ve16NAQ5ni7EbR3P5yvrLRHE8RBLoC5ZDTsR9");

// const HTTP_URL: &str = "http://localhost:8899";
// const WS_URL: &str = "ws://localhost:8900";
const HTTP_URL: &str = "http://api.devnet.solana.com";
const WS_URL: &str = "ws://api.devnet.solana.com";

#[async_std::main]
pub async fn main() -> anyhow::Result<()> {
    let command = Command::from_args();

    env_logger::init();

    let client = Arc::new(solana_rpc_client::SolanaApiClient::new(HTTP_URL)?);
    client.set_default_commitment_level(CommitmentLevel::Confirmed);
    let client =
        solar::offchain::client::SolanaClient::start(client, WS_URL.try_into().unwrap()).await?;

    match command {
        Command::CreateLocker {
            mint,
            source_wallet,
            tag,
        } => {
            let mut settings = load_settings();
            let payer = default_payer_keypair();
            let authority = default_authority_keypair();
            let mint = settings.mint(&mint);
            let source_wallet = settings.wallet(&source_wallet);
            let locker = Keypair::new();
            let vault = Keypair::new();
            let owner = Keypair::new();

            let (program_authority, nonce) = token_locker::data::find_locker_program_authority(
                &LOCKER_PROGRAM_ID,
                &locker.pubkey(),
                &owner.pubkey(),
                0,
            );

            let create_mint_accounts = token_locker::instructions::CreateArgs {
                token_program: (*solar::spl::ID).into(),
                locker: locker.pubkey().into(),
                source_wallet: source_wallet.pubkey().into(),
                source_authority: authority.pubkey().into(),
                vault: vault.pubkey().into(),
                program_authority: program_authority.into(),
                // owner_authority: owner.pubkey().into(),
            }
            .metas();

            let instruction_data = token_locker::Method::CreateLock {
                unlock_date: UnlockDate::Relative(60),
                amount: 1_000_000.into(),
                nonce,
            }
            .encode();

            let mut instructions = vec![];
            instructions.extend_from_slice(&solar::spl::create_wallet(
                &payer.pubkey(),
                &vault.pubkey(),
                &mint.pubkey(),
                &program_authority,
            ));
            instructions.push(system::create_account(
                &payer.pubkey(),
                &locker.pubkey(),
                TokenLockEntity::default_lamports(),
                TokenLockEntity::default_size() as u64,
                &LOCKER_PROGRAM_ID,
            ));
            instructions.push(Instruction {
                program_id: LOCKER_PROGRAM_ID,
                accounts: create_mint_accounts,
                data: instruction_data,
            });

            let hash = client.recent_blockhash();
            let trx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&payer.pubkey()),
                [&payer, &authority, &locker, &vault, &owner],
                hash,
            );
            client.process_transaction(&trx).await?;

            settings
                .lockers
                .insert(tag.clone(), locker.to_bytes().into());
            settings.locker_owners.insert(tag, owner.to_bytes().into());
            store_settings(&settings);
        }
        Command::Withdraw => todo!(),
        Command::Increment => todo!(),
        Command::Init => {
            init_environment(&client).await?;
        }
        Command::CreateMint { tag } => {
            let mut settings = load_settings();
            create_test_mint(
                &client,
                &mut settings,
                tag,
                Keypair::new(),
                default_authority_keypair(),
            )
            .await?;
            store_settings(&settings);
        }
        Command::CreateWallet { mint, tag } => {
            let mut settings = load_settings();
            create_test_wallet(
                &client,
                &mut settings,
                tag,
                mint,
                Keypair::new(),
                default_authority_keypair(),
            )
            .await?;
            store_settings(&settings);
        }

        Command::MintTokens { wallet, amount } => {
            let settings = load_settings();
            let payer = default_payer_keypair();
            let authority = default_authority_keypair();
            let wallet = if settings.wallets.contains_key(&wallet) {
                settings.wallet(&wallet).pubkey()
            } else {
                settings.associated_wallet(&wallet)
            };
            let hash = client.recent_blockhash();
            let wallet_account = client.load::<WalletAccount<_>>(&wallet).await?;
            let mint = settings.mint_by_pubkey(wallet_account.mint());

            let instructions = [mint_to(
                &mint.pubkey(),
                &wallet,
                &authority.pubkey(),
                amount,
            )];
            let trx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&payer.pubkey()),
                [&payer, &authority],
                hash,
            );
            client.process_transaction(&trx).await?;

            info!("minted {} tokens to {}", amount, &wallet);
        }
        Command::GenerateLpJson => {
            let json = predefined::generate_raydium_lp_json();
            let json = serde_json::to_string_pretty(&json).expect("serialize");
            std::fs::write("raydium-lps.json", json).expect("write");
        }
    }

    Ok(())
}

async fn init_environment(client: &SolanaClient) -> anyhow::Result<()> {
    let payer = default_payer_keypair();
    let mut settings = ClientStore::default();

    info!("payer is {}", payer.pubkey());
    info!("requesting airdrop for payer");

    client
        .request_airdrop(&payer.pubkey(), 1_000_000_000)
        .await
        .context("airdrop failed")?;

    async_std::task::sleep(Duration::from_millis(10000)).await;

    info!("requesting airdrop for authority");

    client
        .request_airdrop(&default_authority_keypair().pubkey(), 1_000_000_000)
        .await
        .context("airdrop failed")?;

    info!("airdrop completed");

    info!("creating mints and wallets for built-in tokens");
    for (tag, keypair) in [
        ("USDC", usdc_mint_keypair()),
        ("USDT", usdt_mint_keypair()),
        ("BTC", btc_mint_keypair()),
        ("RAY", ray_mint_keypair()),
        ("USDC-BTC", usdc_btc_lp_mint_keypair()),
        ("USDC-RAY", usdc_ray_lp_mint_keypair()),
        ("BTC-RAY", btc_ray_lp_mint_keypair()),
        ("USDT-BTC", usdt_btc_lp_mint_keypair()),
    ] {
        info!("creating mint and wallet for {}", tag);
        info!("creating mint");

        create_test_mint(
            client,
            &mut settings,
            tag.into(),
            keypair,
            default_authority_keypair(),
        )
        .await
        .with_context(|| anyhow!("couldn't create mint for {}", tag))?;
        let wallet = Keypair::new();

        info!("creating wallet");

        create_test_wallet(
            client,
            &mut settings,
            format!("{}-wallet", tag),
            tag.into(),
            wallet,
            default_authority_keypair(),
        )
        .await
        .with_context(|| anyhow!("couldn't create default wallet for mint {}", tag))?;

        info!("creating associated wallet");

        create_associated_wallet(
            client,
            &mut settings,
            format!("{}-assoc-wallet", tag),
            tag.into(),
            default_authority_keypair(),
        )
        .await
        .with_context(|| anyhow!("couldn't create associated wallet for mint {}", tag))?;

        info!("created mint and wallets for {}", tag);
    }

    store_settings(&settings);

    Ok(())
}

async fn create_test_mint(
    client: &SolanaClient,
    settings: &mut ClientStore,
    tag: String,
    mint: Keypair,
    authority: Keypair,
) -> anyhow::Result<()> {
    let payer = default_payer_keypair();
    let hash = client.recent_blockhash();

    let instructions = create_mint(&payer.pubkey(), &mint.pubkey(), &authority.pubkey(), 6);
    let trx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        [&payer, &mint],
        hash,
    );
    info!("creating mint {} - {}", tag, mint.pubkey());
    client.process_transaction(&trx).await?;

    info!("created mint {} - {}", tag, mint.pubkey());
    settings.mints.insert(tag, mint.to_bytes().into());

    Ok(())
}

async fn create_test_wallet(
    client: &SolanaClient,
    settings: &mut ClientStore,
    tag: String,
    mint: String,
    wallet: Keypair,
    authority: Keypair,
) -> anyhow::Result<()> {
    let payer = default_payer_keypair();
    let mint = settings.mint(&mint);
    let hash = client.recent_blockhash();

    let instructions = create_wallet(
        &payer.pubkey(),
        &wallet.pubkey(),
        &mint.pubkey(),
        &authority.pubkey(),
    );
    let trx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        [&payer, &wallet],
        hash,
    );
    info!("creating wallet {} - {}", tag, wallet.pubkey());
    client.process_transaction(&trx).await?;

    info!("created wallet {} - {}", tag, wallet.pubkey());
    settings.wallets.insert(tag, wallet.to_bytes().into());

    Ok(())
}

async fn create_associated_wallet(
    client: &SolanaClient,
    settings: &mut ClientStore,
    tag: String,
    mint: String,
    authority: Keypair,
) -> anyhow::Result<()> {
    let payer = default_payer_keypair();
    let mint = settings.mint(&mint);
    let hash = client.recent_blockhash();
    let wallet = find_associated_wallet(&authority.pubkey(), &mint.pubkey());

    let instructions = [initialize_associated_wallet(
        &payer.pubkey(),
        &authority.pubkey(),
        &mint.pubkey(),
    )];
    let trx =
        Transaction::new_signed_with_payer(&instructions, Some(&payer.pubkey()), [&payer], hash);
    info!(
        "creating associated {} wallet for {}",
        tag,
        authority.pubkey()
    );
    client.process_transaction(&trx).await?;

    info!(
        "created associated {} wallet for {}",
        tag,
        authority.pubkey()
    );
    settings
        .associated_wallets
        .insert(tag, wallet.as_ref().into());

    Ok(())
}
