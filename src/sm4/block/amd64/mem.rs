use super::super::byteorder::*;

pub(crate) mod load_block4 {
    use super::super::prelude::*;
    use super::*;

    const FLP32: __m128i = unsafe { transmute([0x0405060700010203u64, 0x0C0D0E0F08090A0B]) };

    #[inline(always)]
    unsafe fn load_a_sse2(src: &[u8]) -> __m128i {
        let a = _mm_set_epi32(
            get_u32_le(&src[16 * 3..]) as i32,
            get_u32_le(&src[16 * 2..]) as i32,
            get_u32_le(&src[16 * 1..]) as i32,
            get_u32_le(&src[16 * 0..]) as i32,
        );
        // IF use u32::from_be_bytes, the shuffle is not needed.
        // but I don't know if the from_be_bytes use movbe of use bytes op.
        _mm_shuffle_epi8(a, FLP32)
    }

    #[inline(always)]
    pub fn load_block4(src: &[u8]) -> (__m128i, __m128i, __m128i, __m128i) {
        unsafe {
            let a = load_a_sse2(src);
            let b = load_a_sse2(&src[4..]);
            let c = load_a_sse2(&src[8..]);
            let d = load_a_sse2(&src[12..]);
            (a, b, c, d)
        }
    }

    #[inline(always)]
    fn store(dst: &mut [u8], a: u32, b: u32, c: u32, d: u32) {
        put_u32_le(&mut dst[0..], d);
        put_u32_le(&mut dst[4..], c);
        put_u32_le(&mut dst[8..], b);
        put_u32_le(&mut dst[12..], a);
    }

    #[inline(always)]
    pub fn store_block4(dst: &mut [u8], a: __m128i, b: __m128i, c: __m128i, d: __m128i) {
        let mut va = [0u32; 4];
        let mut vb = [0u32; 4];
        let mut vc = [0u32; 4];
        let mut vd = [0u32; 4];
        unsafe {
            _mm_storeu_si128(va.as_mut_ptr() as *mut __m128i, _mm_shuffle_epi8(a, FLP32));
            _mm_storeu_si128(vb.as_mut_ptr() as *mut __m128i, _mm_shuffle_epi8(b, FLP32));
            _mm_storeu_si128(vc.as_mut_ptr() as *mut __m128i, _mm_shuffle_epi8(c, FLP32));
            _mm_storeu_si128(vd.as_mut_ptr() as *mut __m128i, _mm_shuffle_epi8(d, FLP32));
        }
        store(&mut dst[16 * 0..], va[0], vb[0], vc[0], vd[0]);
        store(&mut dst[16 * 1..], va[1], vb[1], vc[1], vd[1]);
        store(&mut dst[16 * 2..], va[2], vb[2], vc[2], vd[2]);
        store(&mut dst[16 * 3..], va[3], vb[3], vc[3], vd[3]);
    }
}

pub(crate) mod load_block8 {
    use super::super::prelude::*;
    use super::*;

    const FLP32: __m256i = unsafe {
        transmute([
            0x0405060700010203u64,
            0x0C0D0E0F08090A0B,
            0x1415161710111213,
            0x1C1D1E1F18191A1B,
        ])
    };

    #[inline(always)]
    unsafe fn load_a_256(src: &[u8]) -> __m256i {
        let a = _mm256_set_epi32(
            get_u32_le(&src[16 * 7..]) as i32,
            get_u32_le(&src[16 * 6..]) as i32,
            get_u32_le(&src[16 * 5..]) as i32,
            get_u32_le(&src[16 * 4..]) as i32,
            get_u32_le(&src[16 * 3..]) as i32,
            get_u32_le(&src[16 * 2..]) as i32,
            get_u32_le(&src[16 * 1..]) as i32,
            get_u32_le(&src[16 * 0..]) as i32,
        );
        _mm256_shuffle_epi8(a, FLP32)
    }

    #[inline(always)]
    pub fn load_block8(src: &[u8]) -> (__m256i, __m256i, __m256i, __m256i) {
        unsafe {
            let a = load_a_256(src);
            let b = load_a_256(&src[4..]);
            let c = load_a_256(&src[8..]);
            let d = load_a_256(&src[12..]);
            (a, b, c, d)
        }
    }

    #[inline(always)]
    fn store(dst: &mut [u8], a: u32, b: u32, c: u32, d: u32) {
        put_u32_le(&mut dst[0..], d);
        put_u32_le(&mut dst[4..], c);
        put_u32_le(&mut dst[8..], b);
        put_u32_le(&mut dst[12..], a);
    }

    #[inline(always)]
    pub fn store_block8(dst: &mut [u8], a: __m256i, b: __m256i, c: __m256i, d: __m256i) {
        let mut va = [0u32; 8];
        let mut vb = [0u32; 8];
        let mut vc = [0u32; 8];
        let mut vd = [0u32; 8];
        unsafe {
            _mm256_storeu_si256(
                va.as_mut_ptr() as *mut __m256i,
                _mm256_shuffle_epi8(a, FLP32),
            );
            _mm256_storeu_si256(
                vb.as_mut_ptr() as *mut __m256i,
                _mm256_shuffle_epi8(b, FLP32),
            );
            _mm256_storeu_si256(
                vc.as_mut_ptr() as *mut __m256i,
                _mm256_shuffle_epi8(c, FLP32),
            );
            _mm256_storeu_si256(
                vd.as_mut_ptr() as *mut __m256i,
                _mm256_shuffle_epi8(d, FLP32),
            );
        }
        store(&mut dst[16 * 0..], va[0], vb[0], vc[0], vd[0]);
        store(&mut dst[16 * 1..], va[1], vb[1], vc[1], vd[1]);
        store(&mut dst[16 * 2..], va[2], vb[2], vc[2], vd[2]);
        store(&mut dst[16 * 3..], va[3], vb[3], vc[3], vd[3]);
        store(&mut dst[16 * 4..], va[4], vb[4], vc[4], vd[4]);
        store(&mut dst[16 * 5..], va[5], vb[5], vc[5], vd[5]);
        store(&mut dst[16 * 6..], va[6], vb[6], vc[6], vd[6]);
        store(&mut dst[16 * 7..], va[7], vb[7], vc[7], vd[7]);
    }
}

pub(crate) mod load_block16 {
    use super::super::prelude::*;
    use super::*;

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

    #[inline(always)]
    unsafe fn load_a_512(src: &[u8]) -> __m512i {
        let a = _mm512_set_epi32(
            get_u32_le(&src[16 * 15..]) as i32,
            get_u32_le(&src[16 * 14..]) as i32,
            get_u32_le(&src[16 * 13..]) as i32,
            get_u32_le(&src[16 * 12..]) as i32,
            get_u32_le(&src[16 * 11..]) as i32,
            get_u32_le(&src[16 * 10..]) as i32,
            get_u32_le(&src[16 * 9..]) as i32,
            get_u32_le(&src[16 * 8..]) as i32,
            get_u32_le(&src[16 * 7..]) as i32,
            get_u32_le(&src[16 * 6..]) as i32,
            get_u32_le(&src[16 * 5..]) as i32,
            get_u32_le(&src[16 * 4..]) as i32,
            get_u32_le(&src[16 * 3..]) as i32,
            get_u32_le(&src[16 * 2..]) as i32,
            get_u32_le(&src[16 * 1..]) as i32,
            get_u32_le(&src[16 * 0..]) as i32,
        );
        _mm512_shuffle_epi8(a, FLP32)
    }

    #[inline(always)]
    pub fn load_block16(src: &[u8]) -> (__m512i, __m512i, __m512i, __m512i) {
        unsafe {
            let a = load_a_512(src);
            let b = load_a_512(&src[4..]);
            let c = load_a_512(&src[8..]);
            let d = load_a_512(&src[12..]);
            (a, b, c, d)
        }
    }

    #[inline(always)]
    fn store(dst: &mut [u8], a: u32, b: u32, c: u32, d: u32) {
        put_u32_le(&mut dst[0..], d);
        put_u32_le(&mut dst[4..], c);
        put_u32_le(&mut dst[8..], b);
        put_u32_le(&mut dst[12..], a);
    }

    #[inline(always)]
    pub fn store_block16(dst: &mut [u8], a: __m512i, b: __m512i, c: __m512i, d: __m512i) {
        let mut va = [0u32; 16];
        let mut vb = [0u32; 16];
        let mut vc = [0u32; 16];
        let mut vd = [0u32; 16];
        unsafe {
            _mm512_storeu_si512(va.as_mut_ptr() as *mut i32, _mm512_shuffle_epi8(a, FLP32));
            _mm512_storeu_si512(vb.as_mut_ptr() as *mut i32, _mm512_shuffle_epi8(b, FLP32));
            _mm512_storeu_si512(vc.as_mut_ptr() as *mut i32, _mm512_shuffle_epi8(c, FLP32));
            _mm512_storeu_si512(vd.as_mut_ptr() as *mut i32, _mm512_shuffle_epi8(d, FLP32));
        }
        store(&mut dst[16 * 0..], va[0], vb[0], vc[0], vd[0]);
        store(&mut dst[16 * 1..], va[1], vb[1], vc[1], vd[1]);
        store(&mut dst[16 * 2..], va[2], vb[2], vc[2], vd[2]);
        store(&mut dst[16 * 3..], va[3], vb[3], vc[3], vd[3]);
        store(&mut dst[16 * 4..], va[4], vb[4], vc[4], vd[4]);
        store(&mut dst[16 * 5..], va[5], vb[5], vc[5], vd[5]);
        store(&mut dst[16 * 6..], va[6], vb[6], vc[6], vd[6]);
        store(&mut dst[16 * 7..], va[7], vb[7], vc[7], vd[7]);
        store(&mut dst[16 * 8..], va[8], vb[8], vc[8], vd[8]);
        store(&mut dst[16 * 9..], va[9], vb[9], vc[9], vd[9]);
        store(&mut dst[16 * 10..], va[10], vb[10], vc[10], vd[10]);
        store(&mut dst[16 * 11..], va[11], vb[11], vc[11], vd[11]);
        store(&mut dst[16 * 12..], va[12], vb[12], vc[12], vd[12]);
        store(&mut dst[16 * 13..], va[13], vb[13], vc[13], vd[13]);
        store(&mut dst[16 * 14..], va[14], vb[14], vc[14], vd[14]);
        store(&mut dst[16 * 15..], va[15], vb[15], vc[15], vd[15]);
    }
}
