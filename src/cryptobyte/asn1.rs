mod object_identifier;
mod bit_string;
mod raw;
mod time;
pub mod integer;

pub use object_identifier::*;
pub use bit_string::*;

use super::Tag;


pub struct ASN1Object<'a>{
    pub raw: &'a [u8],
    pub tag: Tag,
    pub value: &'a [u8],
}