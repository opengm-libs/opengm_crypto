use crate::sm4::BLOCK_SIZE;

use super::byteorder::*;

#[inline(always)]
pub fn load_block(src: &[u8]) -> (u32, u32, u32, u32) {
    (get_u32_be(&src[0..4]), get_u32_be(&src[4..8]), get_u32_be(&src[8..12]), get_u32_be(&src[12..16]))
}

#[inline(always)]
pub fn store_block(dst: &mut [u8], a: u32, b: u32, c: u32, d: u32) {
    put_u32_be(&mut dst[..4], d as u32);
    put_u32_be(&mut dst[4..8], c as u32);
    put_u32_be(&mut dst[8..12], b as u32);
    put_u32_be(&mut dst[12..16], a as u32);
}

#[inline(always)]
pub fn load_block2(src: &[u8]) -> (u64, u64, u64, u64) {
    (
        (get_u32_be(&src[0..4]) as u64) ^ ((get_u32_be(&src[16..20]) as u64) << 32),
        (get_u32_be(&src[4..8]) as u64) ^ ((get_u32_be(&src[20..24]) as u64) << 32),
        (get_u32_be(&src[8..12]) as u64) ^ ((get_u32_be(&src[24..28]) as u64) << 32),
        (get_u32_be(&src[12..16]) as u64) ^ ((get_u32_be(&src[28..32]) as u64) << 32),
    )
}

#[inline(always)]
pub fn store_block2(dst: &mut [u8], a: u64, b: u64, c: u64, d: u64) {
    put_u32_be(&mut dst[..4], d as u32);
    put_u32_be(&mut dst[4..8], c as u32);
    put_u32_be(&mut dst[8..12], b as u32);
    put_u32_be(&mut dst[12..16], a as u32);
    put_u32_be(&mut dst[16..20], (d >> 32) as u32);
    put_u32_be(&mut dst[20..24], (c >> 32) as u32);
    put_u32_be(&mut dst[24..28], (b >> 32) as u32);
    put_u32_be(&mut dst[28..32], (a >> 32) as u32);
}

pub mod x32 {
    use super::super::tables::*;
    #[inline]
    pub fn tau(x: u32) -> u32 {
        (SBOX[(x & 0xff) as usize] as u32) | (SBOX[((x >> 8) & 0xff) as usize] as u32) << 8 | (SBOX[((x >> 16) & 0xff) as usize] as u32) << 16 | (SBOX[((x >> 24) & 0xff) as usize] as u32) << 24
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn L(x: u32) -> u32 {
        x ^ x.rotate_left(2) ^ x.rotate_left(10) ^ x.rotate_left(18) ^ x.rotate_left(24)
    }

    #[inline]
    pub fn lt_slow(x: u32) -> u32 {
        L(tau(x))
    }

    // lt return L(Tau(x))
    #[inline]
    pub fn lt_fast(x: u32) -> u32 {
        LTAU_TABLE16[(x >> 16) as usize].rotate_left(16) ^ LTAU_TABLE16[(x & 0xffff) as usize]
    }
}

pub mod x64 {
    use super::super::tables::*;
    use super::x32;
    #[inline]
    pub fn tau(x: u64) -> u64 {
        (SBOX[(x & 0xff) as usize] as u64)
            | (SBOX[((x >> 8) & 0xff) as usize] as u64) << 8
            | (SBOX[((x >> 16) & 0xff) as usize] as u64) << 16
            | (SBOX[((x >> 24) & 0xff) as usize] as u64) << 24
            | (SBOX[((x >> 32) & 0xff) as usize] as u64) << 32
            | (SBOX[((x >> 40) & 0xff) as usize] as u64) << 40
            | (SBOX[((x >> 48) & 0xff) as usize] as u64) << 48
            | (SBOX[((x >> 56) & 0xff) as usize] as u64) << 56
    }

    #[inline]
    #[allow(non_snake_case)]
    pub fn L(x: u64) -> u64 {
        let l = x as u32;
        let h = (x >> 32) as u32;

        let l = x32::L(l) as u64;
        let h = x32::L(h) as u64;
        l ^ (h << 32)
    }

    #[inline]
    pub fn lt_slow(x: u64) -> u64 {
        L(tau(x))
    }

    // lt return L(Tau(x))
    #[inline]
    pub fn lt_fast(x: u64) -> u64 {
        let l = x as u32;
        let h = (x >> 32) as u32;

        let l = x32::lt_fast(l) as u64;
        let h = x32::lt_fast(h) as u64;
        l ^ (h << 32)
    }
}

#[inline(always)]
fn expand_roundkey(rk: u32) -> u64 {
    let rk = rk as u64;
    (rk << 32) | rk
}

#[inline(always)]
fn block_generic_option(in_out: &mut [u8], input: Option<&[u8]>, rk: &[u32]) {
    let input = match input {
        Some(input) => input,
        None => in_out,
    };

    let (mut a, mut b, mut c, mut d) = load_block(input);

    a ^= x32::lt_slow(b ^ c ^ d ^ rk[0]);
    b ^= x32::lt_slow(c ^ d ^ a ^ rk[1]);
    c ^= x32::lt_slow(d ^ a ^ b ^ rk[2]);
    d ^= x32::lt_slow(a ^ b ^ c ^ rk[3]);

    for i in 1..7 {
        a ^= x32::lt_fast(b ^ c ^ d ^ rk[4 * i + 0]);
        b ^= x32::lt_fast(c ^ d ^ a ^ rk[4 * i + 1]);
        c ^= x32::lt_fast(d ^ a ^ b ^ rk[4 * i + 2]);
        d ^= x32::lt_fast(a ^ b ^ c ^ rk[4 * i + 3]);
    }

    a ^= x32::lt_slow(b ^ c ^ d ^ rk[28]);
    b ^= x32::lt_slow(c ^ d ^ a ^ rk[29]);
    c ^= x32::lt_slow(d ^ a ^ b ^ rk[30]);
    d ^= x32::lt_slow(a ^ b ^ c ^ rk[31]);

    store_block(in_out, a, b, c, d);
}

fn block2_generic_option(in_out: &mut [u8], input: Option<&[u8]>, rk: &[u32]) {
    let input = match input {
        Some(input) => input,
        None => in_out,
    };

    let (mut a, mut b, mut c, mut d) = load_block2(input);

    a ^= x64::lt_slow(b ^ c ^ d ^ expand_roundkey(rk[0]));
    b ^= x64::lt_slow(c ^ d ^ a ^ expand_roundkey(rk[1]));
    c ^= x64::lt_slow(d ^ a ^ b ^ expand_roundkey(rk[2]));
    d ^= x64::lt_slow(a ^ b ^ c ^ expand_roundkey(rk[3]));

    for i in 1..7 {
        a ^= x64::lt_fast(b ^ c ^ d ^ expand_roundkey(rk[4 * i + 0]));
        b ^= x64::lt_fast(c ^ d ^ a ^ expand_roundkey(rk[4 * i + 1]));
        c ^= x64::lt_fast(d ^ a ^ b ^ expand_roundkey(rk[4 * i + 2]));
        d ^= x64::lt_fast(a ^ b ^ c ^ expand_roundkey(rk[4 * i + 3]));
    }

    a ^= x64::lt_slow(b ^ c ^ d ^ expand_roundkey(rk[28]));
    b ^= x64::lt_slow(c ^ d ^ a ^ expand_roundkey(rk[29]));
    c ^= x64::lt_slow(d ^ a ^ b ^ expand_roundkey(rk[30]));
    d ^= x64::lt_slow(a ^ b ^ c ^ expand_roundkey(rk[31]));

    store_block2(in_out, a, b, c, d);
}

// interleaved encrypt two blocks.
fn block2x2_generic_option(in_out: &mut [u8], input: Option<&[u8]>, rk: &[u32]) {
    let input = match input {
        Some(input) => input,
        None => in_out,
    };

    let (mut a, mut b, mut c, mut d) = load_block2(input);
    let (mut aa, mut bb, mut cc, mut dd) = load_block2(&input[32..]);

    a ^= x64::lt_slow(b ^ c ^ d ^ expand_roundkey(rk[0]));
    aa ^= x64::lt_slow(bb ^ cc ^ dd ^ expand_roundkey(rk[0]));
    b ^= x64::lt_slow(c ^ d ^ a ^ expand_roundkey(rk[1]));
    bb ^= x64::lt_slow(cc ^ dd ^ aa ^ expand_roundkey(rk[1]));
    c ^= x64::lt_slow(d ^ a ^ b ^ expand_roundkey(rk[2]));
    cc ^= x64::lt_slow(dd ^ aa ^ bb ^ expand_roundkey(rk[2]));
    d ^= x64::lt_slow(a ^ b ^ c ^ expand_roundkey(rk[3]));
    dd ^= x64::lt_slow(aa ^ bb ^ cc ^ expand_roundkey(rk[3]));

    for i in 1..7 {
        a ^= x64::lt_fast(b ^ c ^ d ^ expand_roundkey(rk[4 * i + 0]));
        aa ^= x64::lt_fast(bb ^ cc ^ dd ^ expand_roundkey(rk[4 * i + 0]));
        b ^= x64::lt_fast(c ^ d ^ a ^ expand_roundkey(rk[4 * i + 1]));
        bb ^= x64::lt_fast(cc ^ dd ^ aa ^ expand_roundkey(rk[4 * i + 1]));
        c ^= x64::lt_fast(d ^ a ^ b ^ expand_roundkey(rk[4 * i + 2]));
        cc ^= x64::lt_fast(dd ^ aa ^ bb ^ expand_roundkey(rk[4 * i + 2]));
        d ^= x64::lt_fast(a ^ b ^ c ^ expand_roundkey(rk[4 * i + 3]));
        dd ^= x64::lt_fast(aa ^ bb ^ cc ^ expand_roundkey(rk[4 * i + 3]));
    }

    a ^= x64::lt_slow(b ^ c ^ d ^ expand_roundkey(rk[28]));
    aa ^= x64::lt_slow(bb ^ cc ^ dd ^ expand_roundkey(rk[28]));
    b ^= x64::lt_slow(c ^ d ^ a ^ expand_roundkey(rk[29]));
    bb ^= x64::lt_slow(cc ^ dd ^ aa ^ expand_roundkey(rk[29]));
    c ^= x64::lt_slow(d ^ a ^ b ^ expand_roundkey(rk[30]));
    cc ^= x64::lt_slow(dd ^ aa ^ bb ^ expand_roundkey(rk[30]));
    d ^= x64::lt_slow(a ^ b ^ c ^ expand_roundkey(rk[31]));
    dd ^= x64::lt_slow(aa ^ bb ^ cc ^ expand_roundkey(rk[31]));

    store_block2(in_out, a, b, c, d);
    store_block2(&mut in_out[32..], aa, bb, cc, dd);
}

/*
Encrypt one block
*/
#[inline]
pub fn block_generic(output: &mut [u8], input: &[u8], rk: &[u32]) {
    // block!(rk, input, output);
    block_generic_option(output, Some(input), rk);
}

#[inline]
pub fn block_generic_inplace(in_out: &mut [u8], rk: &[u32]) {
    block_generic_option(in_out, None, rk);
}

#[inline]
pub fn block2_generic(output: &mut [u8], input: &[u8], rk: &[u32]) {
    block2_generic_option(output, Some(input), rk);
}

#[inline]
pub fn block2_generic_inplace(in_out: &mut [u8], rk: &[u32]) {
    block2_generic_option(in_out, None, rk);
}

// will faster then 2 block2_generic
#[inline]
pub fn block4_generic(output: &mut [u8], input: &[u8], rk: &[u32]) {
    block2x2_generic_option(output, Some(input), rk);
}

#[inline]
pub fn block4_generic_inplace(in_out: &mut [u8], rk: &[u32]) {
    block2x2_generic_option(in_out, None, rk);
}

#[inline]
pub fn block8_generic(output: &mut [u8], input: &[u8], rk: &[u32]) {
    block4_generic(&mut output[..4*BLOCK_SIZE], &input[..4*BLOCK_SIZE], rk);
    block4_generic(&mut output[4*BLOCK_SIZE..8*BLOCK_SIZE], &input[4*BLOCK_SIZE..8*BLOCK_SIZE], rk);
}

#[inline]
pub fn block8_generic_inplace(in_out: &mut [u8], rk: &[u32]) {
    block4_generic_inplace(&mut in_out[..4*BLOCK_SIZE], rk);
    block4_generic_inplace(&mut in_out[4*BLOCK_SIZE..8*BLOCK_SIZE], rk);
}


#[inline]
pub fn block16_generic(output: &mut [u8], input: &[u8], rk: &[u32]) {
    block8_generic(&mut output[..8*BLOCK_SIZE], &input[..8*BLOCK_SIZE], rk);
    block8_generic(&mut output[8*BLOCK_SIZE..16*BLOCK_SIZE], &input[8*BLOCK_SIZE..16*BLOCK_SIZE], rk);
}

#[inline]
pub fn block16_generic_inplace(in_out: &mut [u8], rk: &[u32]) {
    block8_generic_inplace(&mut in_out[..8*BLOCK_SIZE], rk);
    block8_generic_inplace(&mut in_out[8*BLOCK_SIZE..16*BLOCK_SIZE],  rk);
}

#[inline]
pub fn blocks_generic(dst: &mut [u8], src: &[u8], rk: &[u32]) -> usize {
    let block_size = BLOCK_SIZE;
    let n_blocks = src.len() / BLOCK_SIZE;
    let mut n = n_blocks;
    let mut dst = dst;
    let mut src = src;
    while n >= 4 {
        block4_generic(dst, src, rk);
        dst = &mut dst[block_size * 4..];
        src = &src[block_size * 4..];
        n -= 4;
    }
    if n >= 2 {
        block2_generic(dst, src, rk);
        dst = &mut dst[block_size * 2..];
        src = &src[block_size * 2..];
        n -= 2;
    }

    while n >= 1 {
        block_generic(dst, src, rk);
        dst = &mut dst[block_size..];
        src = &src[block_size..];
        n -= 1;
    }
    n_blocks * block_size
}

#[inline]
pub fn blocks_generic_inplace(in_out: &mut [u8], rk: &[u32]) -> usize {
    let block_size = BLOCK_SIZE;
    let n_blocks = in_out.len() / BLOCK_SIZE;
    let mut n = n_blocks;
    let mut in_out = in_out;
    while n >= 4 {
        block4_generic_inplace(in_out,  rk);
        in_out = &mut in_out[block_size * 4..];
        n -= 4;
    }
    if n >= 2 {
        block2_generic_inplace(in_out,  rk);
        in_out = &mut in_out[block_size * 2..];
        n -= 2;
    }

    while n >= 1 {
        block_generic_inplace(in_out,  rk);
        in_out = &mut in_out[block_size..];
        n -= 1;
    }
    n_blocks * block_size
}

#[cfg(test)]
mod tests {
    use crate::sm4::tests::get_tests_data;

    use super::*;

    #[test]
    fn test_block() {
        let (plain, wanted_cipher, rk) = get_tests_data(256);
        let mut cipher = [0u8; 256];

        block_generic(&mut cipher, &plain, &rk);
        assert_eq!(&wanted_cipher[..16], &cipher[..16]);

        block2_generic(&mut cipher[..16 * 2], &plain[..16 * 2], &rk);
        assert_eq!(&wanted_cipher[..16 * 2], &cipher[..16 * 2]);

        block4_generic(&mut cipher[..16 * 4], &plain[..16 * 4], &rk);
        assert_eq!(&wanted_cipher[..16 * 4], &cipher[..16 * 4]);

        block8_generic(&mut cipher[..16 * 8], &plain[..16 * 8], &rk);
        assert_eq!(&wanted_cipher[..16 * 8], &cipher[..16 * 8]);

        block16_generic(&mut cipher[..16 * 16], &plain[..16 * 16], &rk);
        assert_eq!(&wanted_cipher[..16 * 16], &cipher[..16 * 16]);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_block_generic(b: &mut Bencher) {
        // 1501ns
        let (plain, _, rk) = get_tests_data(16);
        let mut cipher = [0; 16];
        b.iter(|| {
            test::black_box(block_generic(&mut cipher, &plain, &rk));
        });
    }

    #[bench]
    fn bench_block2_generic(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            for _ in 0..8 {
                test::black_box(block2_generic(&mut cipher, &plain, &rk));
            }
        });
    }

    #[bench]
    fn bench_block4_generic(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            for _ in 0..4 {
                test::black_box(block4_generic(&mut cipher, &plain, &rk))
            }
        });
    }


    #[bench]
    fn bench_block8_generic(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            for _ in 0..2 {
                test::black_box(block8_generic(&mut cipher, &plain, &rk))
            }
        });
    }

    #[bench]
    fn bench_block16_generic(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(256);

        let mut cipher = [0; 256];
        b.iter(|| test::black_box(block16_generic(&mut cipher, &plain, &rk)));
    }

    #[bench]
    fn bench_blocks_generic(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(1024 * 1024);
        let mut cipher = vec![0; 1024 * 1024];
        // 3,173,180.17 ns/iter
        b.iter(|| {
            test::black_box(blocks_generic(cipher.as_mut(), plain.as_slice(), rk.as_slice()));
        });
    }
}
