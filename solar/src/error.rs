use crate::log::Loggable;

/// Common error kinds when interfacing with Solar APIs.
#[derive(IntoStaticStr, Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum SolarError {
    InvalidData,
    InvalidOwner,
    InvalidMint,

    InvalidAuthority,
    NotSigned,
}

impl Loggable for SolarError {
    fn push_to_logger<const S: usize>(&self, logger: &mut crate::log::Logger<S>) {
        logger.push_str(self.into())
    }
}
