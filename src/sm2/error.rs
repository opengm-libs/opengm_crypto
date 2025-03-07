use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum SM2Error {
    #[error("invalid point")]
    InvalidPoint,

    #[error("sm2 cipher hash check failed")]
    InvalidCipherHash,
    
    #[error("unknown error")]
    Unknown,
}
pub type Result<T> = core::result::Result<T, SM2Error>;

