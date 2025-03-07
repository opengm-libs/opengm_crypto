use super::GHash;
use core::arch::x86_64::*;
use core::mem::transmute;
use std::simd::u64x2;

pub struct GHasherAmd64 {
    h: __m128i, // the key
    y: __m128i,
}
const BLOCK_SIZE: usize = 16;
const TAG_SIZE: usize = 16;
const MIN_TAG_SIZE: usize = 12;
const STD_NONCE_SIZE: usize = 12;

pub fn support_amd64() -> bool {
    is_x86_feature_detected!("pclmulqdq")
        || is_x86_feature_detected!("vpclmulqdq")
}

const ZERO: __m128i = unsafe { transmute([0u64, 0]) };

// reflext each bytes of x.
fn reflect_xmm(x: __m128i) -> __m128i {
    unsafe {
        // let mut tmp1,tmp2;
        let and_mask: __m128i =
            _mm_set_epi32(0x0f0f0f0f, 0x0f0f0f0f, 0x0f0f0f0f, 0x0f0f0f0f);
        let lower_mask =
            _mm_set_epi32(0x0f070b03, 0x0d050901, 0x0e060a02, 0x0c040800);
        let higher_mask =
            _mm_set_epi32(0xf070b030u32 as i32, 0xd0509010u32 as i32, 0xe060a020u32 as i32, 0xc0408000u32 as i32);
            let tmp2 = _mm_srli_epi16(x, 4);
            let tmp1 = _mm_and_si128(x, and_mask);
            let tmp2 = _mm_and_si128(tmp2, and_mask);
            let tmp1 = _mm_shuffle_epi8(higher_mask, tmp1);
            let tmp2 = _mm_shuffle_epi8(lower_mask, tmp2);
            let tmp1 = _mm_xor_si128(tmp1, tmp2);
            // flip the byte order
            // let bswap_mask =
            //     _mm_set_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
        // return _mm_shuffle_epi8(tmp1, bswap_mask);
        tmp1
    }
}

impl Default for GHasherAmd64 {
    fn default() -> Self {
        Self { h: ZERO, y: ZERO }
    }
}

impl GHash for GHasherAmd64 {
    #[inline]
    fn init(&mut self, key: &[u8; 16]) {
        self.h = unsafe { reflect_xmm(transmute(*key)) };
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
        *h = unsafe { transmute(reflect_xmm(self.y)) };
    }

    #[inline(always)]
    fn update_u64x2(&mut self, a: u64, b: u64) {
        unsafe {
            // let mut a = a.to_be_bytes();
            // let mut b = b.to_be_bytes();
            // for i in 0..8 {
            //     a[i] = reverse(a[i]);
            //     b[i] = reverse(b[i]);
            // }
            // let a = u64::from_le_bytes(a);
            // let b = u64::from_le_bytes(b);

            let z = transmute([b,a]); // (a as u128) | ((b as u128) << 64);
            let z = reflect_xmm(z);
            let bswap_mask =
            _mm_set_epi8(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
            let z = _mm_shuffle_epi8(z, bswap_mask);
            self.y = _mm_xor_si128(self.y, z);
        };
        self.y = ghash_mul(self.y, self.h);
    }
}

impl GHasherAmd64 {
    // #[target_feature(enable = "pclmulqdq")]
    // unsafe fn ghash_unsafe(
    //     &mut self,
    //     ciphertext: &[u8],
    //     add: Option<&[u8]>,
    // ) -> [u8; 16] {
    //     let add_length = match add {
    //         Some(add) => {
    //             self.update(add);
    //             add.len() as u64
    //         }
    //         None => 0,
    //     };

    //     self.update(ciphertext);
    //     self.update_u64x2(
    //         add_length * 8,
    //         (ciphertext.len() * 8) as u64,
    //     );
    //     to_bytes(self.y)
    // }

    // updateBlocks extends y with more polynomial terms from blocks, based on
    // Horner's rule. There must be a multiple of gcmBlockSize bytes in blocks.
    #[inline]
    fn update_blocks(&mut self, blocks: &[u8]) {
        for block in blocks.chunks_exact(16) {
            let b: [u8; 16] = block.try_into().unwrap();
            self.y = unsafe { _mm_xor_si128(self.y, reflect_xmm(transmute(b))) };
            self.y = ghash_mul(self.y, self.h);
        }
    }

    // set y = y*H
    // #[inline]
    // fn mul(&self, y: &mut u128) {
    //     *y = ghash_mul(self.h, *y);
    // }

    // ghash update two u64: (a)_64 || (b)_64.
}

// fn reverse(b: u8) -> u8 {
//     let mut x = 0;
//     for i in 0..8 {
//         x |= ((b >> i) & 1) << (7 - i);
//     }
//     x
// }

// fn reverse_u64(y: u64) -> u64 {
//     let mut x = 0;
//     for i in 0..64 {
//         x |= ((y >> i) & 1) << (63 - i);
//     }
//     x
// }

// fn reverse_u128(y: u128) -> u128 {
//     unsafe { transmute(vrbitq_p8(transmute(y))) }
// }
// #[inline(always)]
// fn to_u128(v: &[u8; 16]) -> u128 {
//     unsafe { transmute(vrbitq_p8(transmute(u128::from_le_bytes(*v)))) }
// }
// #[inline(always)]
// fn to_bytes(v: u128) -> [u8; 16] {
//     let v: u128 = unsafe { transmute(vrbitq_p8(transmute(v))) };
//     v.to_le_bytes()
// }

fn print_u128(v: __m128i) {
    unsafe {
        let v: u64x2 = transmute(v);
        let v = v.to_array();
        println!("{:016x} {:016x}", v[1], v[0]);
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

const X128: __m128i = unsafe { transmute([0x87u64, 0]) };
// return x0 * x^128 = x^0 * (x^7 + x^2 + x + 1)
// deg(x0) < 64
#[inline(always)]
fn poly_mul_x128(x0: __m128i) -> __m128i {
    //   vmull_p64(x0, 0x87)
    unsafe { _mm_clmulepi64_si128::<0x00>(x0, X128) }
}

// // return x1 * x^192 = x^1 * (x^7 + x^2 + x + 1) * x^64
// #[inline(always)]
fn poly_mul_x192(x1: __m128i) -> __m128i {
    let z = unsafe { _mm_clmulepi64_si128::<0x01>(x1, X128) };
    let w = unsafe { _mm_clmulepi64_si128::<0x01>(z, X128) };
    unsafe { _mm_xor_si128(w, _mm_bslli_si128::<8>(z)) }
}

#[inline(always)]
fn poly_mod(z0: __m128i, z1: __m128i) -> __m128i {
    unsafe {
        _mm_xor_si128(z0, _mm_xor_si128(poly_mul_x128(z1), poly_mul_x192(z1)))
    }
}

#[inline(always)]
fn ghash_mul(x: __m128i, y: __m128i) -> __m128i {
    let (z0, z1) = poly_mul_full(x, y);
    poly_mod(z0, z1)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // let x = unsafe { transmute([0x111111111u64,0x12345678]) };
        // let y = unsafe { transmute([123422334u64,1123456788]) };

        // x
        let mut x = [0u8; 16];
        x[0] = 0x1;
        let mut y = [0u8; 16];
        y[0] = 0x2; // y = 1

        // for i in 0..16{
        //     x[i]
        // }

        let x = unsafe { transmute(x) }; // x^127 + 1
        let y = unsafe { transmute(y) };

        let x = ghash_mul(x, y); // x^128 = [0x87, 0]
        let x = ghash_mul(x, y); // x^128 = [0x87, 0]
        let x = ghash_mul(x, y); // x^128 = [0x87, 0]
        print_u128(x);
    }

    #[test]
    fn test_reverse_bit() {
        let mut x = [0u8;16];
        x[0]= 1;
        let x = unsafe{transmute(x)};
        print_u128(x);
        let x = reflect_xmm(x);
        print_u128(x);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_ghash_mul(b: &mut Bencher) {
        let x: __m128i = unsafe { transmute([0x111111111u64, 0x12345678]) };
        let y: __m128i = unsafe { transmute([123422334u64, 1123456788]) };

        // 9.79 ns/iter
        b.iter(|| {
            test::black_box(  ghash_mul(x, y));
        });
    }
}
