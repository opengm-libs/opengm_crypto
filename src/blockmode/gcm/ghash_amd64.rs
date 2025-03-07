use super::GHash;
use super::*;
use core::arch::x86_64::*;
use core::mem::transmute;

pub struct GHasherAmd64 {
    h: __m128i, // the key
    y: __m128i,
}

pub fn support_pmull_amd64() -> bool {
    is_x86_feature_detected!("pclmulqdq")
}

const ZERO: __m128i = unsafe { transmute([0u64, 0]) };

const BSWAP_MASK: __m128i = unsafe {
    transmute([15u8, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0])
};

const AND_MASK: __m128i =
    unsafe { transmute([0x0f0f0f0fu32, 0x0f0f0f0f, 0x0f0f0f0f, 0x0f0f0f0f]) };
const LOWER_MASK: __m128i =
    unsafe { transmute([0x0c040800u32, 0x0e060a02, 0x0d050901, 0x0f070b03]) };
const HIGHER_MASK: __m128i =
    unsafe { transmute([0xf070b030u32, 0xd0509010, 0xe060a020, 0xc0408000]) };

//

// reflect bits for each bytes of x.
// we use ghash_mul_reflect, thus a 16 bytes a 2 u64 doesn't need bitswap.
#[inline(always)]
fn bitswap_epi8(x: __m128i) -> __m128i {
    unsafe {
        let tmp2 = _mm_srli_epi16(x, 4);
        let tmp1 = _mm_and_si128(x, AND_MASK);
        let tmp2 = _mm_and_si128(tmp2, AND_MASK);
        let tmp1 = _mm_shuffle_epi8(HIGHER_MASK, tmp1);
        let tmp2 = _mm_shuffle_epi8(LOWER_MASK, tmp2);
        let tmp1 = _mm_xor_si128(tmp1, tmp2);
        tmp1
    }
}

// swap bytes of a.
// a 16 bytes a0 .. a15 should be (a0)_7 + (a0)_6 * x + (a0)_0 * x^7 + ... + (a15)_0 * x^127.
// where (a0)_7 is the MSB bit of a0, i.e., (a0 >> 7) & 1.
// thus xmm = transmute(bswap([a0..a15])) is just the poly, where a
// (xmm >> i) &  1 is the coefficient of x^i.
#[inline(always)]
fn bswap(a: __m128i) -> __m128i {
    unsafe { _mm_shuffle_epi8(a, BSWAP_MASK) }
}

impl Default for GHasherAmd64 {
    fn default() -> Self {
        Self { h: ZERO, y: ZERO }
    }
}

impl GHash for GHasherAmd64 {
    #[inline]
    fn init(&mut self, key: &[u8; 16]) {
        // key in reflected mode
        self.h = unsafe { bswap(transmute(*key)) };
        self.y = ZERO;
    }

    #[inline]
    fn reset(&mut self) {
        self.y = ZERO;
    }

    // update extends y with more polynomial terms from data. If data is not a
    // multiple of gcmBlockSize bytes long then the remainder is zero padded.
    #[inline]
    fn update(&mut self, data: &[u8]) {
        let full_blocks = (data.len() >> 4) << 4; //data.len() % 16

        self.update_blocks(&data[..full_blocks]);

        if data.len() != full_blocks {
            let mut partial_block = [0u8; BLOCK_SIZE];
            partial_block[..data.len() - full_blocks]
                .copy_from_slice(&data[full_blocks..]);
            self.update_blocks(&partial_block);
        }
    }

    fn sum(&self, h: &mut [u8; 16]) {
        *h = unsafe { transmute(bswap(self.y)) }
    }

    #[inline(always)]
    fn update_u64x2(&mut self, a: u64, b: u64) {
        unsafe {
            // reflect(A7 .. A0 || B7 .. B0) = b0 .. b7 || a0 .. a7
            let z = transmute([b, a]);
            self.y = _mm_xor_si128(self.y, z);
        };
        self.y = unsafe { ghash_mul_reflect(self.y, self.h) };
    }
}

impl GHasherAmd64 {
    // updateBlocks extends y with more polynomial terms from blocks, based on
    // Horner's rule. There must be a multiple of gcmBlockSize bytes in blocks.
    #[inline]
    fn update_blocks(&mut self, blocks: &[u8]) {
        for block in blocks.chunks_exact(16) {
            let b: [u8; 16] = block.try_into().unwrap();
            self.y = unsafe { _mm_xor_si128(self.y, bswap(transmute(b))) };
            self.y = unsafe { ghash_mul_reflect(self.y, self.h) };
        }
    }
}

#[inline(always)]
fn poly_mul_full(x: __m128i, y: __m128i) -> (__m128i, __m128i) {
    unsafe {
        let a = _mm_clmulepi64_si128::<0x00>(x, y);
        let b = _mm_clmulepi64_si128::<0x11>(x, y);

        let x = _mm_xor_si128(x, _mm_bsrli_si128::<8>(x));
        let y = _mm_xor_si128(y, _mm_bsrli_si128::<8>(y));
        let c = _mm_clmulepi64_si128::<0x00>(x, y);

        let d = _mm_xor_si128(c, _mm_xor_si128(a, b));
        let z0 = _mm_xor_si128(a, _mm_bslli_si128::<8>(d));
        let z1 = _mm_xor_si128(b, _mm_bsrli_si128::<8>(d));
        (z0, z1)
    }
}

// a,b is 128 bit reflected, the output is also reflected
// Note that reflect(a) * reflect(b) = reflect(a * b) >> 1.
// For
//   reflect(a) * reflect(b) = x^127 * a(x^-1) * x^127 * b(x^-1)
// = x^(127*2) (a * b) (x^-1)
// = x * a*b(x) // deg(a*b) = 2*127-1, the MSB bit is 0 and thus the LSB bit is 0 after reflect.
// = reflect(a * b) >> 1
// ref: "Intel® Carry-Less Multiplication Instruction and its Usage for Computing the GCM Mode"
#[target_feature(enable = "pclmulqdq")]
unsafe fn ghash_mul_reflect(a: __m128i, b: __m128i) -> __m128i {
    unsafe {
        let (tmp3, tmp6) = if true {
            let tmp3 = _mm_clmulepi64_si128::<0x00>(a, b);
            let tmp4 = _mm_clmulepi64_si128::<0x10>(a, b);
            let tmp5 = _mm_clmulepi64_si128::<0x01>(a, b);
            let tmp6 = _mm_clmulepi64_si128::<0x11>(a, b);
            let tmp4 = _mm_xor_si128(tmp4, tmp5);
            let tmp5 = _mm_slli_si128(tmp4, 8);
            let tmp4 = _mm_srli_si128(tmp4, 8);
            let tmp3 = _mm_xor_si128(tmp3, tmp5);
            let tmp6 = _mm_xor_si128(tmp6, tmp4);
            (tmp3, tmp6)
        } else {
            // may be a little faster, Karatsuba's method
            // one _mm_clmulepi64_si128 less.
            poly_mul_full(a, b)
        };

        let tmp7 = _mm_srli_epi32(tmp3, 31);
        let tmp8 = _mm_srli_epi32(tmp6, 31);
        let tmp3 = _mm_slli_epi32(tmp3, 1);
        let tmp6 = _mm_slli_epi32(tmp6, 1);
        let tmp9 = _mm_srli_si128(tmp7, 12);
        let tmp8 = _mm_slli_si128(tmp8, 4);
        let tmp7 = _mm_slli_si128(tmp7, 4);
        let tmp3 = _mm_or_si128(tmp3, tmp7);
        let tmp6 = _mm_or_si128(tmp6, tmp8);
        let tmp6 = _mm_or_si128(tmp6, tmp9);
        let tmp7 = _mm_slli_epi32(tmp3, 31);
        let tmp8 = _mm_slli_epi32(tmp3, 30);
        let tmp9 = _mm_slli_epi32(tmp3, 25);
        let tmp7 = _mm_xor_si128(tmp7, tmp8);
        let tmp7 = _mm_xor_si128(tmp7, tmp9);
        let tmp8 = _mm_srli_si128(tmp7, 4);
        let tmp7 = _mm_slli_si128(tmp7, 12);
        let tmp3 = _mm_xor_si128(tmp3, tmp7);
        let tmp2 = _mm_srli_epi32(tmp3, 1);
        let tmp4 = _mm_srli_epi32(tmp3, 2);
        let tmp5 = _mm_srli_epi32(tmp3, 7);
        let tmp2 = _mm_xor_si128(tmp2, tmp4);
        let tmp2 = _mm_xor_si128(tmp2, tmp5);
        let tmp2 = _mm_xor_si128(tmp2, tmp8);
        let tmp3 = _mm_xor_si128(tmp3, tmp2);
        let tmp6 = _mm_xor_si128(tmp6, tmp3);
        tmp6
    }
}

#[cfg(test)]
mod tests {
    use std::simd::u64x2;

    use super::*;
    fn print_u128(v: __m128i) {
        unsafe {
            let v: u64x2 = transmute(v);
            let v = v.to_array();
            println!("{:016x} {:016x}", v[1], v[0]);
        }
    }

    #[test]
    fn test_pmull_full() {
        let x = unsafe { transmute([0u64, 1]) };
        let y = unsafe { transmute([0u64, 1]) };
        let (z0, z1) = poly_mul_full(x, y);
        print_u128(z0);
        print_u128(z1);
        // println!("{:016x} {:016x}", z1, z0);
    }

    #[test]
    fn test_ghash_mull() {
        // x
        let mut x = [0u8; 16];
        x[0] = 1;
        let mut y = [0u8; 16];
        y[0] = 2; // y = x

        let x = unsafe { transmute(x) }; // 1
        let y = unsafe { transmute(y) }; // x

        let x = unsafe { ghash_mul_reflect(x, y) };
        print_u128(x);
    }

    #[cfg(test)]
    mod tests {
        use core::{arch::x86_64::__m128i, mem::transmute};
        extern crate test;
        use test::Bencher;

        use super::ghash_mul_reflect;
        #[bench]
        fn bench_ghash_mul(b: &mut Bencher) {
            let x: __m128i = unsafe { transmute([0x111111111u64, 0x12345678]) };
            let y: __m128i = unsafe { transmute([123422334u64, 1123456788]) };

            // 4.03 ns
            b.iter(|| {
                test::black_box(unsafe { ghash_mul_reflect(x, y) });
            });
        }
    }
}
