#[cfg(target_arch = "x86")]
use std::arch::x86::*;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

use crate::{aligned16_mut, sm3::{util::*, BLOCK_SIZE}};
use core::mem::transmute;

//equals to _mm_rol_epi32($x, $n) in avx512f + avx512vl
macro_rules! _mm_rol_epi32_ {
    ($x:ident, $n: literal) => {{
        _mm_xor_si128(_mm_slli_epi32($x, $n), _mm_srli_epi32($x, 32 - $n))
    }};
}

//equals to _mm_alignr_epi32 in avx512f + avx512vl
macro_rules! _mm_alignr_epi32_ {
    ($x: ident, $y: ident, $n: literal) => {{
        _mm_xor_si128(_mm_srli_si128($y, $n * 4), _mm_slli_si128($x, 16 - $n * 4))
    }};
}

// msg_sched computes the next 3 Ws.
// in:
// w0: x, W0,  x,   x
// w1: x, W3,  W2,  W1
// w2: x, W6,  W5,  W4
// w3: x, W9,  W8,  W7
// w4: x, W12, W11, W10
// w5: x, W15, W14, W13
// out:
// w1: x, W3,  x,   x
// w2: x, W6,  W5,  W4
// w3: x, W9,  W8,  W7
// w4: x, W12, W11, W10
// w5: x, W15, W14, W13
// w0: x, W18, W17, W16
#[inline(always)]
unsafe fn msg_sched(w0: __m128i, w1: __m128i, w2: __m128i, w3: __m128i, w4: __m128i, w5: __m128i) -> __m128i {
    let t0 = _mm_shuffle_epi32(w0, 0b10111111); // t0: W0, x, x, x
    let t0 = _mm_alignr_epi32_!(w1, t0, 3); // t0: x,  W2, W1, W0

    let t1 = _mm_shuffle_epi32(w1, 0b10111111); // t1: W3, x, x, x
    let t1 = _mm_alignr_epi32_!(w2, t1, 3); // t1: x,  W5, W4, W3

    let t2 = _mm_xor_si128(w3, t0); // t2: W0 ^ W7
    let t3 = _mm_rol_epi32_!(w5, 15); // t3: W13 <<< 15
    let t2 = _mm_xor_si128(t2, t3); // t2: W0 ^ W7 ^ (W13 <<< 15)
    let t0 = _mm_rol_epi32_!(t2, 15);
    let t3 = _mm_rol_epi32_!(t2, 23);
    let t2 = _mm_xor_si128(t2, _mm_xor_si128(t0, t3)); // t2: P1(W0 ^ W7 ^ (W13 <<< 15))
    let t2 = _mm_xor_si128(t2, _mm_rol_epi32_!(t1, 7)); // t2: P1(W0 ^ W7 ^ (W13 <<< 15)) ^ (W3 <<< 7)
    _mm_xor_si128(t2, w4) // w0: x, W18, W17, W16
}

// in:
// w4: x, W12, W11, W10
// w5: x, W15, W14, W13
// w0: x, W18, W17, W16
// out:
// W:  W14    , W13    , W12,
// W:  W14^W18, W13^W17, W12^W16,
#[inline(always)]
fn store(w: &mut [u32; 8], w0: __m128i, w4: __m128i, w5: __m128i) {
    unsafe {
        let t0 = _mm_shuffle_epi32(w4, 0b10111111);
        let t1 = _mm_alignr_epi32_!(w5, t0, 3); // t1: x, W14, W13, W12
        _mm_store_si128(w.as_ptr() as *mut __m128i, t1);
        _mm_store_si128(w.as_ptr().offset(4) as *mut __m128i, _mm_xor_si128(w0, t1));
    }
}

use crate::sm3::generic::round;

// 调整端序
// t0 = _mm_shuffle_epi8(t0, flp);
// 将t0中保存的4个32比特的整数转换端序
const FLIP32: __m128i = unsafe { transmute([0x0405060700010203u64, 0x0C0D0E0F08090A0B]) };
pub(crate)  fn compress_amd64_sse2<'a>(iv: &mut [u32; 8], p: &'a [u8]) -> &'a [u8] {
    unsafe { unsafe_compress_amd64_sse2(iv, p) }
}
#[target_feature(enable = "ssse3", enable = "sse2")]
unsafe fn unsafe_compress_amd64_sse2<'a>(iv: &mut [u32; 8], p: &'a [u8]) -> &'a [u8] {
    let w = aligned16_mut!([0u32; 8]);
    let v = aligned16_mut!([0u32; 8]);
    assert!(w.as_ptr() as u32 % 16 == 0);
    assert!(v.as_ptr() as u32 % 16 == 0);



    let (chunks, tail) = p.as_chunks::<BLOCK_SIZE>();
    for chunk in chunks {
        let mut a = iv[0];
        let mut b = iv[1];
        let mut c = iv[2];
        let mut d = iv[3];
        let mut e = iv[4];
        let mut f = iv[5];
        let mut g = iv[6];
        let mut h = iv[7];

        //        3     2       1      0
        //  v3: W[15], W[14], W[13], W[12]
        //  v2: W[11], W[10], W[9],  W[8]
        //  v1: W[7],  W[6],  W[5],  W[4]
        //  v0: W[3],  W[2],  W[1],  W[0]
        unsafe {
            let v0 = _mm_loadu_si128(chunk.as_ptr() as *const __m128i);
            let v1 = _mm_loadu_si128(chunk[16..].as_ptr() as *const __m128i);
            let v2 = _mm_loadu_si128(chunk[32..].as_ptr() as *const __m128i);
            let v3 = _mm_loadu_si128(chunk[48..].as_ptr() as *const __m128i);
            let v0 = _mm_shuffle_epi8(v0, FLIP32);
            let v1 = _mm_shuffle_epi8(v1, FLIP32);
            let v2 = _mm_shuffle_epi8(v2, FLIP32);
            let v3 = _mm_shuffle_epi8(v3, FLIP32);

            _mm_store_si128(w.as_ptr() as *mut __m128i, v0);
            _mm_store_si128(w.as_ptr().offset(4) as *mut __m128i, _mm_xor_epi32(v0, v1));
            round!(0, w[0], w[4], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(1, w[1], w[5], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(2, w[2], w[6], c, d, a, b, g, h, e, f, ff0, gg0);
            round!(3, w[3], w[7], b, c, d, a, f, g, h, e, ff0, gg0);

            _mm_store_si128(v.as_ptr() as *mut __m128i, v1);
            _mm_store_si128(v.as_ptr().offset(4) as *mut __m128i, _mm_xor_epi32(v1, v2));
            round!(4, v[0], v[4], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(5, v[1], v[5], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(6, v[2], v[6], c, d, a, b, g, h, e, f, ff0, gg0);
            round!(7, v[3], v[7], b, c, d, a, f, g, h, e, ff0, gg0);

            _mm_store_si128(w.as_ptr() as *mut __m128i, v2);
            _mm_store_si128(w.as_ptr().offset(4) as *mut __m128i, _mm_xor_epi32(v2, v3));
            round!(8, w[0], w[4], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(9, w[1], w[5], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(10, w[2], w[6], c, d, a, b, g, h, e, f, ff0, gg0);
            round!(11, w[3], w[7], b, c, d, a, f, g, h, e, ff0, gg0);

            let w0 = _mm_shuffle_epi32(v0, 0b11001111); // w0: x, W0,  x,   x
            let w1 = _mm_shuffle_epi32(v0, 0b11111001); // w1: x, W3,  W2,  W1
            let w2 = v1; // w2: x, W6,  W5,  W4
            let w3 = _mm_alignr_epi32_!(v2, v1, 3); // w3: x, W9,  W8,  W7
            let w4 = _mm_alignr_epi32_!(v3, v2, 2); // w4: x, W12, W11, W10
            let w5 = _mm_shuffle_epi32(v3, 0b11111001); // w5: x, W15, W14, W13

            let w0 = msg_sched(w0, w1, w2, w3, w4, w5);
            store(v, w0, w4, w5);
            round!(12, v[0], v[4], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(13, v[1], v[5], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(14, v[2], v[6], c, d, a, b, g, h, e, f, ff0, gg0);

            let w1 = msg_sched(w1, w2, w3, w4, w5, w0);
            store(w, w1, w5, w0);
            round!(15, w[0], w[4], b, c, d, a, f, g, h, e, ff0, gg0);
            round!(16, w[1], w[5], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(17, w[2], w[6], d, a, b, c, h, e, f, g, ff1, gg1);

            let w2 = msg_sched(w2, w3, w4, w5, w0, w1);
            store(v, w2, w0, w1);
            round!(18, v[0], v[4], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(19, v[1], v[5], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(20, v[2], v[6], a, b, c, d, e, f, g, h, ff1, gg1);

            let w3 = msg_sched(w3, w4, w5, w0, w1, w2);
            store(w, w3, w1, w2);
            round!(21, w[0], w[4], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(22, w[1], w[5], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(23, w[2], w[6], b, c, d, a, f, g, h, e, ff1, gg1);

            let w4 = msg_sched(w4, w5, w0, w1, w2, w3);
            store(v, w4, w2, w3);
            round!(24, v[0], v[4], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(25, v[1], v[5], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(26, v[2], v[6], c, d, a, b, g, h, e, f, ff1, gg1);

            let w5 = msg_sched(w5, w0, w1, w2, w3, w4);
            store(w, w5, w3, w4);
            round!(27, w[0], w[4], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(28, w[1], w[5], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(29, w[2], w[6], d, a, b, c, h, e, f, g, ff1, gg1);

            let w0 = msg_sched(w0, w1, w2, w3, w4, w5);
            store(v, w0, w4, w5);
            round!(30, v[0], v[4], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(31, v[1], v[5], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(32, v[2], v[6], a, b, c, d, e, f, g, h, ff1, gg1);

            let w1 = msg_sched(w1, w2, w3, w4, w5, w0);
            store(w, w1, w5, w0);
            round!(33, w[0], w[4], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(34, w[1], w[5], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(35, w[2], w[6], b, c, d, a, f, g, h, e, ff1, gg1);

            let w2 = msg_sched(w2, w3, w4, w5, w0, w1);
            store(v, w2, w0, w1);
            round!(36, v[0], v[4], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(37, v[1], v[5], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(38, v[2], v[6], c, d, a, b, g, h, e, f, ff1, gg1);

            let w3 = msg_sched(w3, w4, w5, w0, w1, w2);
            store(w, w3, w1, w2);
            round!(39, w[0], w[4], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(40, w[1], w[5], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(41, w[2], w[6], d, a, b, c, h, e, f, g, ff1, gg1);

            let w4 = msg_sched(w4, w5, w0, w1, w2, w3);
            store(v, w4, w2, w3);
            round!(42, v[0], v[4], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(43, v[1], v[5], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(44, v[2], v[6], a, b, c, d, e, f, g, h, ff1, gg1);

            let w5 = msg_sched(w5, w0, w1, w2, w3, w4);
            store(w, w5, w3, w4);
            round!(45, w[0], w[4], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(46, w[1], w[5], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(47, w[2], w[6], b, c, d, a, f, g, h, e, ff1, gg1);

            let w0 = msg_sched(w0, w1, w2, w3, w4, w5);
            store(v, w0, w4, w5);
            round!(48, v[0], v[4], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(49, v[1], v[5], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(50, v[2], v[6], c, d, a, b, g, h, e, f, ff1, gg1);

            let w1 = msg_sched(w1, w2, w3, w4, w5, w0);
            store(w, w1, w5, w0);
            round!(51, w[0], w[4], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(52, w[1], w[5], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(53, w[2], w[6], d, a, b, c, h, e, f, g, ff1, gg1);

            let w2 = msg_sched(w2, w3, w4, w5, w0, w1);
            store(v, w2, w0, w1);
            round!(54, v[0], v[4], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(55, v[1], v[5], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(56, v[2], v[6], a, b, c, d, e, f, g, h, ff1, gg1);

            let w3 = msg_sched(w3, w4, w5, w0, w1, w2);
            store(w, w3, w1, w2);
            round!(57, w[0], w[4], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(58, w[1], w[5], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(59, w[2], w[6], b, c, d, a, f, g, h, e, ff1, gg1);

            let w4 = msg_sched(w4, w5, w0, w1, w2, w3);
            store(v, w4, w2, w3);
            round!(60, v[0], v[4], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(61, v[1], v[5], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(62, v[2], v[6], c, d, a, b, g, h, e, f, ff1, gg1);

            let w5 = msg_sched(w5, w0, w1, w2, w3, w4);
            store(w, w5, w3, w4);
            round!(63, w[0], w[4], b, c, d, a, f, g, h, e, ff1, gg1);
        }
        iv[0] ^= a;
        iv[1] ^= b;
        iv[2] ^= c;
        iv[3] ^= d;
        iv[4] ^= e;
        iv[5] ^= f;
        iv[6] ^= g;
        iv[7] ^= h;
    }
    tail
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_compress() {
        let mut iv = [0x7380166fu32, 0x4914b2b9, 0x172442d7, 0xda8a0600, 0xa96f30bc, 0x163138aa, 0xe38dee4d, 0xb0fb0e4e];
        let p: [u8; 64] = [1; 64];
        let expect: [u32; 8] = [0xb9122804, 0xc515b3c2, 0xb34a42f1, 0x06edad4e, 0x52ecd5c7, 0x8545dd67, 0xf42b4275, 0x900ed3ad];
        compress_amd64_sse2(&mut iv, p.as_slice());
        assert_eq!(iv, expect);
    }
}
