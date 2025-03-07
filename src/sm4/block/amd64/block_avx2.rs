// use super::super::byteorder::*;
use super::super::generic::{load_block2, store_block2, x64};
use super::super::tables::*;
// use super::mem;
use super::prelude::*;

const FLP32: __m256i = unsafe {
    transmute([
        0x0405060700010203u64,
        0x0C0D0E0F08090A0B,
        0x1415161710111213,
        0x1C1D1E1F18191A1B,
    ])
};

const MASK16: __m256i = unsafe {
    transmute([
        0x0000ffff0000ffffu64,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
        0x0000ffff0000ffff,
    ])
};

#[inline(always)]
fn ltau(x: __m256i) -> __m256i {
    unsafe {
        let l = _mm256_and_si256(x, MASK16);
        let l = _mm256_i32gather_epi32(LTAU_TABLE16.as_ptr() as *const i32, l, 4);

        let h = _mm256_srli_epi32(x, 16);
        let h = _mm256_i32gather_epi32(LTAU_TABLE16.as_ptr() as *const i32, h, 4);
        // let h = _mm256_ror_epi32(h, 16);// avx512f
        let h = _mm256_xor_si256(_mm256_slli_epi32(h, 16), _mm256_srli_epi32(h, 16));

        _mm256_xor_si256(l, h)
    }
}

#[inline(always)]
fn expand_roundkey(rk: &[u32], i: usize) -> [__m256i; 4] {
    unsafe {
        [
            _mm256_set1_epi32(rk[i + 0] as i32),
            _mm256_set1_epi32(rk[i + 1] as i32),
            _mm256_set1_epi32(rk[i + 2] as i32),
            _mm256_set1_epi32(rk[i + 3] as i32),
        ]
    }
}

#[inline(always)]
fn round_avx2(a: __m256i, b: __m256i, c: __m256i, d: __m256i, rk: __m256i) -> __m256i {
    unsafe {
        let t = _mm256_xor_si256(_mm256_xor_si256(_mm256_xor_si256(c, d), b), rk);
        _mm256_xor_si256(a, ltau(t))
    }
}

// load 8 blocks and perform the first round use the sbox to void cache attack.
#[inline(always)]
fn load_and_first_4round_slow(input: &[u8], rk: &[u32]) -> (__m256i, __m256i, __m256i, __m256i) {
    let mut a = [0u64; 4];
    let mut b = [0u64; 4];
    let mut c = [0u64; 4];
    let mut d = [0u64; 4];
    for i in 0..4 {
        (a[i], b[i], c[i], d[i]) = load_block2(input);
        a[i] ^= x64::lt_slow(b[i] ^ c[i] ^ d[i] ^ (((rk[0] as u64) << 32) ^ (rk[0] as u64)));
        b[i] ^= x64::lt_slow(c[i] ^ d[i] ^ a[i] ^ (((rk[1] as u64) << 32) ^ (rk[1] as u64)));
        c[i] ^= x64::lt_slow(d[i] ^ a[i] ^ b[i] ^ (((rk[2] as u64) << 32) ^ (rk[2] as u64)));
        d[i] ^= x64::lt_slow(a[i] ^ b[i] ^ c[i] ^ (((rk[3] as u64) << 32) ^ (rk[3] as u64)));
    }

    unsafe {
        (
            _mm256_set_epi64x(a[0] as i64, a[1] as i64, a[2] as i64, a[3] as i64),
            _mm256_set_epi64x(b[0] as i64, b[1] as i64, b[2] as i64, b[3] as i64),
            _mm256_set_epi64x(c[0] as i64, c[1] as i64, c[2] as i64, c[3] as i64),
            _mm256_set_epi64x(d[0] as i64, d[1] as i64, d[2] as i64, d[3] as i64),
        )
    }
}
#[inline(always)]
fn last_4round_and_store(output: &mut [u8], a: __m256i, b: __m256i, c: __m256i, d: __m256i, rk: &[u32]) {
    let mut va = [0u64; 4];
    let mut vb = [0u64; 4];
    let mut vc = [0u64; 4];
    let mut vd = [0u64; 4];
    unsafe {
        _mm256_storeu_si256(va.as_mut_ptr() as *mut __m256i, a);
        _mm256_storeu_si256(vb.as_mut_ptr() as *mut __m256i, b);
        _mm256_storeu_si256(vc.as_mut_ptr() as *mut __m256i, c);
        _mm256_storeu_si256(vd.as_mut_ptr() as *mut __m256i, d);
    }
    for i in 0..4 {
        va[i] ^= x64::lt_slow(vb[i] ^ vc[i] ^ vd[i] ^ (((rk[28] as u64) << 32) ^ (rk[28] as u64)));
        vb[i] ^= x64::lt_slow(vc[i] ^ vd[i] ^ va[i] ^ (((rk[29] as u64) << 32) ^ (rk[29] as u64)));
        vc[i] ^= x64::lt_slow(vd[i] ^ va[i] ^ vb[i] ^ (((rk[30] as u64) << 32) ^ (rk[30] as u64)));
        vd[i] ^= x64::lt_slow(va[i] ^ vb[i] ^ vc[i] ^ (((rk[31] as u64) << 32) ^ (rk[31] as u64)));
    }

    store_block2(&mut output[32 * 0..], va[0], vb[0], vc[0], vd[0]);
    store_block2(&mut output[32 * 1..], va[1], vb[1], vc[1], vd[1]);
    store_block2(&mut output[32 * 2..], va[2], vb[2], vc[2], vd[2]);
    store_block2(&mut output[32 * 3..], va[3], vb[3], vc[3], vd[3]);
}

#[inline(always)]
fn round4_avx2(
    a: __m256i,
    b: __m256i,
    c: __m256i,
    d: __m256i,
    rk: [__m256i; 4],
) -> (__m256i, __m256i, __m256i, __m256i) {
    let a = round_avx2(a, b, c, d, rk[0]);
    let b = round_avx2(b, a, c, d, rk[1]);
    let c = round_avx2(c, d, a, b, rk[2]);
    let d = round_avx2(d, c, a, b, rk[3]);
    (a, b, c, d)
}

#[target_feature(enable = "avx2", enable = "avx")]
pub unsafe fn block8_avx2(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    let (a, b, c, d) = load_and_first_4round_slow(src, rk);

    let (a, b, c, d) = round4_avx2(a, b, c, d, expand_roundkey(rk, 4));
    let (a, b, c, d) = round4_avx2(a, b, c, d, expand_roundkey(rk, 8));
    let (a, b, c, d) = round4_avx2(a, b, c, d, expand_roundkey(rk, 12));
    let (a, b, c, d) = round4_avx2(a, b, c, d, expand_roundkey(rk, 16));
    let (a, b, c, d) = round4_avx2(a, b, c, d, expand_roundkey(rk, 20));
    let (a, b, c, d) = round4_avx2(a, b, c, d, expand_roundkey(rk, 24));

    last_4round_and_store(dst, a, b, c, d, rk);
}

#[cfg(test)]
mod tests {
    use crate::sm4::tests::get_tests_data;

    use super::*;

    #[test]
    fn test_block8_avx2() {
        let (plain, wanted_cipher, rk) = get_tests_data(128);
        let mut cipher = [0u8; 128];
        unsafe {
            block8_avx2(&mut cipher, &plain, &rk);
        }
        assert_eq!(&wanted_cipher, &cipher);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_block8_avx2(b: &mut Bencher) {
        let (plain, _wanted_cipher, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| unsafe {
            test::black_box(block8_avx2(&mut cipher[..128], &plain[..128], &rk));
            test::black_box(block8_avx2(&mut cipher[128..], &plain[128..], &rk));
        });
    }
}
