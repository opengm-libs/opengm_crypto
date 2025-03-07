// pub use block16::{block16_vaes, block16_inplace_vaes};
// pub use block8::{block8_vaes, block8_inplace_vaes};
pub use block16::*;
pub use block8::*;



mod block16 {
    use super::super::mem::load_block16::*;
    use super::super::prelude::*;
    const ZERO: __m512i = unsafe { transmute([0u64, 0, 0, 0, 0, 0, 0, 0]) };

    // 掩码, 取每个字节的低4位
    const C0F: __m512i = unsafe {
        transmute([
            0x0F0F0F0F0F0F0F0Fu64,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
        ])
    };

    const R08: __m512i = unsafe {
        transmute([
            0x0605040702010003u64,
            0x0E0D0C0F0A09080B,
            0x1615141712111013,
            0x1E1D1C1F1A19181B,
            0x2625242722212023,
            0x2E2D2C2F2A29282B,
            0x3635343732313033,
            0x3E3D3C3F3A39383B,
        ])
    };

    const R16: __m512i = unsafe {
        transmute([
            0x0504070601000302u64,
            0x0D0C0F0E09080B0A,
            0x1514171611101312,
            0x1D1C1F1E19181B1A,
            0x2524272621202322,
            0x2D2C2F2E29282B2A,
            0x3534373631303332,
            0x3D3C3F3E39383B3A,
        ])
    };

    const R24: __m512i = unsafe {
        transmute([
            0x0407060500030201u64,
            0x0C0F0E0D080B0A09,
            0x1417161510131211,
            0x1C1F1E1D181B1A19,
            0x2427262520232221,
            0x2C2F2E2D282B2A29,
            0x3437363530333231,
            0x3C3F3E3D383B3A39,
        ])
    };

    // ShiftRows逆变换
    const SHR: __m512i = unsafe {
        transmute([
            0x0B0E0104070A0D00u64,
            0x0306090C0F020508,
            0x1B1E1114171A1D10,
            0x1316191C1F121518,
            0x2B2E2124272A2D20,
            0x2326292C2F222528,
            0x3B3E3134373A3D30,
            0x3336393C3F323538,
        ])
    };

    // 线性变换A1(高低位)
    // M*y_l + c_l
    // M*y_h + c_h
    const M1L: __m512i = unsafe {
        transmute([
            0x37bb078bb23e820eu64,
            0xa82498142da11d91,
            0x37bb078bb23e820e,
            0xa82498142da11d91,
            0x37bb078bb23e820e,
            0xa82498142da11d91,
            0x37bb078bb23e820e,
            0xa82498142da11d91,
        ])
    };

    const M1H: __m512i = unsafe {
        transmute([
            0x7db29f5c21eec30u64,
            0xfd321fdca16e438,
            0x7db29f5c21eec30,
            0xfd321fdca16e438,
            0x7db29f5c21eec30,
            0xfd321fdca16e438,
            0x7db29f5c21eec30,
            0xfd321fdca16e438,
        ])
    };

    // 线性变换A2(高低位)
    const M2L: __m512i = unsafe {
        transmute([
            0x40f88a327ec6b40cu64,
            0x279fed5519a1d36b,
            0x40f88a327ec6b40c,
            0x279fed5519a1d36b,
            0x40f88a327ec6b40c,
            0x279fed5519a1d36b,
            0x40f88a327ec6b40c,
            0x279fed5519a1d36b,
        ])
    };
    const M2H: __m512i = unsafe {
        transmute([
            0x4dad1dfdd0308060u64,
            0x8d6ddd3d10f040a0,
            0x4dad1dfdd0308060,
            0x8d6ddd3d10f040a0,
            0x4dad1dfdd0308060,
            0x8d6ddd3d10f040a0,
            0x4dad1dfdd0308060,
            0x8d6ddd3d10f040a0,
        ])
    };

    #[inline(always)]
    unsafe fn round_vaes(a: __m512i, b: __m512i, c: __m512i, d: __m512i, rk: __m512i) -> __m512i {
        let mut x = _mm512_xor_si512(_mm512_xor_si512(_mm512_xor_si512(b, c), d), rk);

        /* A1 */
        let mut y = _mm512_and_si512(x, C0F);
        y = _mm512_shuffle_epi8(M1L, y);
        x = _mm512_srli_epi64(x, 4);
        x = _mm512_and_si512(x, C0F);
        x = _mm512_xor_si512(_mm512_shuffle_epi8(M1H, x), y);

        /* ShiftRows inverse */
        x = _mm512_shuffle_epi8(x, SHR);
        x = _mm512_aesenclast_epi128(x, ZERO);

        /* A2 */
        y = _mm512_and_si512(x, C0F);
        y = _mm512_shuffle_epi8(M2L, y);
        x = _mm512_srli_epi64(x, 4);
        x = _mm512_and_si512(x, C0F);
        x = _mm512_xor_si512(_mm512_shuffle_epi8(M2H, x), y);

        let mut y = _mm512_xor_si512(x, _mm512_shuffle_epi8(x, R08));
        y = _mm512_xor_si512(y, _mm512_shuffle_epi8(x, R16));
        y = _mm512_rol_epi32(y, 2);
        _mm512_xor_si512(
            _mm512_xor_si512(_mm512_xor_si512(_mm512_shuffle_epi8(x, R24.into()), y), x),
            a,
        )
    }

    #[inline(always)]
    unsafe fn round4_vaes(
        a: __m512i,
        b: __m512i,
        c: __m512i,
        d: __m512i,
        rk: [__m512i; 4],
    ) -> (__m512i, __m512i, __m512i, __m512i) {
        let a = round_vaes(a, b, c, d, rk[0]);
        let b = round_vaes(b, c, d, a, rk[1]);
        let c = round_vaes(c, d, a, b, rk[2]);
        let d = round_vaes(d, a, b, c, rk[3]);
        (a, b, c, d)
    }

    #[inline(always)]
    fn expand_roundkey(rk: &[u32], i: usize) -> [__m512i; 4] {
        unsafe {
            [
                _mm512_set1_epi32(rk[i] as i32),
                _mm512_set1_epi32(rk[i + 1] as i32),
                _mm512_set1_epi32(rk[i + 2] as i32),
                _mm512_set1_epi32(rk[i + 3] as i32),
            ]
        }
    }

    pub fn block16_vaes(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        unsafe { unsafe_block16_vaes(dst, src, rk) };
    }

    #[target_feature(enable = "avx512f", enable = "avx512bw", enable = "vaes")]
    unsafe fn unsafe_block16_vaes(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        let (a, b, c, d) = load_block16(src);

        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 28));

        store_block16(dst, a, b, c, d);
    }

    pub fn block16_vaes_inplace(dst_src: &mut [u8], rk: &[u32]) {
        unsafe { unsafe_block16_vaes_inplace(dst_src, rk) };
    }

    #[target_feature(enable = "avx512f", enable = "avx512bw", enable = "vaes")]
    unsafe fn unsafe_block16_vaes_inplace(dst_src: &mut [u8], rk: &[u32]) {
        let (a, b, c, d) = load_block16(dst_src);

        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 28));

        store_block16(dst_src, a, b, c, d);
    }
}

mod block8 {
    use super::super::mem::load_block8::*;
    use super::super::prelude::*;
    const ZERO: __m256i = unsafe { transmute([0u64, 0, 0, 0]) };

    // 掩码, 取每个字节的低4位
    const C0F: __m256i = unsafe {
        transmute([
            0x0F0F0F0F0F0F0F0Fu64,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
            0x0F0F0F0F0F0F0F0F,
        ])
    };

    const R08: __m256i = unsafe {
        transmute([
            0x0605040702010003u64,
            0x0E0D0C0F0A09080B,
            0x1615141712111013,
            0x1E1D1C1F1A19181B,
        ])
    };

    const R16: __m256i = unsafe {
        transmute([
            0x0504070601000302u64,
            0x0D0C0F0E09080B0A,
            0x1514171611101312,
            0x1D1C1F1E19181B1A,
        ])
    };

    const R24: __m256i = unsafe {
        transmute([
            0x0407060500030201u64,
            0x0C0F0E0D080B0A09,
            0x1417161510131211,
            0x1C1F1E1D181B1A19,
        ])
    };

    // ShiftRows逆变换
    const SHR: __m256i = unsafe {
        transmute([
            0x0B0E0104070A0D00u64,
            0x0306090C0F020508,
            0x1B1E1114171A1D10,
            0x1316191C1F121518,
        ])
    };

    // 线性变换A1(高低位)
    // M*y_l + c_l
    // M*y_h + c_h
    const M1L: __m256i = unsafe {
        transmute([
            0x37bb078bb23e820eu64,
            0xa82498142da11d91,
            0x37bb078bb23e820e,
            0xa82498142da11d91,
        ])
    };

    const M1H: __m256i = unsafe {
        transmute([
            0x7db29f5c21eec30u64,
            0xfd321fdca16e438,
            0x7db29f5c21eec30,
            0xfd321fdca16e438,
        ])
    };

    // 线性变换A2(高低位)
    const M2L: __m256i = unsafe {
        transmute([
            0x40f88a327ec6b40cu64,
            0x279fed5519a1d36b,
            0x40f88a327ec6b40c,
            0x279fed5519a1d36b,
        ])
    };
    const M2H: __m256i = unsafe {
        transmute([
            0x4dad1dfdd0308060u64,
            0x8d6ddd3d10f040a0,
            0x4dad1dfdd0308060,
            0x8d6ddd3d10f040a0,
        ])
    };

    #[inline(always)]
    unsafe fn round_vaes(a: __m256i, b: __m256i, c: __m256i, d: __m256i, rk: __m256i) -> __m256i {
        let mut x = _mm256_xor_si256(_mm256_xor_si256(_mm256_xor_si256(b, c), d), rk);

        /* A1 */
        let mut y = _mm256_and_si256(x, C0F);
        y = _mm256_shuffle_epi8(M1L, y);
        x = _mm256_srli_epi64(x, 4);
        x = _mm256_and_si256(x, C0F);
        x = _mm256_xor_si256(_mm256_shuffle_epi8(M1H, x), y);

        /* ShiftRows inverse */
        x = _mm256_shuffle_epi8(x, SHR);
        x = _mm256_aesenclast_epi128(x, ZERO);

        /* A2 */
        y = _mm256_and_si256(x, C0F);
        y = _mm256_shuffle_epi8(M2L, y);
        x = _mm256_srli_epi64(x, 4);
        x = _mm256_and_si256(x, C0F);
        x = _mm256_xor_si256(_mm256_shuffle_epi8(M2H, x), y);

        let mut y = _mm256_xor_si256(x, _mm256_shuffle_epi8(x, R08));
        y = _mm256_xor_si256(y, _mm256_shuffle_epi8(x, R16));
        y = _mm256_rol_epi32(y, 2);
        _mm256_xor_si256(
            _mm256_xor_si256(_mm256_xor_si256(_mm256_shuffle_epi8(x, R24.into()), y), x),
            a,
        )
    }

    #[inline(always)]
    unsafe fn round4_vaes(
        a: __m256i,
        b: __m256i,
        c: __m256i,
        d: __m256i,
        rk: [__m256i; 4],
    ) -> (__m256i, __m256i, __m256i, __m256i) {
        let a = round_vaes(a, b, c, d, rk[0]);
        let b = round_vaes(b, c, d, a, rk[1]);
        let c = round_vaes(c, d, a, b, rk[2]);
        let d = round_vaes(d, a, b, c, rk[3]);
        (a, b, c, d)
    }

    #[inline(always)]
    fn expand_roundkey(rk: &[u32], i: usize) -> [__m256i; 4] {
        unsafe {
            [
                _mm256_set1_epi32(rk[i] as i32),
                _mm256_set1_epi32(rk[i + 1] as i32),
                _mm256_set1_epi32(rk[i + 2] as i32),
                _mm256_set1_epi32(rk[i + 3] as i32),
            ]
        }
    }
    pub fn block8_vaes(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        unsafe { unsafe_block8_vaes(dst, src, rk) };
    }
    #[target_feature(enable = "avx2", enable = "avx", enable = "avx512f", enable = "avx512bw", enable = "vaes")]
    unsafe fn unsafe_block8_vaes(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        let (a, b, c, d) = load_block8(src);

        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 28));

        store_block8(dst, a, b, c, d);
    }

    pub  fn block8_vaes_inplace(dst_src: &mut [u8], rk: &[u32]) {

        unsafe { unsafe_block8_vaes_inplace(dst_src, rk) };

    }

    #[target_feature(enable = "avx2", enable = "avx", enable = "avx512f", enable = "avx512bw", enable = "vaes")]
    unsafe fn unsafe_block8_vaes_inplace(dst_src: &mut [u8], rk: &[u32]) {
        let (a, b, c, d) = load_block8(dst_src);

        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4_vaes(a, b, c, d, expand_roundkey(rk, 28));

        store_block8(dst_src, a, b, c, d);
    }
}


#[cfg(test)]
mod tests {
    use crate::sm4::tests::get_tests_data;

    use super::*;

    #[test]
    fn test_block16() {
        let (plain, wanted_cipher, rk) = get_tests_data(256);
        let mut cipher = [0u8; 256];
        block16_vaes(&mut cipher, &plain, &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    #[test]
    fn test_block8() {
        let (plain, wanted_cipher, rk) = get_tests_data(128);
        let mut cipher = [0u8; 128];

        block8_vaes(&mut cipher, &plain, &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_block16_vaes(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            test::black_box(block16_vaes(&mut cipher, &plain, &rk));
        });
    }

    #[bench]
    fn bench_block8_vaes(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(128);
        let mut cipher = [0; 128];
        b.iter(|| {
            test::black_box(block8_vaes(&mut cipher, &plain, &rk));
        });
    }

}
