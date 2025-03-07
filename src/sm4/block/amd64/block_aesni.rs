use super::mem::load_block4::*;
use super::prelude::*;

const ZERO: __m128i = unsafe { transmute([0u64, 0]) };

// 掩码, 取每个字节的低4位
const C0F: __m128i = unsafe { transmute([0x0F0F0F0F0F0F0F0Fu64, 0x0F0F0F0F0F0F0F0F]) };

const R08: __m128i = unsafe { transmute([0x0605040702010003u64, 0x0E0D0C0F0A09080B]) };

const R16: __m128i = unsafe { transmute([0x0504070601000302u64, 0x0D0C0F0E09080B0A]) };

const R24: __m128i = unsafe { transmute([0x0407060500030201u64, 0x0C0F0E0D080B0A09]) };

// ShiftRows逆变换
const SHR: __m128i = unsafe { transmute([0x0B0E0104070A0D00u64, 0x0306090C0F020508]) };

// 线性变换A1(高低位)
// M*y_l + c_l
// M*y_h + c_h
const M1L: __m128i = unsafe { transmute([0x37bb078bb23e820eu64, 0xa82498142da11d91]) };

const M1H: __m128i = unsafe { transmute([0x7db29f5c21eec30u64, 0xfd321fdca16e438]) };

// 线性变换A2(高低位)
const M2L: __m128i = unsafe { transmute([0x40f88a327ec6b40cu64, 0x279fed5519a1d36b]) };
const M2H: __m128i = unsafe { transmute([0x4dad1dfdd0308060u64, 0x8d6ddd3d10f040a0]) };

#[target_feature(enable = "aes", enable = "sse2", enable = "ssse3")]
unsafe fn round_aesni(a: __m128i, b: __m128i, c: __m128i, d: __m128i, rk: __m128i) -> __m128i {
    let mut x = _mm_xor_si128(_mm_xor_si128(_mm_xor_si128(b, c), d), rk);

    /* A1 */
    let mut y = _mm_and_si128(x, C0F);
    y = _mm_shuffle_epi8(M1L, y);
    x = _mm_srli_epi64(x, 4);
    x = _mm_and_si128(x, C0F);
    x = _mm_xor_si128(_mm_shuffle_epi8(M1H, x), y);

    /* ShiftRows inverse */
    x = _mm_shuffle_epi8(x, SHR);
    x = _mm_aesenclast_si128(x, ZERO);

    /* A2 */
    y = _mm_and_si128(x, C0F);
    y = _mm_shuffle_epi8(M2L, y);
    x = _mm_srli_epi64(x, 4);
    x = _mm_and_si128(x, C0F);
    x = _mm_xor_si128(_mm_shuffle_epi8(M2H, x), y);

    let mut y = _mm_xor_si128(x, _mm_shuffle_epi8(x, R08));
    y = _mm_xor_si128(y, _mm_shuffle_epi8(x, R16));
    // y = _mm_rol_epi32(y, 2);//avx512f
    y = _mm_xor_si128(_mm_slli_epi32(y, 2), _mm_srli_epi32(y, 30));
    _mm_xor_si128(_mm_xor_si128(_mm_xor_si128(_mm_shuffle_epi8(x, R24), y), x), a)
}

#[inline(always)]
unsafe fn round4_aesni(
    a: __m128i,
    b: __m128i,
    c: __m128i,
    d: __m128i,
    rk: [__m128i; 4],
) -> (__m128i, __m128i, __m128i, __m128i) {
    let a = round_aesni(a, b, c, d, rk[0]);
    let b = round_aesni(b, c, d, a, rk[1]);
    let c = round_aesni(c, d, a, b, rk[2]);
    let d = round_aesni(d, a, b, c, rk[3]);
    (a, b, c, d)
}

#[inline(always)]
fn expand_roundkey(rk: &[u32], i: usize) -> [__m128i; 4] {
    unsafe {
        [
            _mm_set1_epi32(rk[i] as i32),
            _mm_set1_epi32(rk[i + 1] as i32),
            _mm_set1_epi32(rk[i + 2] as i32),
            _mm_set1_epi32(rk[i + 3] as i32),
        ]
    }
}

pub fn block4_aesni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    unsafe { unsafe_block4_aesni(dst, src, rk) };

}

#[target_feature(enable = "aes", enable = "sse2", enable = "ssse3")]
unsafe fn unsafe_block4_aesni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    let (a, b, c, d) = load_block4(src);

    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 0));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 4));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 8));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 12));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 16));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 20));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 24));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 28));

    store_block4(dst, a, b, c, d);
}

pub fn block4_aesni_inplace(dst: &mut [u8],  rk: &[u32]) {
    unsafe { unsafe_block4_aesni_inplace(dst,  rk) };

}

#[target_feature(enable = "avx512f", enable = "avx512bw", enable = "gfni")]
unsafe fn unsafe_block4_aesni_inplace(dst_src: &mut [u8], rk: &[u32]) {
    let (a, b, c, d) = load_block4(dst_src);

    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 0));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 4));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 8));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 12));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 16));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 20));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 24));
    let (a, b, c, d) = round4_aesni(a, b, c, d, expand_roundkey(rk, 28));

    store_block4(dst_src, a, b, c, d);
}

#[cfg(test)]
mod tests {
    use crate::sm4::tests::get_tests_data;

    use super::*;

    #[test]
    fn test_block() {
        let (plain, wanted_cipher, rk) = get_tests_data(64);
        let mut cipher = [0u8; 64];
        block4_aesni(&mut cipher, &plain[..], &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_block4_aesni(b: &mut Bencher) {
        let (plain, _wanted_cipher, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            block4_aesni(&mut cipher[..64], &plain[..64], &rk);
            block4_aesni(&mut cipher[64..128], &plain[64..128], &rk);
            block4_aesni(&mut cipher[128..192], &plain[128..192], &rk);
            block4_aesni(&mut cipher[192..], &plain[192..], &rk);
        });
    }
}
