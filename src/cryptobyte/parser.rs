use core::ops::Index;
use alloc::vec::Vec;

use self::asn1::{ASN1Object, BitString, ObjectIdentifier};
use num::bigint::BigInt;

use super::*;

pub trait AsParser<'a> {
    fn as_parser(self: Self) -> Parser<'a>;
}


#[derive(Debug, PartialEq, Eq)]
pub struct Parser<'a> {
    pub v: &'a [u8],
    // records how many bytes have parsed.
    bytes: usize,
}

impl<'a> From<&'a [u8]> for Parser<'a> {
    fn from(value: &'a [u8]) -> Self {
        Parser { 
            v: value,
            bytes: 0,
        }
    }
}

impl<'a> From<Parser<'a>> for &'a [u8] {
    fn from(value: Parser<'a>) -> Self {
        value.v
    }
}

impl<'a> AsParser<'a> for &'a [u8] {
    fn as_parser(self) -> Parser<'a> {
        Parser{ 
            v: self,
            bytes: 0,
        }
    }
}


impl<'a> Index<usize> for Parser<'a> {
    type Output = u8;

    fn index(&self, index: usize) -> &u8 { 
        &self.v[index]
    }
}

macro_rules! read_length_prefixed {
    ($name:ident, $n:expr) => {
        #[inline]
        pub fn $name(&mut self) -> Option<&'a [u8]> {
            self.read_length_prefixed($n)
        }
    };
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a [u8]) -> Self {
        Parser { 
            v: s,
            bytes: 0,
        }
    }

    fn as_slice(self: &'a Self) -> &'a [u8] {
        self.v
    }

    pub fn bytes_read(&self)-> usize{
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.v.len()
    }

    pub fn to_owned(&mut self)-> Vec<u8>{
        let mut res = Vec::with_capacity(self.len());
        for i in self.v{
            res.push(*i);
        }
        res
    }

    pub fn read(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.v.len() >= n {
            let (v1,v2 ) = self.v.split_at(n);
            self.v = v2;
            self.bytes += n;
            Some(v1)
        }else{
            None
        }
    }

    pub fn skip(&mut self, n: usize) -> bool {
        self.read(n).is_some()
    }

    pub fn read_u8(&mut self) -> Option<u8> {
        match self.read(1) {
            Some(v) => Some(v[0]),
            None => None,
        }
    }

    pub fn read_u16(&mut self) -> Option<u16> {
        match self.read(2) {
            Some(v) => Some(((v[0] as u16) << 8) | (v[1] as u16)),
            None => None,
        }
    }

    pub fn read_u24(&mut self) -> Option<u32> {
        match self.read(3) {
            Some(v) => Some(((v[0] as u32) << 16) | ((v[1] as u32) << 8) | (v[2] as u32)),
            None => None,
        }
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        match self.read(4) {
            Some(v) => Some(((v[0] as u32) << 24) | ((v[1] as u32) << 16) | ((v[2] as u32) << 8) | (v[3] as u32)),
            None => None,
        }
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        match self.read(8) {
            Some(v) => Some(
                ((v[0] as u64) << 56) | ((v[1] as u64) << 48) | ((v[2] as u64) << 40) | ((v[3] as u64) << 32) | ((v[4] as u64) << 24) | ((v[5] as u64) << 16) | ((v[6] as u64) << 8) | (v[7] as u64),
            ),
            None => None,
        }
    }

    fn read_unsigned(&mut self, length: usize) -> Option<u32> {
        let bytes = self.read(length)?;
        // let bytes = bytes.try_into();
        // let bytes = bytes.ok()?;
        if bytes.len() > 4{
            return None;
        }
        let mut res = 0u32;
        for i in bytes{
            res = (res << 8) + (*i as u32);
        }
        Some(res)
    }

    pub fn read_length_prefixed(&mut self, len_len: usize) -> Option<&'a [u8]> {
        let len_bytes = self.read(len_len)?;
        if len_bytes.is_empty() {
            return None;
        }
        let mut length = 0usize;
        for b in len_bytes {
            length <<= 8;
            length |= *b as usize;
        }
        self.read(length)
    }

    read_length_prefixed!(read_u8_length_prefixed, 1);
    read_length_prefixed!(read_u16_length_prefixed, 2);
    read_length_prefixed!(read_u24_length_prefixed, 3);
    read_length_prefixed!(read_u32_length_prefixed, 4);
    read_length_prefixed!(read_u64_length_prefixed, 8);


    pub fn read_bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        self.read(n)
    }

    // copy out.len() bytes and advance self
    // return Option for consistent.
    pub fn copy_bytes(&mut self, out: &mut [u8]) -> Option<()> {
        let n = out.len();
        out.copy_from_slice(self.read(n)?);
        Some(())
    }

    pub fn empty(&self) -> bool {
        self.v.len() == 0
    }

    // ASN.1

    pub fn peek_tag(&mut self) -> Option<Tag> {
        if self.v.len() < 2 {
            return None;
        }
        Some(Tag(self[0]))
    }

    // helper function for String, read an ASN.1 element (tag < 30).
    // the return String include Tag and Length if skip_header = true.
    pub fn read_asn1_object(&mut self) -> Option<ASN1Object> {
        if self.v.len() < 2 {
            return None;
        }
        let raw = self.v;

        let tag = self.peek_tag()?;
        let len_byte = self[1];

        let length: usize;
        let header_len: usize;
        if len_byte & 0x80 == 0 {
            // short-form encoding.
            length = len_byte as usize + 2;
            header_len = 2;
        } else {
            let len_len = (len_byte & 0x7f) as usize;
            if len_len == 0 || len_len > 4 || self.v.len() < len_len + 2 {
                return None;
            }
            let len32 = Parser {v: &self.v[2..2 + len_len], bytes: 0 }.read_unsigned(len_len)?;
            if len32 < 128 {
                // length should have used short-form encoding.
                return None;
            }
            header_len = 2 + len_len;
            if (header_len as u32).wrapping_add(len32) < len32 {
                // overflow
                return None;
            }
            length = header_len + len32 as usize;
        }

        let mut out = Parser { v: self.read_bytes(length)?, bytes: 0};
        if !out.skip(header_len) {
            panic!("internal error");
        }

        Some(ASN1Object{
            raw: &raw[..length],
            tag:tag, 
            value: out.v,
        })
    }

    pub fn read_asn1(&mut self, tag: Tag) -> Option<&[u8]> {
        let obj = self.read_asn1_object()?;
        if obj.tag != tag {
            return None;
        }
        Some(obj.value)
    }

    // // read an ASN.1 element's Value
    // pub fn read_any_asn1(&mut self) -> Option<(Tag, Parser)> {
    //     self.read_asn1_inner(true)
    // }

    // // read an ASN.1 element, include TLV.
    // pub fn read_any_asn1_element(&mut self) -> Option<(Tag, Parser)> {
    //     self.read_asn1_inner(false)
    // }

    pub fn read_asn1_sequence(&mut self) -> Option<Parser>{
        Some(Parser::new(self.read_asn1(SEQUENCE)?))
    }

    pub fn read_asn1_octet_string(&mut self) -> Option<&[u8]>{
        let bytes = self.read_asn1(OCTET_STRING)?;
        Some(bytes)
    }

    pub fn read_asn1_boolean(&mut self) -> Option<bool> {
        let bytes = self.read_asn1(BOOLEAN)?;
        if bytes.len() != 1 {
            return None;
        }
        match bytes[0] {
            0 => Some(false),
            0xff => Some(true),
            _ => None,
        }
    }

    pub fn read_asn1_i64(&mut self) -> Option<i64> {
        let bytes = self.read_asn1(INTEGER)?;
        if bytes.len() == 0 {
            return None;
        }

        // check bytes = [0xff, 0b1xxxxxxx, ...], not a valid asn.1 der encoding.
        if bytes.len() > 1 && bytes[0] == 0xff && bytes[1] & 0x80 != 0 {
            return None;
        }

        let mut result: i64 = (bytes[0] as i8) as i64;
        for b in &bytes[1..] {
            result <<= 8;
            result += (*b) as i64;
        }
        Some(result)
    }

    pub fn read_asn1_bigint(&mut self) -> Option<BigInt> {
        let bytes = self.read_asn1(INTEGER)?;
        if bytes.len() == 0 {
            return None;
        }
        // check bytes = [0xff, 0b1xxxxxxx, ...], not a valid asn.1 der encoding.
        if bytes.len() > 1 && bytes[0] == 0xff && bytes[1] & 0x80 != 0 {
            return None;
        }

        Some(BigInt::from_signed_bytes_be(bytes))
    }

    pub fn read_asn1_enum(&mut self) -> Option<i32> {
        Some(self.read_asn1_i64()? as i32)
    }

    pub fn read_asn1_object_identifier(&mut self) -> Option<ObjectIdentifier> {
        let bytes = self.read_asn1(OBJECT_IDENTIFIER)?;
        ObjectIdentifier::try_from_asn1(bytes).ok()
    }

    pub fn read_asn1_bit_string(&mut self) -> Option<BitString> {
        let bytes = self.read_asn1(BIT_STRING)?;
        BitString::try_from(bytes).ok()
    }

    pub fn read_asn1_generalized_time(&mut self) -> Option<()> {
        todo!()
    }

    pub fn read_asn1_utc_time(&mut self) -> Option<()> {
        todo!()
    }

}

#[cfg(test)]
mod tests {
    use num::*;

    use super::*;

    #[test]
    fn test_read_u64() {
        let mut s = Parser::new(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let n = s.read_u64().unwrap();
        assert_eq!(n, 0x0102030405060708);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0], 9);
    }
    #[test]
    fn test_read_length_prefixed() {
        let mut s = Parser::new(&[0, 3, 4, 5, 6, 7, 8, 9]);
        let n = s.read_length_prefixed(2).unwrap();
        println!("{:?}", n);
        println!("{:?}", s);

        let mut s = Parser::new(&[0, 0, 3, 4, 5, 6, 7, 8, 9]);
        let n = s.read_length_prefixed(3).unwrap();
        println!("{:?}", n);
        println!("{:?}", s);
    }

    #[test]
    fn test_read_asn1_boolean() {
        let mut s = Parser::new(&[1, 1, 0, 1, 1, 0xff, 5, 6, 7, 8, 9]);
        let b = s.read_asn1_boolean().unwrap();
        assert!(!b);
        let b = s.read_asn1_boolean().unwrap();
        assert!(b);
        assert_eq!(s, Parser::new(&[5, 6, 7, 8, 9]));
    }

    #[test]
    fn test_read_asn1_i64() {
        let mut b = Builder::new(Vec::new());
        for n in -100000..100000 {
            b.add_asn1_i64(n);
            let bytes = b.take().unwrap();

            let mut s = Parser::new(&bytes);
            let m = s.read_asn1_i64().unwrap();

            assert_eq!(n, m);
        }
    }
    #[test]
    #[should_panic]
    fn test_read_asn1_i64_panic() {
        let mut s = Parser::new(&[2, 2, 0xff, 0xff]);
        s.read_asn1_i64().unwrap();
    }

    #[test]
    fn test_read_asn1_bigint() {
        let mut b = Builder::new(Vec::new());
        let n = BigInt::from_str_radix("ff1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", 16).unwrap();
        b.add_asn1_bigint(&n);
        let bytes = b.take().unwrap();

        let mut s = Parser::new(&bytes);
        let m = s.read_asn1_bigint().unwrap();

        assert_eq!(n, m);
    }
}
