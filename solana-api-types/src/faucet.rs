use thiserror::Error;

#[derive(Error, Debug)]
pub enum FaucetError {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialize(#[from] bincode::Error),

    #[error("transaction_length from faucet exceeds limit: {0}")]
    TransactionDataTooLarge(usize),

    #[error("transaction_length from faucet: 0")]
    NoDataReceived,

    #[error("request too large; req: ◎{0}, cap: ◎{1}")]
    PerRequestCapExceeded(f64, f64),

    #[error("limit reached; req: ◎{0}, to: {1}, current: ◎{2}, cap: ◎{3}")]
    PerTimeCapExceeded(f64, String, f64, f64),
}
