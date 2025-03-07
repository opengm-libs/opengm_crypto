
pub use block16::*;
pub use block8::*;



const M1_ITEM: u64 = 0x4c287db91a22505d;
const C1: i32 = 0b00111110;
const M3_ITEM: u64 = 0xf3ab34a974a6b589;
const C3: i32 = 0b11010011;

mod block16 {
    use super::super::mem::load_block16::*;
    use super::super::prelude::*;
    use super::*;

    // const M1: u64x8 = u64x8::from_array([M1_ITEM; 8]);
    const M1: __m512i = unsafe { transmute([M1_ITEM; 8]) };
    const M3: __m512i = unsafe { transmute([M3_ITEM; 8]) };

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

    #[inline(always)]
    unsafe fn round_gfni(a: __m512i, b: __m512i, c: __m512i, d: __m512i, rk: __m512i) -> __m512i {
        let mut t = _mm512_xor_si512(_mm512_xor_si512(_mm512_xor_si512(b, c), d), rk);
        t = _mm512_gf2p8affine_epi64_epi8::<C1>(t, M1);
        t = _mm512_gf2p8affineinv_epi64_epi8::<C3>(t, M3);

        let mut y = _mm512_xor_si512(t, _mm512_shuffle_epi8(t, R08));
        y = _mm512_xor_si512(y, _mm512_shuffle_epi8(t, R16));
        y = _mm512_rol_epi32(y, 2);
        _mm512_xor_si512(
            _mm512_xor_si512(_mm512_xor_si512(_mm512_shuffle_epi8(t, R24), y), t),
            a,
        )
    }

    #[inline(always)]
    unsafe fn round4_gfni(
        a: __m512i,
        b: __m512i,
        c: __m512i,
        d: __m512i,
        rk: [__m512i; 4],
    ) -> (__m512i, __m512i, __m512i, __m512i) {
        let a = round_gfni(a, b, c, d, rk[0]);
        let b = round_gfni(b, c, d, a, rk[1]);
        let c = round_gfni(c, d, a, b, rk[2]);
        let d = round_gfni(d, a, b, c, rk[3]);
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

    pub fn block16_gfni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        unsafe { unsafe_block16_gfni(dst, src, rk) };
    }

    #[target_feature(enable = "avx512f", enable = "avx512bw", enable = "gfni")]
    unsafe fn unsafe_block16_gfni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        let (a, b, c, d) = load_block16(src);

        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 28));

        store_block16(dst, a, b, c, d);
    }


    pub fn block16_gfni_inplace(dst_src: &mut [u8], rk: &[u32]) {
        unsafe { unsafe_block16_gfni_inplace(dst_src, rk) };
    }

    #[target_feature(enable = "avx512f", enable = "avx512bw", enable = "gfni")]
    unsafe fn unsafe_block16_gfni_inplace(inout: &mut [u8], rk: &[u32]) {
        unsafe {
            let (a, b, c, d) = load_block16(inout);

            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 0));
            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 4));
            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 8));
            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 12));
            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 16));
            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 20));
            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 24));
            let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 28));

            store_block16(inout, a, b, c, d);
        }
    }
}

mod block8 {
    use super::*;
    use super::super::mem::load_block8::*;
    use super::super::prelude::*;

    const M1: __m256i = unsafe { transmute([M1_ITEM; 4]) };
    const M3: __m256i = unsafe { transmute([M3_ITEM; 4]) };

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

    #[inline(always)]
    unsafe fn round_gfni(
        a: __m256i,
        b: __m256i,
        c: __m256i,
        d: __m256i,
        rk: __m256i,
    ) -> __m256i {
        let mut t = _mm256_xor_si256(_mm256_xor_si256(_mm256_xor_si256(b, c), d), rk);
        t = _mm256_gf2p8affine_epi64_epi8::<C1>(t, M1);
        t = _mm256_gf2p8affineinv_epi64_epi8::<C3>(t, M3);

        let mut y = _mm256_xor_si256(t, _mm256_shuffle_epi8(t, R08));
        y = _mm256_xor_si256(y, _mm256_shuffle_epi8(t, R16));
        y = _mm256_rol_epi32(y, 2);
        _mm256_xor_si256(
            _mm256_xor_si256(_mm256_xor_si256(_mm256_shuffle_epi8(t, R24), y), t),
            a,
        )
    }

    #[inline(always)]
    unsafe fn round4_gfni(
        a: __m256i,
        b: __m256i,
        c: __m256i,
        d: __m256i,
        rk: [__m256i; 4],
    ) -> (__m256i, __m256i, __m256i, __m256i) {
        let a = round_gfni(a, b, c, d, rk[0]);
        let b = round_gfni(b, c, d, a, rk[1]);
        let c = round_gfni(c, d, a, b, rk[2]);
        let d = round_gfni(d, a, b, c, rk[3]);
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

    pub fn block8_gfni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        unsafe { unsafe_block8_gfni(dst, src, rk) };
    }


    #[target_feature(enable = "avx2", enable = "avx", enable = "gfni")]
    unsafe fn unsafe_block8_gfni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        let (a, b, c, d) = load_block8(src);

        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 28));

        store_block8(dst, a, b, c, d);
    }

    pub fn block8_gfni_inplace(dst: &mut [u8],  rk: &[u32]) {
        unsafe { unsafe_block8_gfni_inplace(dst,  rk) };
    }


    #[target_feature(enable = "avx2", enable = "avx", enable = "gfni")]
    unsafe fn unsafe_block8_gfni_inplace(dst_src: &mut [u8], rk: &[u32]) {
        let (a, b, c, d) = load_block8(dst_src);

        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4_gfni(a, b, c, d, expand_roundkey(rk, 28));

        store_block8(dst_src, a, b, c, d);
    }

}

#[cfg(test)]
mod tests {
    use crate::sm4::tests::get_tests_data;

    use super::{*,super::*};

    #[test]
    fn test_block8_gfni() {
        if !gfni_avaliable(){
            println!("gfni not supported");
            return;
        }
        let (plain, wanted_cipher, rk) = get_tests_data(128);
        let mut cipher = [0; 128];
    
        block8_gfni(&mut cipher, &plain, &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    #[test]
    fn test_block16_gfni() {
        if !gfni_avaliable(){
            println!("gfni not supported");
            return;
        }
        let (plain, wanted_cipher, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
    
        block16_gfni(&mut cipher, &plain, &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_block16_gfni(b: &mut Bencher) {
        if !gfni_avaliable(){
            println!("gfni not supported");
            return;
        }
        let (plain, _, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            test::black_box(block16_gfni(&mut cipher, &plain, &rk));
        });
    }
    #[bench]
    fn bench_block8_gfni(b: &mut Bencher) {
        let (plain, _, rk) = get_tests_data(256);
        let mut cipher = [0; 256];
        b.iter(|| {
            test::black_box(block8_gfni(&mut cipher, &plain, &rk));
            test::black_box(block8_gfni(&mut cipher[128..], &plain[128..], &rk));
        });
    }
}
