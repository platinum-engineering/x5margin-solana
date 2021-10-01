use solana_api_types::Pubkey;

use crate::{
    account::{pubkey::PubkeyAccount, AccountFields},
    error::SolarError,
    prelude::AccountBackend,
};

#[derive(Debug)]
pub struct Authority<B: AccountBackend> {
    account: B,
}

impl From<Pubkey> for Authority<PubkeyAccount> {
    fn from(pubkey: Pubkey) -> Self {
        Self {
            account: pubkey.into(),
        }
    }
}

impl<B: AccountBackend> Authority<B> {
    pub fn any(account: B) -> Authority<B> {
        Self { account }
    }

    pub fn any_signed(account: B) -> Result<Authority<B>, SolarError> {
        if !account.is_signer() {
            Err(SolarError::NotSigned)
        } else {
            Ok(Self { account })
        }
    }

    pub fn expected(account: B, expected: &Pubkey) -> Result<Authority<B>, SolarError> {
        if account.key() != expected {
            Err(SolarError::InvalidAuthority)
        } else {
            Ok(Self { account })
        }
    }

    pub fn expected_signed(account: B, expected: &Pubkey) -> Result<Authority<B>, SolarError> {
        let authority = Self::expected(account, expected)?;

        if !authority.account.is_signer() {
            Err(SolarError::NotSigned)
        } else {
            Ok(authority)
        }
    }

    pub fn key(&self) -> &Pubkey {
        self.account.key()
    }

    pub fn account(&self) -> &B {
        &self.account
    }
}
