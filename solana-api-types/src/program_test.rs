use crate::{Account, Hash, Keypair, Pubkey, Transaction};
use solana_program_test::BanksClient;
use solana_sdk::process_instruction::ProcessInstructionWithContext;

use crate::sdk_proxy::ToSdk;

#[derive(Default)]
pub struct ProgramTest {
    inner: solana_program_test::ProgramTest,
}

impl ProgramTest {
    pub fn add_program(
        &mut self,
        name: &str,
        pk: Pubkey,
        handler: Option<ProcessInstructionWithContext>,
    ) {
        self.inner.add_program(name, pk.to_sdk(), handler)
    }

    pub async fn start(self) -> (Runtime, Keypair, Hash) {
        let (client, keypair, hash) = self.inner.start().await;

        let keypair = Keypair::from_bytes(&keypair.to_bytes()).unwrap();
        let hash = Hash(hash.0);

        (Runtime { client }, keypair, hash)
    }
}

pub struct Runtime {
    client: BanksClient,
}

impl Runtime {
    pub async fn process_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), anyhow::Error> {
        self.client
            .process_transaction(transaction.to_sdk())
            .await
            .map_err(|err| err.into())
    }

    pub async fn get_account(&mut self, pk: &Pubkey) -> Result<Option<Account>, anyhow::Error> {
        self.client
            .get_account(pk.to_sdk())
            .await
            .map(|s| {
                s.map(|account| Account {
                    lamports: account.lamports,
                    data: account.data,
                    owner: Pubkey::new(account.owner.to_bytes()),
                    executable: account.executable,
                    rent_epoch: account.rent_epoch,
                    pubkey: *pk,
                })
            })
            .map_err(|err| err.into())
    }
}
