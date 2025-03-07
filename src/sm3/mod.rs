#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod amd64;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use amd64::get_compress_fn;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
use aarch64::get_compress_fn;


mod generic;
pub(crate) mod util;

type CompressFn = for<'a> fn(iv: &mut [u32; 8], p: &'a [u8]) -> &'a [u8];

fn new_compress() -> CompressFn {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64"))]
    return get_compress_fn();
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    return compress_generic;
}

#[cfg(feature = "std")]
#[ctor::ctor]
static GLOBAL_COMPRESS:CompressFn = {new_compress()};

use core::fmt;

pub const BLOCK_SIZE: usize = 64;
pub const DIGEST_SIZE: usize = 32;

pub fn new()-> Digest{
    Digest::new()
}

#[derive(Debug, Copy, Clone)]
pub struct Digest {
    pub s: [u32; 8],
    x: [u8; BLOCK_SIZE],
    nx: usize,
    // len should be u64, but we are on 32 bits platform, so we make one.
    len: u64,
    #[cfg(not(feature = "std"))]
    compress: CompressFn,
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0x{:08x}\n0x{:08x}\n0x{:08x}\n0x{:08x}\n0x{:08x}\n0x{:08x}\n0x{:08x}\n0x{:08x}\n",
            self.s[0], self.s[1], self.s[2], self.s[3], self.s[4], self.s[5], self.s[6], self.s[7]
        )
    }
}

impl Default for Digest {
    fn default() -> Self {
        Self::new()
    }
}

impl Digest {
    pub fn new() -> Digest {
        Digest {
            s: [0x7380166f, 0x4914b2b9, 0x172442d7, 0xda8a0600, 0xa96f30bc, 0x163138aa, 0xe38dee4d, 0xb0fb0e4e],
            x: [0; BLOCK_SIZE],
            nx: 0,
            len: 0,
            #[cfg(not(feature = "std"))]
            compress: new_compress(),
        }
    }

    pub fn new_with_default_id() -> Digest {
        let mut d = Digest{
            s: [
                0xadadedb5, 0x0446043f,
                0x08a87ace, 0xe86d2243,
                0x8e232383, 0xbfc81fe2,
                0xcf9117c8, 0x4707011d],
            x: [0;BLOCK_SIZE],
            nx:  18,
            len: 146,
            #[cfg(not(feature = "std"))]
            compress: new_compress(),
        };
        d.x[..18].copy_from_slice(&[0x21, 0x53, 0xd0, 0xa9,0x87, 0x7c, 0xc6, 0x2a,0x47, 0x40, 0x02, 0xdf,0x32, 0xe5, 0x21, 0x39,0xf0, 0xa0]);
        d
    }

    pub fn reset(&mut self) -> &Digest {
        self.s = [0x7380166f, 0x4914b2b9, 0x172442d7, 0xda8a0600, 0xa96f30bc, 0x163138aa, 0xe38dee4d, 0xb0fb0e4e];
        self.x = [0; BLOCK_SIZE];
        self.nx = 0;
        self.len = 0;
        self
    }

    pub fn write(&mut self, p: &[u8]) -> &Digest {
        #[cfg(not(feature = "std"))]
        let compress = self.compress;
        #[cfg(feature = "std")]
        let compress = &GLOBAL_COMPRESS;

        let mut p = p;
        let n = p.len();
        self.len += n as u64;

        if self.nx > 0 {
            let copy_len = p.len().min(self.x.len() - self.nx);
            self.x[self.nx..self.nx + copy_len].copy_from_slice(&p[..copy_len]);
            self.nx += copy_len;

            if self.nx == BLOCK_SIZE {
                compress(&mut self.s, self.x.as_slice());
                self.nx = 0;
            }
            p = &p[copy_len..];
        }
        if p.len() >= BLOCK_SIZE{
            p = compress(&mut self.s, p);
        }
        if p.len() > 0{
            self.x[..p.len()].copy_from_slice(p);
            self.nx = p.len();
        }
        self
    }


    // sum not change the internal states.
    pub fn sum_into(&self, output: &mut [u8]) {
        #[cfg(not(feature = "std"))]
        let compress = self.compress;
        #[cfg(feature = "std")]
        let compress = &GLOBAL_COMPRESS;

        let len = self.len * 8;
        let mut buf: [u8; BLOCK_SIZE * 2] = [0; BLOCK_SIZE * 2];
        buf[..self.nx].copy_from_slice(&self.x[..self.nx as usize]);

        let mut n = self.nx as usize;
        buf[n] = 0x80u8;
        n += 1;
        let mut nn = BLOCK_SIZE;
        if n > BLOCK_SIZE - 8 {
            nn += BLOCK_SIZE;
        }
        buf[nn - 8..nn].copy_from_slice(&len.to_be_bytes());

        // copy internal state to d.
        let mut d: [u32; 8] = self.s;
        compress(&mut d, &buf[..nn]);

        for i in 0..8 {
            let di = d[i].to_be_bytes();
            output[4 * i] = di[0];
            output[4 * i + 1] = di[1];
            output[4 * i + 2] = di[2];
            output[4 * i + 3] = di[3];
        }
    }

    // sum not change the internal states.
    pub fn sum(&self) -> [u8; 32] {
        let mut result: [u8; 32] = [0; 32];
        self.sum_into(&mut result);
        result
    }
}

// (l, h) = h<<32 + l
#[derive(Debug, Clone, Copy)]
struct Uint64(u32, u32);

impl Uint64 {
    fn add(&mut self, n: u32) {
        self.0 += n;
        if self.0 < n {
            self.1 += 1;
        }
    }

    // left shift 3 bits, i.e., *8.
    fn lsh3(&self) -> Self {
        Self(self.0 << 3, (self.1 << 3) | (self.0 >> 29))
    }

    // marshal to bytes in be.
    fn marshal(&self, b: &mut [u8]) {
        b[0] = (self.1 >> 24) as u8;
        b[1] = (self.1 >> 16) as u8;
        b[2] = (self.1 >> 8) as u8;
        b[3] = self.1 as u8;
        b[4] = (self.0 >> 24) as u8;
        b[5] = (self.0 >> 16) as u8;
        b[6] = (self.0 >> 8) as u8;
        b[7] = self.0 as u8;
    }
}


#[macro_export]
macro_rules! sm3 {
    ($($x:expr),+ $(,)?) => {{
        let mut h = $crate::sm3::Digest::new();
        $(
            h.write($x);
        )* 
        h.sum()
    }};
}


#[cfg(test)]
mod test_data;

#[cfg(test)]
mod tests {
    use super::test_data::*;
    use super::*;
    use hex::FromHex;
    use hex_literal;
    use std::vec::Vec;

    #[test]
    fn test_sm3_update() {
        let mut dig = Digest::new();
        let p: [u8; 64] = [1; 64];

        dig.write(&p[..32]);
        dig.write(&p[32..]);

        let expect: [u32; 8] = [0xb9122804, 0xc515b3c2, 0xb34a42f1, 0x06edad4e, 0x52ecd5c7, 0x8545dd67, 0xf42b4275, 0x900ed3ad];
        for i in 0..8 {
            assert_eq!(dig.s[i], expect[i]);
        }
    }

    #[test]
    fn test_sm3_small() {
        let digest = sm3!("abc".as_bytes());

        let expect: [u8; 32] = [
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10, 0xe4, 0xe2, 0x41, 0x67, 0xc4, 0x87, 0x5c, 0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b, 0x8f, 0x4b, 0xa8,
            0xe0,
        ];
        for i in 0..32 {
            assert_eq!(digest[i], expect[i]);
        }
    }
    #[test]
    fn test_sm3_long() {
        let digest = sm3!(LONG_MSG);
        let expect: [u8; 32] = hex_literal::hex!("c5c13a8f59a97cdeae64f16a2272a9e7dd228cb67912cce1fbe3616c954bcbf3");
        assert_eq!(digest, expect);
    }

    #[test]
    fn test_sm3_macro() {
        let digest = sm3!("a".as_bytes(), "b".as_bytes(), "c".as_bytes());

        let expect: [u8; 32] = [
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10, 0xe4, 0xe2, 0x41, 0x67, 0xc4, 0x87, 0x5c, 0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b, 0x8f, 0x4b, 0xa8,
            0xe0,
        ];
        for i in 0..32 {
            assert_eq!(digest[i], expect[i]);
        }
    }

    #[test]
    fn test_sm3() {
        let mut msg = Vec::with_capacity(TEST_VEC.len());
        for i in 0..TEST_VEC.len() {
            msg.push(i as u8);
        }

        for (i, s) in TEST_VEC.iter().enumerate() {
            let expect = <[u8; 32]>::from_hex(*s).unwrap();
            let digest = sm3!(&msg[..i]);
            assert_eq!(digest, expect);
        }
    }

    // cargo test --release --package opengm_crypto --lib -- sm3::tests::test_bench --exact --show-output 
    #[test]
    fn test_bench() {
        extern crate std;
        use std::time::*;
        const TOTAL_BYTES: usize = 10 * 1024 * 1024;
        const COUNT: usize = 100;
        let msg = vec![0u8; TOTAL_BYTES];
        let msg = msg.into_boxed_slice();

        let start = Instant::now();
        for _ in 0..COUNT {
            sm3!(msg.as_ref());
        }
        let d = (Instant::now() - start).as_micros() as f64 / 1000000.0;
        println!("{:.2} MB/s", TOTAL_BYTES as f64 * COUNT as f64 / 1024.0 / 1024.0 / d);
    }
}
