use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig,
        pubkey::Pubkey,
        signature::{read_keypair_file, Signer},
        system_instruction,
        sysvar::clock,
    },
    Client,
};
use anyhow::{anyhow, Result};

use structopt::StructOpt;

#[derive(Debug)]
struct CliKeypair<A> {
    path: String,
    ty: std::marker::PhantomData<A>,
}

impl<A> std::fmt::Display for CliKeypair<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.path)
    }
}

impl<A> std::str::FromStr for CliKeypair<A> {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            path: s.to_string(),
            ty: std::marker::PhantomData {},
        })
    }
}

impl<A> AsRef<String> for CliKeypair<A> {
    fn as_ref(&self) -> &String {
        &self.path
    }
}

impl<A> Default for CliKeypair<A>
where
    A: DefaultPath,
{
    fn default() -> Self {
        Self {
            path: A::default_path(),
            ty: std::marker::PhantomData {},
        }
    }
}

trait DefaultPath {
    fn default_path() -> String;
}

#[derive(Debug)]
struct Payer;

impl DefaultPath for Payer {
    fn default_path() -> String {
        shellexpand::tilde("~/.config/solana/id.json").to_string()
    }
}

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(long)]
    pool_program_id: Pubkey,
    #[structopt(long)]
    cluster: anchor_client::Cluster,
    #[structopt(long, default_value)]
    payer: CliKeypair<Payer>,
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Generate program derived address.
    GeneratePDA {
        #[structopt(long)]
        administrator: Pubkey,
        #[structopt(long)]
        pool: Pubkey,
    },
    /// Initialize stake pool.
    Initialize {
        #[structopt(long)]
        administrator: CliKeypair<()>,
        #[structopt(long)]
        pool_authority: Pubkey,
        #[structopt(long)]
        pool: CliKeypair<()>,
        #[structopt(long)]
        stake_mint: Pubkey,
        #[structopt(long)]
        stake_vault: Pubkey,
        #[structopt(long)]
        nonce: u8,
        #[structopt(long)]
        lockup_duration: i64,
        #[structopt(long)]
        topup_duration: i64,
        #[structopt(long)]
        reward_amount: u64,
        #[structopt(long)]
        target_amount: u64,
    },
}

fn main() -> Result<()> {
    let opts = Opts::from_args();

    let payer = read_keypair_file(opts.payer.as_ref())
        .map_err(|err| anyhow!("failed to read keypair: {}", err))?;

    let client = Client::new_with_options(opts.cluster, payer, CommitmentConfig::processed());
    let pool_client = client.program(opts.pool_program_id);

    match opts.cmd {
        Command::GeneratePDA {
            administrator,
            pool,
        } => {
            let (pda_key, nonce) = Pubkey::find_program_address(
                &[pool.as_ref(), administrator.as_ref()],
                &opts.pool_program_id,
            );

            println!("Generated PDA: {}\nNonce: {}", pda_key, nonce);
        }
        Command::Initialize {
            administrator,
            pool_authority,
            pool,
            stake_mint,
            stake_vault,
            nonce,
            lockup_duration,
            topup_duration,
            reward_amount,
            target_amount,
        } => {
            let administrator = read_keypair_file(administrator.as_ref())
                .map_err(|err| anyhow!("failed to read keypair: {}", err))?;
            let pool = read_keypair_file(pool.as_ref())
                .map_err(|err| anyhow!("failed to read keypair: {}", err))?;

            let r = pool_client
                .request()
                .instruction(system_instruction::create_account(
                    &pool_client.payer(),
                    &pool.pubkey(),
                    pool_client
                        .rpc()
                        .get_minimum_balance_for_rent_exemption(500)?,
                    500,
                    &pool_client.id(),
                ))
                .accounts(pool::accounts::InitializePool {
                    administrator_authority: administrator.pubkey(),
                    pool_authority,
                    pool: pool.pubkey(),
                    stake_mint,
                    stake_vault,
                    clock: clock::ID,
                })
                .args(pool::instruction::InitializePool {
                    nonce,
                    lockup_duration,
                    topup_duration,
                    reward_amount,
                    target_amount,
                })
                .signer(&administrator)
                .signer(&pool)
                .send()?;
        }
    }

    Ok(())
}
