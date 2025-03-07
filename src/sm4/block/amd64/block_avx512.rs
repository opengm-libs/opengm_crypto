use super::super::tables::*;
// use super::mem::load_block16::*;
use super::prelude::*;
use super::super::generic::{load_block2, store_block2, x64};


const FLP32: __m512i = unsafe {
    transmute([
        0x0405060700010203u64,
        0x0C0D0E0F08090A0B,
        0x1415161710111213,
        0x1C1D1E1F18191A1B,
        0x2425262720212223,
        0x2C2D2E2F28292A2B,
        0x3435363730313233,
        0x3C3D3E3F38393A3B,
    ])
};

const MASK16: __m512i = unsafe {
    transmute([
        0x0000ffff0000ffffu64,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
    ])
};

#[inline(always)]
fn ltau(x: __m512i) -> __m512i {
    unsafe {
        let l = _mm512_and_si512(x, MASK16);
        let l = _mm512_i32gather_epi32::<4>(l, LTAU_TABLE16.as_ptr() as *const u8);

        let h = _mm512_srli_epi32(x, 16);
        let h = _mm512_i32gather_epi32::<4>(h, LTAU_TABLE16.as_ptr() as *const u8);
        let h = _mm512_ror_epi32(h, 16);

        _mm512_xor_si512(l, h)
    }
}

#[inline(always)]
fn expand_roundkey(rk: &[u32], i: usize) -> [__m512i; 4] {
    unsafe {
        [
            _mm512_set1_epi32(rk[i + 0] as i32),
            _mm512_set1_epi32(rk[i + 1] as i32),
            _mm512_set1_epi32(rk[i + 2] as i32),
            _mm512_set1_epi32(rk[i + 3] as i32),
        ]
    }
}


// load 8 blocks and perform the first round use the sbox to void cache attack.
#[inline(always)]
fn load_and_first_4round_slow(input: &[u8], rk: &[u32]) -> (__m512i, __m512i, __m512i, __m512i) {
    let mut a = [0u64; 8];
    let mut b = [0u64; 8];
    let mut c = [0u64; 8];
    let mut d = [0u64; 8];
    for i in 0..8 {
        (a[i], b[i], c[i], d[i]) = load_block2(input);
        a[i] ^= x64::lt_slow(b[i] ^ c[i] ^ d[i] ^ (((rk[0] as u64) << 32) ^ (rk[0] as u64)));
        b[i] ^= x64::lt_slow(c[i] ^ d[i] ^ a[i] ^ (((rk[1] as u64) << 32) ^ (rk[1] as u64)));
        c[i] ^= x64::lt_slow(d[i] ^ a[i] ^ b[i] ^ (((rk[2] as u64) << 32) ^ (rk[2] as u64)));
        d[i] ^= x64::lt_slow(a[i] ^ b[i] ^ c[i] ^ (((rk[3] as u64) << 32) ^ (rk[3] as u64)));
    }

    unsafe {
        (
            _mm512_set_epi64(a[0] as i64, a[1] as i64, a[2] as i64, a[3] as i64,a[4] as i64, a[5] as i64, a[6] as i64, a[7] as i64),
            _mm512_set_epi64(b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64,b[4] as i64, b[5] as i64, b[6] as i64, b[7] as i64),
            _mm512_set_epi64(c[0] as i64, c[1] as i64, c[2] as i64, c[3] as i64,c[4] as i64, c[5] as i64, c[6] as i64, c[7] as i64),
            _mm512_set_epi64(d[0] as i64, d[1] as i64, d[2] as i64, d[3] as i64,d[4] as i64, d[5] as i64, d[6] as i64, d[7] as i64),
        )
    }
}
#[inline(always)]
fn last_4round_and_store(output: &mut [u8], a: __m512i, b: __m512i, c: __m512i, d: __m512i, rk:&[u32]) {
    let mut va = [0u64; 8];
    let mut vb = [0u64; 8];
    let mut vc = [0u64; 8];
    let mut vd = [0u64; 8];
    unsafe {
        _mm512_storeu_epi64(va.as_mut_ptr() as *mut i64, a);
        _mm512_storeu_epi64(vb.as_mut_ptr() as *mut i64, b);
        _mm512_storeu_epi64(vc.as_mut_ptr() as *mut i64, c);
        _mm512_storeu_epi64(vd.as_mut_ptr() as *mut i64, d);
    }
    for i in 0..8 {
        va[i] ^= x64::lt_slow(vb[i] ^ vc[i] ^ vd[i] ^ (((rk[28] as u64) << 32) ^ (rk[28] as u64)));
        vb[i] ^= x64::lt_slow(vc[i] ^ vd[i] ^ va[i] ^ (((rk[29] as u64) << 32) ^ (rk[29] as u64)));
        vc[i] ^= x64::lt_slow(vd[i] ^ va[i] ^ vb[i] ^ (((rk[30] as u64) << 32) ^ (rk[30] as u64)));
        vd[i] ^= x64::lt_slow(va[i] ^ vb[i] ^ vc[i] ^ (((rk[31] as u64) << 32) ^ (rk[31] as u64)));
    }


    store_block2(&mut output[32 * 0..], va[0], vb[0], vc[0], vd[0]);
    store_block2(&mut output[32 * 1..], va[1], vb[1], vc[1], vd[1]);
    store_block2(&mut output[32 * 2..], va[2], vb[2], vc[2], vd[2]);
    store_block2(&mut output[32 * 3..], va[3], vb[3], vc[3], vd[3]);
    store_block2(&mut output[32 * 4..], va[4], vb[4], vc[4], vd[4]);
    store_block2(&mut output[32 * 5..], va[5], vb[5], vc[5], vd[5]);
    store_block2(&mut output[32 * 6..], va[6], vb[6], vc[6], vd[6]);
    store_block2(&mut output[32 * 7..], va[7], vb[7], vc[7], vd[7]);
}


#[inline(always)]
fn round_avx512(a: __m512i, b: __m512i, c: __m512i, d: __m512i, rk: __m512i) -> __m512i {
    unsafe {
        let t = _mm512_xor_si512(_mm512_xor_si512(_mm512_xor_si512(c, d), b), rk);
        _mm512_xor_si512(a, ltau(t))
    }
}

#[inline(always)]
fn round4_avx512(
    a: __m512i,
    b: __m512i,
    c: __m512i,
    d: __m512i,
    rk: [__m512i; 4],
) -> (__m512i, __m512i, __m512i, __m512i) {
    let a = round_avx512(a, b, c, d, rk[0]);
    let b = round_avx512(b, a, c, d, rk[1]);
    let c = round_avx512(c, d, a, b, rk[2]);
    let d = round_avx512(d, c, a, b, rk[3]);
    (a, b, c, d)
}


pub fn block16_avx512(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    unsafe { unsafe_block16_avx512(dst, src, rk) }
}

#[target_feature(enable = "avx512f",enable = "avx512bw")]
pub unsafe fn unsafe_block16_avx512(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    // let (a, b, c, d) = load_block16(src);
    // let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 0));

    let (a,b,c,d) = load_and_first_4round_slow(src, rk);
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 4));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 8));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 12));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 16));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 20));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 24));
    
    last_4round_and_store(dst, a,b,c,d,rk);
    // let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 28));
    // store_block16(dst, a, b, c, d);
}


pub fn block16_avx512_inplace(dst_src: &mut [u8],  rk: &[u32]) {
    unsafe { unsafe_block16_avx512_inplace(dst_src, rk) }
}

#[target_feature(enable = "avx512f",enable = "avx512bw")]
pub unsafe fn unsafe_block16_avx512_inplace(dst_src: &mut [u8], rk: &[u32]) {
    // let (a, b, c, d) = load_block16(src);
    // let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 0));

    let (a,b,c,d) = load_and_first_4round_slow(dst_src, rk);
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 4));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 8));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 12));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 16));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 20));
    let (a, b, c, d) = round4_avx512(a, b, c, d, expand_roundkey(rk, 24));
    
    last_4round_and_store(dst_src, a,b,c,d,rk);
}


#[cfg(test)]
mod tests {
    use crate::sm4::tests::get_tests_data;

    use super::*;

    #[test]
    fn test_block() {
        let (plain, wanted_cipher, rk) = get_tests_data(256);
        let mut cipher = [0u8; 256];

        block16_avx512(&mut cipher, &plain, &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_block16_avx512(b: &mut Bencher) {
        let (plain, _wanted_cipher, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            test::black_box(block16_avx512(&mut cipher, &plain, &rk))
        });
    }
}
