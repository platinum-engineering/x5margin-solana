use std::sync::Arc;

use async_trait::async_trait;
use anyhow::{anyhow, Result};

use solana_api_types::{Pubkey, Signature, Transaction};

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use url::Url;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Cluster {
    Devnet,
    Testnet,
    MainnetBeta,
    Custom(String),
}

impl Cluster {
    pub fn url(&self) -> &str {
        match self {
            Cluster::Devnet => "https://api.devnet.solana.com",
            Cluster::Testnet => "https://api.testnet.solana.com",
            Cluster::MainnetBeta => "https://api.mainnet-beta.solana.com",
            Cluster::Custom(url) => url,
        }
    }
}

impl Default for Cluster {
    fn default() -> Self {
        Cluster::Devnet
    }
}

impl std::fmt::Display for Cluster {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let c_name = match self {
            Cluster::Devnet => "devnet",
            Cluster::Testnet => "testnet",
            Cluster::MainnetBeta => "mainnet",
            Cluster::Custom(url) => url,
        };
        write!(f, "{}", c_name)
    }
}

impl FromStr for Cluster {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Cluster> {
        match s.to_lowercase().as_str() {
            "d" | "devnet" => Ok(Cluster::Devnet),
            "t" | "testnet" => Ok(Cluster::Testnet),
            "m" | "mainnet" => Ok(Cluster::MainnetBeta),
            url if url.contains("http") => {
                Ok(Cluster::Custom(url.to_string()))
            }
            _ => Err(anyhow::Error::msg(
                "you can use 'devnet' or 'testnet' or 'mainnet' or an http/https url for cluster\n",
            )),
        }
    }
}


struct Account {
    pk: Pubkey,
    inner: Option<Arc<AccountData>>
}

struct AccountData {
    connector: Arc<dyn SolanaConnector>,
    key: Pubkey,
    owner: Pubkey,
}

#[async_trait]
trait SolanaConnector {
    async fn get_account(&self, pk: &Pubkey) -> Result<AccountData>;
    async fn send_transaction(&self, transaction: Transaction) -> Result<Signature>;
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_cluster(name: &str, cluster: Cluster) {
        assert_eq!(Cluster::from_str(name).unwrap(), cluster);
    }

    #[test]
    fn cluster_endpoints_test() {
        test_cluster("devnet", Cluster::Devnet);
        test_cluster("testnet", Cluster::Testnet);
        test_cluster("mainnet", Cluster::MainnetBeta);
    }

    #[test]
    #[should_panic]
    fn cluster_bad_url_test() {
        let bad_url = "htts://bad_url.net";
        Cluster::from_str(bad_url).unwrap();
    }
}