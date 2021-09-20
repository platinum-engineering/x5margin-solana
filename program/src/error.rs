use solana_api_types::program::ProgramError;
use solar::{error::SolarError, spl::TokenError};

#[derive(IntoStaticStr, Debug, Display)]
pub enum Error {
    InvalidData,
    InvalidAlignment,
    InvalidOwner,
    InvalidParent,
    InvalidKind,
    InvalidAuthority,
    InvalidMint,
    InvalidAccount,
    NotRentExempt,
    Validation,
    TokenError(TokenError),
    SolarError(SolarError),
}

impl From<TokenError> for Error {
    fn from(other: TokenError) -> Self {
        Self::TokenError(other)
    }
}

impl From<Error> for ProgramError {
    fn from(_: Error) -> Self {
        todo!()
    }
}

impl From<SolarError> for Error {
    fn from(other: SolarError) -> Self {
        Self::SolarError(other)
    }
}
