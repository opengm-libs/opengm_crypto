use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("cryptobyte: length overflow")]
    LengthOverflow,

    #[error("Builder is exceeding its fixed-size buffer")]
    FixedSizeBufferOverflow,

    #[error("ASN.1 tag number {} not supported", .0)]
    ASN1HighTag(u8),

    #[error("pending ASN.1 child too long")]
    ASN1PendingChildTooLong,

    #[error("invalid OID")]
    ASN1InvalidOid,

    #[error("invalid OID encoding")]
    ASN1InvalidOidEncoding,

    #[error("invalid BIT STRING length")]
    ASN1InvalidBitStringLength,

    #[error("invalid BIT STRING padding")]
    ASN1InvalidBitStringPadding,

    #[error("unknown error")]
    Unknown,
}

impl Default for Error {
    fn default() -> Self {
        return Error::Unknown;
    }
}

pub type Result<T> = core::result::Result<T, Error>;