use crate::oid;
use super::asn1::ObjectIdentifier;

// The const raw bytes for an OID.
pub const OidSignatureMD2WithRSA: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 2);
pub const OidSignatureMD5WithRSA: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 4);
pub const OidSignatureSHA1WithRSA: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 5);
pub const OidSignatureSHA256WithRSA: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 11);
pub const OidSignatureSHA384WithRSA: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 12);
pub const OidSignatureSHA512WithRSA: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 13);
pub const OidSignatureRSAPSS: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 10);
pub const OidSignatureDSAWithSHA1: ObjectIdentifier = oid!(1, 2, 840, 10040, 4, 3);
pub const OidSignatureDSAWithSHA256: ObjectIdentifier = oid!(2, 16, 840, 1, 101, 3, 4, 3, 2);
pub const OidSignatureECDSAWithSHA1: ObjectIdentifier = oid!(1, 2, 840, 10045, 4, 1);
pub const OidSignatureECDSAWithSHA256: ObjectIdentifier = oid!(1, 2, 840, 10045, 4, 3, 2);
pub const OidSignatureECDSAWithSHA384: ObjectIdentifier = oid!(1, 2, 840, 10045, 4, 3, 3);
pub const OidSignatureECDSAWithSHA512: ObjectIdentifier = oid!(1, 2, 840, 10045, 4, 3, 4);
pub const OidSignatureEd25519: ObjectIdentifier = oid!(1, 3, 101, 112);
pub const OidSHA256: ObjectIdentifier = oid!(2, 16, 840, 1, 101, 3, 4, 2, 1);
pub const OidSHA384: ObjectIdentifier = oid!(2, 16, 840, 1, 101, 3, 4, 2, 2);
pub const OidSHA512: ObjectIdentifier = oid!(2, 16, 840, 1, 101, 3, 4, 2, 3);
pub const OidMGF1: ObjectIdentifier = oid!(1, 2, 840, 113549, 1, 1, 8);

// oidISOSignatureSHA1WithRSA means the same as oidSignatureSHA1WithRSA
// but it's specified by ISO. Microsoft's makecert.exe has been known
// to produce certificates with this OID.
pub const OidISOSignatureSHA1WithRSA:ObjectIdentifier = oid!(1, 3, 14, 3, 2, 29);
pub const OidSignatureSM2WithSM3: ObjectIdentifier = oid!(1, 2, 840, 10045, 2,1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oid() {
        assert_eq!("1.2.840.113549.1.1.2", OidSignatureMD2WithRSA.to_string());
        assert_eq!(ObjectIdentifier::try_from("1.2.840.113549.1.1.2").unwrap(), OidSignatureMD2WithRSA);
    }
}
