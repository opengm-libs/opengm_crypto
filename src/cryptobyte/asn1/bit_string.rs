use crate::cryptobyte::Error;
use alloc::vec::Vec;

// BitString is the structure to use when you want an ASN.1 BIT STRING type. A
// bit string is padded up to the nearest byte in memory and the number of
// valid bits is recorded. Padding bits will be zero.
#[derive(Default)]
pub struct BitString {
    pub bytes: Vec<u8>,
    pub bit_length: usize,
    pub right_aligned: Option<Vec<u8>>,
}

impl BitString {
    pub fn new(bytes: impl Into<Vec<u8>>, bit_length: usize)-> BitString{
        return BitString {
            bytes: bytes.into(),
            bit_length,
            right_aligned:None,
        }
    }
    // returns the i-th bit
    // Note the the index is count from highest bit to lowest.
    // EX:
    // 6e        5d        c0
    // 0110 1110 0101 1101 1100 0000
    pub fn at(&self, i: usize) -> Option<u8> {
        if i >= self.bit_length {
            return None;
        }

        let n = i % 8;
        let shift = 7 - (i % 8);
        Some((self.bytes[n] >> shift) & 1)
    }
    pub fn as_slice(&self) -> &[u8]{
        self.bytes.as_slice()
    }
    pub fn right_align(&mut self) -> &[u8] {
        if self.right_aligned.is_some(){
            return self.right_aligned.as_ref().unwrap().as_slice();
        }

        let shift = 8 - self.bit_length % 8;
        if shift == 8 || self.bytes.len() == 0{
            return self.bytes.as_slice();
        }

        let mut a = Vec::with_capacity(self.bytes.len());
        a.push(self.bytes[0] >> shift);
        for i in 1..self.bytes.len(){
            a.push((self.bytes[i-1] << (8-shift)) | (self.bytes[i] >> shift));
        }
        self.right_aligned = Some(a);
        
        self.right_aligned.as_ref().unwrap().as_slice()
    }

}


// parse a asn.1 encoded to BitString
impl TryFrom<&[u8]> for BitString {
    type Error = Error;

    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        if v.len() == 0 {
            return Err(Error::ASN1InvalidBitStringLength);
        }
        let pad_length = v[0] as usize;
        if pad_length > 7 || (v.len() == 1 && pad_length > 0) || (v[v.len() - 1] & ((1 << v[0]) - 1) != 0) {
            return Err(Error::ASN1InvalidBitStringPadding);
        }

        let bit_length = 8 * (v.len() - 1) - pad_length;

        Ok(BitString { bytes: v[1..].to_vec(), bit_length, right_aligned: None})
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shift() {
        let mut b = BitString{
            bytes: vec![0xff, 0xff, 0xe0],
            bit_length: 19,
            right_aligned: None,
        };
        let shifted = b.right_align();
        assert_eq!([0x07, 0xff, 0xff], shifted);
    }

}