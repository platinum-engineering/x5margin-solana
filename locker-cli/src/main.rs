use std::{collections::HashMap, convert::TryInto};

use parity_scale_codec::Encode;
use solana_api_types::{Instruction, Keypair, Pubkey, Signer, Transaction};
use solar::spl::{create_mint, create_wallet};
use solar_macros::parse_pubkey;
use structopt::StructOpt;

#[macro_use]
extern crate serde;

#[derive(StructOpt)]
enum Command {
    CreateMint {
        tag: String,
    },
    CreateWallet {
        mint: String,
        tag: String,
    },

    CreateLocker {
        mint: String,
        source_wallet: String,
        tag: String,
    },
    Withdraw,
    Increment,
    Init,
}

#[derive(Serialize, Deserialize)]
pub struct ClientStore {
    payer: Vec<u8>,
    mints: HashMap<String, Vec<u8>>,
    wallets: HashMap<String, Vec<u8>>,
    lockers: HashMap<String, Vec<u8>>,
    locker_owners: HashMap<String, Vec<u8>>,
}

impl ClientStore {
    pub fn payer(&self) -> Keypair {
        Keypair::from_bytes(&self.payer).unwrap()
    }

    pub fn mint(&self, tag: &str) -> Keypair {
        Keypair::from_bytes(self.mints.get(tag).expect("missing mint")).unwrap()
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
    serde_json::from_slice(&file).expect("couldn't parse settings")
}

pub fn store_settings(settings: &ClientStore) {
    let data = serde_json::to_vec(settings).expect("couldn't serialize settings");
    std::fs::write("store.json", &data).expect("couldn't write settings");
}

const LOCKER_PROGRAM_ID: Pubkey = parse_pubkey!("8HQopi9Ve16NAQ5ni7EbR3P5yvrLRHE8RBLoC5ZDTsR9");

#[async_std::main]
pub async fn main() -> anyhow::Result<()> {
    let command = Command::from_args();

    env_logger::init();

    let client = solana_rpc_client::SolanaApiClient::new("http://localhost:8899".into());
    let client = solar::offchain::client::SolanaClient::start(
        client,
        "ws://localhost:8900".try_into().unwrap(),
    )
    .await?;

    match command {
        Command::CreateLocker {
            mint,
            source_wallet,
            tag,
        } => {
            let mut settings = load_settings();
            let payer = settings.payer();
            let mint = settings.mint(&mint);
            let source_wallet = settings.wallet(&source_wallet);
            let locker = Keypair::new();
            let vault = Keypair::new();
            let owner = Keypair::new();

            let create_mint_accounts = token_locker::instructions::CreateArgs {
                token_program: (*solar::spl::ID).into(),
                locker: locker.pubkey().into(),
                source_wallet: source_wallet.pubkey().into(),
                source_authority: payer.pubkey().into(),
                vault: vault.pubkey().into(),
                program_authority: Pubkey::default().into(),
                owner_authority: owner.pubkey().into(),
            }
            .metas();

            let instruction_data = token_locker::Method::CreateLock {
                unlock_date: 0.into(),
                amount: 1_000_000.into(),
            }
            .encode();

            let mut instructions = vec![];
            instructions.extend_from_slice(&solar::spl::create_wallet(
                &payer.pubkey(),
                &vault.pubkey(),
                &mint.pubkey(),
                &payer.pubkey(),
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
                [&payer, &source_wallet, &locker, &vault, &owner],
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
            let keypair = Keypair::new();

            let settings = ClientStore {
                payer: keypair.to_bytes().into(),
                mints: Default::default(),
                wallets: Default::default(),
                lockers: Default::default(),
                locker_owners: Default::default(),
            };
            store_settings(&settings);

            client
                .request_airdrop(&keypair.pubkey(), 1_000_000_000)
                .await?;
        }
        Command::CreateMint { tag } => {
            let mut settings = load_settings();
            let payer = settings.payer();
            let mint = Keypair::new();
            let hash = client.recent_blockhash();

            let instructions = create_mint(&payer.pubkey(), &mint.pubkey(), &payer.pubkey(), 6);
            let trx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&payer.pubkey()),
                [&payer, &mint],
                hash,
            );
            client.process_transaction(&trx).await?;

            println!("created mint {} - {}", tag, mint.pubkey());
            settings.mints.insert(tag, mint.to_bytes().into());
            store_settings(&settings);
        }
        Command::CreateWallet { mint, tag } => {
            let mut settings = load_settings();
            let payer = settings.payer();
            let mint = settings.mint(&mint);
            let wallet = Keypair::new();
            let hash = client.recent_blockhash();

            let instructions = create_wallet(
                &payer.pubkey(),
                &wallet.pubkey(),
                &mint.pubkey(),
                &payer.pubkey(),
            );
            let trx = Transaction::new_signed_with_payer(
                &instructions,
                Some(&payer.pubkey()),
                [&payer, &wallet],
                hash,
            );
            client.process_transaction(&trx).await?;

            println!("created wallet {} - {}", tag, wallet.pubkey());
            settings.wallets.insert(tag, wallet.to_bytes().into());
            store_settings(&settings);
        }
    }

    Ok(())
}
