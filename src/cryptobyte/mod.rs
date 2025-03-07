// Module cryptobyte contains types that help with parsing and constructing 
// length-prefixed, binary messages, including ASN.1 DER, and theose structs
// used in TLCP encoding/decoding.
// 
// The cryptobyte Module is borrowed from golang.org/x/crypto/cryptobyte.


#![allow(non_upper_case_globals)]


pub mod asn1;
pub mod builder;
pub mod errors;
pub mod parser;
pub mod oid;
pub use builder::Builder;
pub use errors::{Error, Result};
pub use parser::Parser;

// use std::convert::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tag(pub u8);

impl From<Tag> for u8 {
    fn from(v: Tag) -> Self {
        v.0
    }
}

// Constructed types include:
// - simple string types(BER, NOT DER)
// - types derived simple string types(BER, NOT DER)
// - structured types
// - structured types by implicit tagging
// - types derived from anything by explicit tagging
// Bit 6 has value "1," indicating that the encoding is constructed.
const ClassConstructed: u8 = 0x20;


// Note
// [[class] number] EXPLICIT/IMPLICITE Type
// class = UNIVERSAL | APPLICATION | PRIVATE
// If the class name is absent, then the tag is context-specific.
// EX:
// attributes [0] IMPLICIT Attributes OPTIONAL,
// ClassContextSpecific | SEQUENCE = 0x80 | 0x30 = 0xb0
pub const ClassUniversal: u8 = 0<<6;
pub const ClassApplication: u8 = 1<<6;
pub const ClassContextSpecific: u8 = 2<<6;
pub const ClassPrivate: u8 = 3<<6;


// The following is a list of standard tag and class combinations.
pub const BOOLEAN: Tag = Tag(1);
pub const INTEGER: Tag = Tag(2);
pub const BIT_STRING: Tag = Tag(3);
pub const OCTET_STRING: Tag = Tag(4);
pub const NULL: Tag = Tag(5);
pub const OBJECT_IDENTIFIER: Tag = Tag(6);
pub const ENUM: Tag = Tag(10);
pub const UTF8String: Tag = Tag(12);
pub const SEQUENCE: Tag = Tag(16 | ClassConstructed); // 0x30
pub const SET: Tag = Tag(17 | ClassConstructed); // 0x31
pub const PrintableString: Tag = Tag(19);
pub const T61String: Tag = Tag(20);
pub const IA5String: Tag = Tag(22);
pub const UTCTime: Tag = Tag(23);
pub const GeneralizedTime: Tag = Tag(24);
pub const GeneralString: Tag = Tag(27);


