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
