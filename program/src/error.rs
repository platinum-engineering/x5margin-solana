use solar::spl::SplReadError;

pub enum Error {
    InvalidData,
    InvalidAlignment,
    SplReadError(SplReadError),
}

impl From<SplReadError> for Error {
    fn from(other: SplReadError) -> Self {
        Self::SplReadError(other)
    }
}
