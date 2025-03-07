pub mod block;
pub const BLOCK_SIZE: usize = 16;
pub const KEY_SIZE: usize = 16;

use block::byteorder::*;
use block::*;

#[derive(Clone, Copy)]
pub(crate) struct Blocks {
    block16: fn(dst: &mut [u8], src: &[u8], rk: &[u32]),
    block8: fn(dst: &mut [u8], src: &[u8], rk: &[u32]),
    block4: fn(dst: &mut [u8], src: &[u8], rk: &[u32]),
    block2: fn(dst: &mut [u8], src: &[u8], rk: &[u32]),
    block: fn(dst: &mut [u8], src: &[u8], rk: &[u32]),

    block16_inplace: fn(inout: &mut [u8], rk: &[u32]),
    block8_inplace: fn(inout: &mut [u8], rk: &[u32]),
    block4_inplace: fn(inout: &mut [u8], rk: &[u32]),
    block2_inplace: fn(inout: &mut [u8], rk: &[u32]),
    block_inplace: fn(inout: &mut [u8], rk: &[u32]),
}


#[cfg(feature = "std")]
#[ctor::ctor]
static GLOBAL_BLOCKS: Blocks = {Blocks::new()};


impl Default for Blocks {
    fn default() -> Self {
        Self {
            block16: block16_generic,
            block8: block8_generic,
            block4: block4_generic,
            block2: block2_generic,
            block: block_generic,

            block16_inplace: block16_generic_inplace,
            block8_inplace: block8_generic_inplace,
            block4_inplace: block4_generic_inplace,
            block2_inplace: block2_generic_inplace,
            block_inplace: block_generic_inplace,
        }
    }
}



impl Blocks {
    #[inline(always)]
    fn new() -> Blocks {

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        return new_blocks_amd64();

        #[cfg(target_arch = "aarch64")]
        return new_blocks_aarch64();

        #[cfg(not(any(
            target_arch = "x86",
            target_arch = "x86_64",
            target_arch = "aarch64"
        )))]
        Blocks::default()
    }

    // blocks_aarch64 encrypts as much as possible. returns the
    // bytes that were encrypted.
    // dst must have a length no less then src.
    #[inline(always)]
    pub fn blocks(self, dst: &mut [u8], src: &[u8], rk: &[u32]) -> usize {
        let block_size = crate::sm4::BLOCK_SIZE;
        let n_blocks = src.len() / block_size;
        let mut n = n_blocks;
        let mut dst = dst;
        let mut src = src;

        while n >= 16 {
            (self.block16)(dst, src, rk);
            dst = &mut dst[block_size * 16..];
            src = &src[block_size * 16..];
            n -= 16;
        }

        if n >= 8 {
            (self.block8)(dst, src, rk);
            dst = &mut dst[block_size * 8..];
            src = &src[block_size * 8..];
            n -= 8;
        }

        if n >= 4 {
            (self.block4)(dst, src, rk);
            dst = &mut dst[block_size * 4..];
            src = &src[block_size * 4..];
            n -= 4;
        }

        if n >= 2 {
            (self.block2)(dst, src, rk);
            dst = &mut dst[block_size * 2..];
            src = &src[block_size * 2..];
            n -= 2;
        }

        if n >= 1 {
            (self.block)(dst, src, rk);
        }
        n_blocks * block_size
    }

    #[inline(always)]
    pub fn blocks_inplace(self, dst_src: &mut [u8], rk: &[u32]) -> usize {
        let block_size = crate::sm4::BLOCK_SIZE;
        let n_blocks = dst_src.len() / block_size;
        let mut n = n_blocks;
        let mut dst_src = dst_src;

        while n >= 16 {
            (self.block16_inplace)(dst_src, rk);
            dst_src = &mut dst_src[block_size * 16..];
            n -= 16;
        }
        if n >= 8 {
            (self.block8_inplace)(dst_src, rk);
            dst_src = &mut dst_src[block_size * 8..];
            n -= 8;
        }
        if n >= 4 {
            (self.block4_inplace)(dst_src, rk);
            dst_src = &mut dst_src[block_size * 4..];
            n -= 4;
        }
        if n >= 2 {
            (self.block2_inplace)(dst_src, rk);
            dst_src = &mut dst_src[block_size * 2..];
            n -= 2;
        }
        if n >= 1 {
            (self.block_inplace)(dst_src, rk);
        }

        n_blocks * block_size
    }
}

pub struct Cipher {
    rk: [u32; 32],
    rk_rev: [u32; 32],

    #[cfg(not(feature = "std"))]
    blocks: Blocks,
}

impl Cipher {
    pub fn new(key: &[u8]) -> Self {
        let (rk, rk_rev) = key_schedule(&key);
        Cipher {
            rk: rk,
            rk_rev: rk_rev,
            #[cfg(not(feature = "std"))]
            blocks: Blocks::new(),
        }
    }

    // encrypt blocks into dst. returns the bytes encrypted.
    pub fn encrypt(&self, dst: &mut [u8], src: &[u8]) -> usize {
        #[cfg(not(feature = "std"))]
        let blocks = &self.blocks;
        #[cfg(feature = "std")]
        let blocks = &GLOBAL_BLOCKS;

        blocks.blocks(dst, src, &self.rk)
    }

    // encrypt blocks into dst. returns the bytes encrypted.
    pub fn decrypt(&self, dst: &mut [u8], src: &[u8]) -> usize {
        #[cfg(not(feature = "std"))]
        let blocks = &self.blocks;
        #[cfg(feature = "std")]
        let blocks = &GLOBAL_BLOCKS;

        blocks.blocks(dst, src, &self.rk_rev)
    }

    pub fn encrypt_inplace(&self, in_out: &mut [u8]) -> usize {
        #[cfg(not(feature = "std"))]
        let blocks = &self.blocks;
        #[cfg(feature = "std")]
        let blocks = &GLOBAL_BLOCKS;

        blocks.blocks_inplace(in_out, &self.rk)
    }

    pub fn decrypt_inplace(&self, in_out: &mut [u8]) -> usize {
        #[cfg(not(feature = "std"))]
        let blocks = &self.blocks;
        #[cfg(feature = "std")]
        let blocks = &GLOBAL_BLOCKS;

        blocks.blocks_inplace(in_out, &self.rk_rev)
    }
}

impl Drop for Cipher {
    fn drop(&mut self) {
        for k in &mut self.rk {
            *k = 0;
        }
        for k in &mut self.rk_rev {
            *k = 0;
        }
    }
}

const FK: [u32; 4] = [0xa3b1bac6, 0x56aa3350, 0x677d9197, 0xb27022dc];
const CK: [u32; 32] = [
    0x00070e15, 0x1c232a31, 0x383f464d, 0x545b6269, 0x70777e85, 0x8c939aa1,
    0xa8afb6bd, 0xc4cbd2d9, 0xe0e7eef5, 0xfc030a11, 0x181f262d, 0x343b4249,
    0x50575e65, 0x6c737a81, 0x888f969d, 0xa4abb2b9, 0xc0c7ced5, 0xdce3eaf1,
    0xf8ff060d, 0x141b2229, 0x30373e45, 0x4c535a61, 0x686f767d, 0x848b9299,
    0xa0a7aeb5, 0xbcc3cad1, 0xd8dfe6ed, 0xf4fb0209, 0x10171e25, 0x2c333a41,
    0x484f565d, 0x646b7279,
];

// fn key_schedule(key: &[u8; KEY_SIZE])-> ([u32; 32], [u32; 32]) {
#[inline]
fn key_schedule(key: &[u8]) -> ([u32; 32], [u32; 32]) {
    let mut rk = [0u32; 32];
    let mut rk_rev = [0u32; 32];
    let mut a: u32 = get_u32_be(&key[0..4]) ^ FK[0];
    let mut b = get_u32_be(&key[4..8]) ^ FK[1];
    let mut c = get_u32_be(&key[8..12]) ^ FK[2];
    let mut d = get_u32_be(&key[12..16]) ^ FK[3];
    let mut i = 0;
    let mut t: u32;
    while i < 32 {
        t = x32::tau(b ^ c ^ d ^ CK[i]);
        a ^= t ^ t.rotate_left(13) ^ t.rotate_left(23);
        rk[i] = a;
        rk_rev[31 - i] = a;
        i += 1;

        t = x32::tau(a ^ c ^ d ^ CK[i]);
        b ^= t ^ t.rotate_left(13) ^ t.rotate_left(23);
        rk[i] = b;
        rk_rev[31 - i] = b;
        i += 1;

        t = x32::tau(b ^ a ^ d ^ CK[i]);
        c ^= t ^ t.rotate_left(13) ^ t.rotate_left(23);
        rk[i] = c;
        rk_rev[31 - i] = c;
        i += 1;

        t = x32::tau(b ^ c ^ a ^ CK[i]);
        d ^= t ^ t.rotate_left(13) ^ t.rotate_left(23);
        rk[i] = d;
        rk_rev[31 - i] = d;
        i += 1;
    }
    (rk, rk_rev)
}


#[cfg(test)]
mod tests {
    use crate::sm4::Blocks;
    use std::vec::Vec;

    // returns (plain, cipher, rk) of N blocks
    pub fn get_tests_data(n: usize) -> (Vec<u8>, Vec<u8>, [u32; 32]) {
        let mut plain = Vec::with_capacity(n);
        let mut cipher = Vec::with_capacity(n);
        for _ in 0..n / 16 {
            for j in 0..16 {
                plain.push(TEST_PLAIN[j]);
                cipher.push(TEST_CIPHER[j]);
            }
        }
        (plain, cipher, TEST_ROUNDKEY.clone())
    }

    pub const _KEY: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];

    const TEST_ROUNDKEY: [u32; 32] = [
        0xf12186f9, 0x41662b61, 0x5a6ab19a, 0x7ba92077, 0x367360f4, 0x776a0c61, 0xb6bb89b3,
        0x24763151, 0xa520307c, 0xb7584dbd, 0xc30753ed, 0x7ee55b57, 0x6988608c, 0x30d895b7,
        0x44ba14af, 0x104495a1, 0xd120b428, 0x73b55fa3, 0xcc874966, 0x92244439, 0xe89e641f,
        0x98ca015a, 0xc7159060, 0x99e1fd2e, 0xb79bd80c, 0x1d2115b0, 0x0e228aeb, 0xf1780c81,
        0x428d3654, 0x62293496, 0x01cf72e5, 0x9124a012,
    ];

    const TEST_PLAIN: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];

    const TEST_CIPHER: [u8; 16] = [
        0x68, 0x1e, 0xdf, 0x34, 0xd2, 0x06, 0x96, 0x5e, 0x86, 0xb3, 0xe9, 0x4f, 0x53, 0x6e, 0x42,
        0x46,
    ];

    #[test]
    fn test_blocks() {
        for n in 0..1024 {
            let (plain, wanted_cipher, rk) = get_tests_data(n as usize);
            let mut cipher = vec![0u8; n];
            let nn = Blocks::new().blocks(cipher.as_mut(), &plain, &rk);
            assert_eq!(&cipher[..nn], &wanted_cipher[..nn]);
        }
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_blocks(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(1024*1024);
        let mut cipher = vec![0; 1024*1024];
        let blocks = Blocks::new();
        
        // 3,171,920.85 ns/iter
        b.iter(|| {
            test::black_box(blocks.blocks(cipher.as_mut(), plain.as_slice(), rk.as_slice()));
        });
    }
}
