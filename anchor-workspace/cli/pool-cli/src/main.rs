use anchor_client::{
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair_file,
    },
    Client,
};
use anyhow::{anyhow, Result};

use structopt::StructOpt;

#[derive(Debug)]
struct PayerKeypair(String);

impl std::fmt::Display for PayerKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

impl Default for PayerKeypair {
    fn default() -> Self {
        Self(shellexpand::tilde("~/.config/solana/id.json").to_string())
    }
}

impl std::str::FromStr for PayerKeypair {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
    }
}

impl AsRef<String> for PayerKeypair {
    fn as_ref(&self) -> &String {
        &self.0
    }
}

#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(long)]
    pool_program_id: Pubkey,
    #[structopt(long)]
    cluster: anchor_client::Cluster,
    #[structopt(long, default_value)]
    payer: PayerKeypair,
}

fn main() -> Result<()> {
    let opts = Opts::from_args();

    let payer = read_keypair_file(opts.payer.as_ref())
        .map_err(|err| anyhow!("failed to read keypair: {}", err))?;

    let client = Client::new_with_options(opts.cluster, payer, CommitmentConfig::processed());
    let pool = client.program(opts.pool_program_id);

    // TODO: initialize subcommand

    Ok(())
}
