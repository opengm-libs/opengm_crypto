use crate::traits::Hash;
use crate::sm3;

const MAX_BLOCK_SIZE: usize = 64;
pub struct HMac<H: Hash<DIGEST_SIZE>, const DIGEST_SIZE:usize>
{
    outer: H,
    inner: H,
    opad: [u8; MAX_BLOCK_SIZE], // only opad[..block_size] are available.

    // for reset.
    block_size: usize,
    processed_key:[u8;MAX_BLOCK_SIZE],
}

#[inline]
fn sum<H: Hash<DIGEST_SIZE>, const DIGEST_SIZE:usize>(f: &fn()->H, data: &[u8])-> [u8; DIGEST_SIZE]{
    let mut h = f();
    h.write(data);
    let mut d = [0u8; DIGEST_SIZE];
    h.sum_into(&mut d);
    d
}

impl<H: Hash<DIGEST_SIZE>,const DIGEST_SIZE:usize> HMac<H, DIGEST_SIZE>{
    // reset HMac for next computation, use the same key.
    pub fn reset(&mut self){
        let block_size = self.block_size;
        let mut ipad = [0u8; MAX_BLOCK_SIZE];
        
        for i in 0..block_size {
            ipad[i] = self.processed_key[i] ^ 0x36;
            self.opad[i] = self.processed_key[i] ^ 0x5c;
        }
        self.inner.reset();
        self.outer.reset();
        self.inner.write(&ipad[..block_size]);
    }

    fn new_f(key: &[u8], f: fn()->H) -> Self{
        let mut h = HMac {
            block_size: 0,
            outer: f(),
            inner: f(),
            opad: [0u8; MAX_BLOCK_SIZE],
            processed_key:[0u8; MAX_BLOCK_SIZE],
        };
        let block_size = h.inner.block_size();
        debug_assert!(block_size <= MAX_BLOCK_SIZE);
        h.block_size = block_size;

        let mut ipad = [0u8; MAX_BLOCK_SIZE];
        let processed_key = &mut h.processed_key;

        if key.len() > block_size {
            processed_key[..DIGEST_SIZE].copy_from_slice(&sum::<H, DIGEST_SIZE>(&f, key));
        } else {
            processed_key[..key.len()].copy_from_slice(key);
        }

        for i in 0..block_size {
            ipad[i] = processed_key[i] ^ 0x36;
            h.opad[i] = processed_key[i] ^ 0x5c;
        }

        h.inner.write(&ipad[..block_size]);
        h
    }

    pub fn write(&mut self, data: &[u8]) {
        self.inner.write(data);
    }

    pub fn sum(&mut self) -> [u8; DIGEST_SIZE] {
        let block_size = self.inner.block_size();
        let opad = &mut self.opad[0..block_size];

        let mut mac = self.inner.sum();
        self.outer.write(opad);
        self.outer.write(&mac);
        self.outer.sum_into(&mut mac);
        mac
    }

    pub fn sum_into(&mut self, out: &mut [u8]) {
        let mac = self.inner.sum();
        self.outer.write(&self.opad);
        self.outer.write(&mac);
        self.outer.sum_into(out);
    }
}

pub type HMacSM3 = HMac::<sm3::Digest, 32>;
impl HMacSM3{
    pub fn new(key:&[u8])-> Self {
        HMacSM3::new_f(key, sm3::new)
    }
}

pub fn hmac_sm3(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut hm = HMacSM3::new(key);
    hm.write(data);
    hm.sum()
}

#[macro_export]
macro_rules! hmac_sm3 {
    ($key:expr, $($x:expr),+ $(,)?) => {{
        let mut h = $crate::mac::hmac::HMacSM3::new($key, sm3::new);
        $(
            h.write($x);
        )* 
        h.sum()
    }};
}

#[macro_export]
macro_rules! hmac_sm3_into {
    ($mac:expr, $key:expr, $($x:expr),+ $(,)?) => {{
        let mut h = $crate::mac::hmac::HMacSM3::new($key);
        $(
            h.write($x);
        )* 
        h.sum_into($mac);
    }};
}

#[cfg(test)]
mod tests {
    use std::borrow::ToOwned;
    use crate::mac::HMacSM3;
    use hex_literal::hex;
    #[test]
    fn test_hmac() {
        let key = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f];
        let data = "Hello".to_owned();
        let data = data.as_bytes();
        let mut hm = HMacSM3::new(&key);
        hm.write(data);
        let mac = hm.sum();
        assert_eq!(mac, hex!("06d19e9ee3a3db273490fb6cf15d001fc3a9dfa9288f4dd801c60f9c8176b8ab"));
        hm.reset();
        hm.write(data);
        let mac = hm.sum();
        assert_eq!(mac, hex!("06d19e9ee3a3db273490fb6cf15d001fc3a9dfa9288f4dd801c60f9c8176b8ab"));
    }
}
