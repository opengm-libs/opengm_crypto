use core::panic;
use core::mem::{self, take};

use alloc::vec::Vec;

use super::*;
use self::asn1::{BitString, ObjectIdentifier};
use num::bigint::{Sign, BigInt};

// Builder is a rust version of golang.org/x/crypto/cryptobyte.
#[derive(Default)]
pub struct Builder {
    err: Option<Error>,
    result: Vec<u8>,
    fixed_size: bool,
    offset: i32,
    pending_len_len: i32,
    pending_is_asn1: bool,
}

impl Builder {
    pub fn new(out: Vec<u8>) -> Self {
        Builder {
            err: None,
            result: out,
            fixed_size: false,
            offset: 0,
            pending_len_len: 0,
            pending_is_asn1: false,
        }
    }

    pub fn new_fixed_builder(buffer: Vec<u8>) -> Self {
        Builder {
            err: None,
            result: buffer,
            fixed_size: true,
            // child: None,
            offset: 0,
            pending_len_len: 0,
            pending_is_asn1: false,
        }
    }

    fn set_error(&mut self, err: Error) {
        self.err = Some(err)
    }

    // take_bytes takes the result as a Vec<u8> and returns.
    // The Builder::result is set to default after return.
    pub fn take(&mut self) -> Result<Vec<u8>> {
        match &mut self.err {
            Some(err) => Err(mem::take(err)),
            None => Ok(mem::take(&mut self.result)),
        }
    }

    pub fn add_u8(&mut self, v: u8) {
        self.add_bytes(&[v]);
    }

    pub fn add_u16(&mut self, v: u16) {
        self.add_bytes(&[(v >> 8) as u8, v as u8]);
    }

    pub fn add_u32(&mut self, v: u32) {
        self.add_bytes(&[(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]);
    }

    pub fn add_u64(&mut self, v: u64) {
        self.add_bytes(&[
            (v >> 56) as u8,
            (v >> 48) as u8,
            (v >> 40) as u8,
            (v >> 32) as u8,
            (v >> 24) as u8,
            (v >> 16) as u8,
            (v >> 8) as u8,
            v as u8,
        ]);
    }

    pub fn add_bytes(&mut self, bytes: &[u8]) {
        if self.err.is_some() {
            return;
        }

        if self.result.len() + bytes.len() < bytes.len() {
            self.err = Some(Error::LengthOverflow);
            return;
        }

        if self.fixed_size && self.result.len() + bytes.len() > self.result.capacity() {
            self.err = Some(Error::FixedSizeBufferOverflow);
            return;
        }

        for b in bytes {
            self.result.push(*b);
        }
    }

    fn add_length_prefixed<F>(&mut self, len_len: usize, is_asn1: bool, mut f: F)
    where
        F: FnMut(&mut Builder),
    {
        if self.err.is_some() {
            return;
        }
        let offset = self.result.len();
        self.add_bytes(&vec![0; len_len]);

        let mut child = Builder {
            result: take(&mut self.result),
            fixed_size: self.fixed_size,
            offset: offset as i32,
            pending_len_len: len_len as i32,
            pending_is_asn1: is_asn1,
            err: None,
        };
        
        f(&mut child);
        child.flush();

        match child.take(){
            Err(e) => self.err = Some(e),
            Ok(v) => self.result = v,
        }
    }

    fn flush(&mut self) {
        let mut length = self.result.len() as i64 - self.pending_len_len as i64 - self.offset as i64;
        if length < 0 {
            panic!("cryptobyte: internal error");
        }

        if self.pending_is_asn1 {
            // for asn1, the length is encoded to un-fixed multiple bytes:
            // (0x80 | n) + n bytes of length.

            // pending_len_len is set to 1 by default when calling add_length_prefixed.
            assert_eq!(self.pending_len_len, 1);

            // length of the total bytes of length encoded.
            let len_len: i32;

            // len_byte = 0x80 | (len_len - 1)
            let len_byte: u8;
            if length > 0xfffffffe {
                self.err = Some(Error::ASN1PendingChildTooLong);
                return;
            } else if length > 0xffffff {
                len_len = 5;
                len_byte = 0x80 | 4;
            } else if length > 0xffff {
                len_len = 4;
                len_byte = 0x80 | 3;
            } else if length > 0xff {
                len_len = 3;
                len_byte = 0x80 | 2;
            } else if length > 0x7f {
                len_len = 2;
                len_byte = 0x80 | 1;
            } else {
                len_len = 1;
                len_byte = length as u8;
                length = 0;
            }

            self.result[self.offset as usize] = len_byte;
            self.offset += 1;

            let extra_bytes = len_len - 1;
            let offset = self.offset as usize;
            self.result.splice(offset..offset, vec![0; extra_bytes as usize]);

            self.pending_len_len = extra_bytes;
        }

        let mut l = length;
        let mut i = self.pending_len_len - 1;
        while i >= 0 {
            self.result[self.offset as usize + i as usize] = l as u8;
            l = l >> 8;
            i -= 1;
        }
        if l != 0 {
            self.err = Some(Error::FixedSizeBufferOverflow);
        }
        if self.fixed_size && self.result[0] != self.result[0] {
            panic!("cryptobyte: ")
        }
    }
}

macro_rules! add_length_prefixed {
    ($name:ident, $n:expr) => {
        pub fn $name<F>(&mut self, f: F)
        where
            F: FnMut(&mut Builder),
        {
            self.add_length_prefixed($n, false, f);
        }
    };
}

impl Builder {
    add_length_prefixed!(add_u8_length_prefixed, 1);
    add_length_prefixed!(add_u16_length_prefixed, 2);
    add_length_prefixed!(add_u24_length_prefixed, 3);
    add_length_prefixed!(add_u32_length_prefixed, 4);
    add_length_prefixed!(add_u64_length_prefixed, 8);
}

// ASN.1 related functions
impl Builder {
    // AddASN1 appends an ASN.1 object. The object is prefixed with the given tag.
    // Tags greater than 30 are not supported and result in an error (i.e.
    // low-tag-number form only). The child builder passed to the
    // BuilderContinuation can be used to build the content of the ASN.1 object.
    pub fn add_asn1<F>(&mut self, tag: Tag, f: F)
    where
        F: FnMut(&mut Builder),
    {
        if self.err.is_some() {
            return;
        }

        // Identifiers with the low five bits set indicate high-tag-number format
        // (two or more octets), which we don't support.
        if tag.0 & 0x1f == 0x1f {
            self.err = Some(Error::ASN1HighTag(tag.0));
            return;
        }
        self.add_u8(tag.0);
        // the len_len set default to 1, if length > 1, a shift will be applied.
        // see Builder.flush_child.
        self.add_length_prefixed(1, true, f);
    }

    pub fn add_asn1_sequence<F>(&mut self, f: F)
    where
        F: FnMut(&mut Builder),
    {
        self.add_asn1(SEQUENCE, f)
    }
    
    pub fn add_asn1_u64(&mut self, v: u64) {
        self.add_asn1(INTEGER, |b| {
            let mut length = 1;
            let mut i = v;
            while i >= 0x80 {
                length += 1;
                i >>= 8;
            }
            while length > 0 {
                let i = v >> ((length - 1) * 8) & 0xff;
                b.add_u8(i as u8);
                length -= 1;
            }
        })
    }

    #[inline]
    fn add_asn1_signed(&mut self, tag: Tag, v: i64) {
        self.add_asn1(tag, |b| {
            let mut length = 1;
            let mut i = v;
            while i >= 0x80 || i < -0x80 {
                length += 1;
                i >>= 8;
            }

            while length > 0 {
                let i = v >> ((length - 1) * 8) & 0xff;
                b.add_u8(i as u8);
                length -= 1;
            }
        })
    }

    pub fn add_asn1_i64(&mut self, v: i64) {
        self.add_asn1_signed(INTEGER, v);
    }

    // AddASN1Enum appends a DER-encoded ASN.1 ENUMERATION.
    pub fn add_asn1_enum(&mut self, v: i64) {
        self.add_asn1_signed(ENUM, v);
    }

    // AddASN1BigInt appends a DER-encoded ASN.1 INTEGER.
    pub fn add_asn1_bigint(&mut self, n: &BigInt) {
        if self.err.is_some() {
            return;
        }

        self.add_asn1(INTEGER, |c| match n.sign() {
            Sign::Minus => {
                c.add_bytes(n.to_signed_bytes_be().as_slice());
            }
            Sign::NoSign => {
                c.add_u8(0);
            }
            Sign::Plus => {
                let (_, bytes) = n.to_bytes_be();
                if bytes[0] & 0x80 != 0 {
                    c.add_u8(0)
                }
                c.add_bytes(bytes.as_slice())
            }
        })
    }

    // add_asn1_octet_string appends a DER-encoded ASN.1 OCTET STRING.
    pub fn add_asn1_octet_string(&mut self, bytes: &[u8]) {
        self.add_asn1(OCTET_STRING, |b| b.add_bytes(bytes))
    }

    pub fn add_asn1_bit_string(&mut self, v: &BitString) {
        self.add_asn1(BIT_STRING, |b| {
            // b.add_u8( (8 - v.bit_length as u8 % 8) % 8);
            b.add_u8(0);
            b.add_bytes(&v.bytes);
        })
    }

    // pub fn add_asn1_generalized_time(&mut self, t: time::SystemTime) {}

    // To encode the oid = [value1, value2, value3, ...], value1 = 0,1,2, value2 = 0..=39 if value1 is 0 or 1
    // The first octet has value 40 * value1 + value2. The following octets, if any, encode value3, ..., valuen.
    // Each value is encoded base 128, most significant digit first, with as few digits as possible,
    // and the most significant bit of each octet except the last in the value's encoding set to "1."
    // For example, for oid { 1 2 840 113549 },
    // 40 * 1 + 2 = 42 = 0x2a.
    // 840 = 0x06 * 128 + 0x48
    // 113549 = 6 * 128^2 + 0x77 * 128 + 0x0d
    // The DER encode is 06 06 2a 86 48 86 f7 0d
    pub fn add_asn1_object_identifier(&mut self, oid: &ObjectIdentifier) {
        self.add_asn1(OBJECT_IDENTIFIER, |b| {
            if !oid.is_valid() {
                b.err = Some(Error::ASN1InvalidOid);
            }
            b.add_bytes(oid.as_der());
        });
    }

    pub fn add_asn1_boolean(&mut self, v: bool) {
        match v {
            true => self.add_bytes(&[u8::from(BOOLEAN), 1, 0xff]),
            false => self.add_bytes(&[u8::from(BOOLEAN), 1, 0]),
        }
    }

    pub fn add_asn1_null(&mut self) {
        self.add_bytes(&[u8::from(NULL), 0])
    }

    fn add_base128_int(&mut self, n: i64) {
        let mut length: i32 = 0;
        if n == 0 {
            length = 1
        } else {
            let mut i = n;
            while i > 0 {
                length += 1;
                i >>= 7;
            }
        }

        let mut i = length - 1;
        while i >= 0 {
            let mut o = (n >> (i * 7 as i32)) as u8;
            o &= 0x7f;
            if i != 0 {
                o |= 0x80;
            }

            self.add_u8(o);
            i -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::i64;

    use num::bigint::ToBigInt;

    use super::*;

    #[test]
    fn test_builder() {
        let mut parent = Builder::new(Vec::new());
        parent.add_u8_length_prefixed(|child| {
            child.add_u8(1);
            child.add_u16_length_prefixed(|grandchild| {
                grandchild.add_u8(2);
                grandchild.add_u24_length_prefixed(|grandgrandchild| {
                    grandgrandchild.add_u8(3);
                    grandgrandchild.add_bytes(&[4, 5, 6]);
                })
            });
        });

        let b = parent.take().unwrap();
        assert_eq!(b, vec![11, 1, 0, 8, 2, 0, 0, 4, 3, 4, 5, 6]);
    }

    #[test]
    fn test_builder_asn1() {
        let mut parent = Builder::new(Vec::new());
        parent.add_asn1(Tag(1), |child| {
            let mut vec = Vec::new();
            for i in 1..129 {
                vec.push(i as u8);
            }
            child.add_bytes(&vec);
            child.add_asn1(Tag(2), |grandchild| {
                grandchild.add_asn1_u64(0x80);
            })
        });

        let b = parent.take().unwrap();
        println!("{:?}", b);
        // assert_eq!(b, vec![1,7,1,2,3,2,2,0,4]);
    }

    #[test]
    fn test_add_asn1_u64() {
        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_u64(1);
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 1, 1]);

        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_u64(0x8081);
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 3, 0, 0x80, 0x81]);
    }

    #[test]
    fn test_add_asn1_i64() {
        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_i64(1);
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 1, 1]);

        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_i64(-1);
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 1, 0xff]);

        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_i64(-128);
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 1, 0x80]);

        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_i64(-129);
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 2, 0xff, 0x7f]);

        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_i64(i64::MIN);
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 8, 128, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_add_asn1_bigint() {
        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_bigint(&-1125.to_bigint().unwrap());
        let b = parent.take().unwrap();
        assert_eq!(b, vec![2, 2, 251, 155]);
    }

    #[test]
    fn test_add_object_identifier() {
        let mut parent = Builder::new(Vec::new());
        parent.add_asn1_object_identifier(
            &ObjectIdentifier::from_slice(&[1, 2, 156, 10197, 1, 301]).unwrap(), //SM2ECC
        );
        let b = parent.take().unwrap();
        assert_eq!(b, vec![6, 0x08, 0x2A, 0x81, 0x1C, 0xCF, 0x55, 0x01, 0x82, 0x2D]);
    }
}
