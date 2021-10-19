use solana_api_types::program::ProgramError;
use solar::{entity::EntityError, error::SolarError, log::Loggable, spl::TokenError};

#[cfg_attr(feature = "debug", derive(Debug))]
#[derive(IntoStaticStr)]
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
    EntityError(EntityError),
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

impl From<EntityError> for Error {
    fn from(other: EntityError) -> Self {
        Self::EntityError(other)
    }
}

impl Loggable for Error {
    fn push_to_logger<const S: usize>(&self, logger: &mut solar::log::Logger<S>) {
        match self {
            Self::TokenError(error) => {
                logger.push_str("TokenError: ");
                error.push_to_logger(logger);
            }
            Self::EntityError(error) => {
                logger.push_str("EntityError: ");
                error.push_to_logger(logger);
            }
            Self::SolarError(error) => {
                logger.push_str("SolarError: ");
                error.push_to_logger(logger);
            }
            other => logger.push_str(other.into()),
        }
    }
}
