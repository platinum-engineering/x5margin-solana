use solana_api_types::program::ProgramError;
use solar::{
    entity::EntityError,
    log::Loggable,
    spl::{SplReadError, TokenError},
};

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
    EntityError(EntityError),
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
    fn from(_: Error) -> Self {
        todo!()
    }
}

impl From<EntityError> for Error {
    fn from(other: EntityError) -> Self {
        Self::EntityError(other)
    }
}

impl Loggable for Error {
    fn push_to_logger<const S: usize>(&self, logger: &mut solar::log::Logger<S>) {
        todo!()
    }
}
