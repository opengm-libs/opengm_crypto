pub mod cbc;
pub mod gcm;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid input size")]
    InvalidInputSize,

    #[error("invalid nonce size, want {}, got {}", .0, .1)]
    InvalidNonceSize(usize, usize),

    #[error("GCM authentication failed while decrypting")]
    GCMAuthenticationError,
    
    #[error("GCM ciphertext's length ({}) is shorter than tag size({})", .0, .1)]
    GCMCiphertextTooSmall(usize, usize),

    #[error("output buffer's length too short, want {}, got {}", .0, .1)]
    OutputBufferTooShort(usize, usize),

    #[error("output too small, want: {}, got: {}", .0, .1)]
    OutputTooSmall(usize, usize),
}
pub type Result<T> = core::result::Result<T, Error>;

