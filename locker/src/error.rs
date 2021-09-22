use solana_api_types::program::ProgramError;
use solar::spl::{SplReadError, TokenError};

#[derive(Debug)]
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
    SplReadError(SplReadError),
    TokenError(TokenError),
}

impl Error {
    fn code(&self) -> u32 {
        match self {
            Error::InvalidData => 1,
            Error::InvalidAlignment => 2,
            Error::InvalidOwner => 3,
            Error::InvalidParent => 4,
            Error::InvalidKind => 5,
            Error::InvalidAuthority => 6,
            Error::InvalidMint => 7,
            Error::InvalidAccount => 8,
            Error::NotRentExempt => 9,
            Error::Validation => 10,
            Error::SplReadError(_) => 11,
            Error::TokenError(_) => 12,
        }
    }
}

impl From<SplReadError> for Error {
    fn from(other: SplReadError) -> Self {
        Self::SplReadError(other)
    }
}

impl From<TokenError> for Error {
    fn from(other: TokenError) -> Self {
        Self::TokenError(other)
    }
}

impl From<Error> for ProgramError {
    fn from(e: Error) -> Self {
        Self::Custom(e.code())
    }
}
