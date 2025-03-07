use crate::aligned16_mut;
use crate::sm3::{util::*, BLOCK_SIZE};
use core::arch::aarch64::*;
use core::mem::transmute;

macro_rules! vrol_u32 {
    ($v: ident, $n: literal) => {{
        const M: i32 = 32 - $n;
        veorq_u32(vshlq_n_u32::<$n>($v), vshrq_n_u32::<M>($v))
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
unsafe fn msg_sched(w0: uint32x4_t, w1: uint32x4_t, w2: uint32x4_t, w3: uint32x4_t, w4: uint32x4_t, w5: uint32x4_t) -> uint32x4_t {
    let t0 = vtrn1q_u32(w0, w0);
    let t0 = vextq_u32::<3>(t0, w1); // t0: x,  W2, W1, W0

    let t1 = vtrn1q_u32(w1, w1); // t1: W3, x, x, x
    let t1 = vextq_u32::<3>(t1, w2); // t1: x,  W5, W4, W3

    let t2 = veorq_u32(w3, t0); // t2: W0 ^ W7
    let t3 = vrol_u32!(w5, 15); // t3: W13 <<< 15
    let t2 = veorq_u32(t2, t3); // t2: W0 ^ W7 ^ (W13 <<< 15)
    let t0 = vrol_u32!(t2, 15);
    let t3 = vrol_u32!(t2, 23);
    let t2 = veorq_u32(t2, veorq_u32(t0, t3)); // t2: P1(W0 ^ W7 ^ (W13 <<< 15))
    let t2 = veorq_u32(t2, vrol_u32!(t1, 7)); // t2: P1(W0 ^ W7 ^ (W13 <<< 15)) ^ (W3 <<< 7)
    let w0 = veorq_u32(t2, w4); // w0: x, W18, W17, W16
    w0
}

// in:
// w4: x, W12, W11, W10
// w5: x, W15, W14, W13
// w0: x, W18, W17, W16
// out:
// W:  W14    , W13    , W12,
// W:  W14^W18, W13^W17, W12^W16,
#[inline(always)]
unsafe fn store(w: &mut [u32; 8], w0: uint32x4_t, w4: uint32x4_t, w5: uint32x4_t) {
    let t0 = vtrn1q_u32(w4, w4);// w12, x, x, x
    let t1 = vextq_u32::<3>(t0, w5); // x,W15, W14, W13, w12, x,x,x => x, W14, W13, w12
    let t2 = veorq_u32(w0, t1);
    vst2q_u32(w.as_ptr() as *mut u32, uint32x4x2_t(t1, t2));// t1.0, t2.0, t1.1, t2.1,...
}

use crate::sm3::generic::round;

pub(crate) fn compress_aarch64_neon<'a>(iv: &mut [u32; 8], p: &'a [u8]) -> &'a [u8] {
    unsafe { unsafe_compress_aarch64_neon(iv, p) }
}

#[target_feature(enable = "neon")]
unsafe fn unsafe_compress_aarch64_neon<'a>(iv: &mut [u32; 8], p: &'a [u8]) -> &'a [u8] {
    let w = aligned16_mut!([0u32; 8]);
    let v = aligned16_mut!([0u32; 8]);

    let (chunks, tail) = p.as_chunks::<{BLOCK_SIZE}>();
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
            //        3     2       1      0
            //  v3: W[15], W[14], W[13], W[12]
            //  v2: W[11], W[10], W[9],  W[8]
            //  v1: W[7],  W[6],  W[5],  W[4]
            //  v0: W[3],  W[2],  W[1],  W[0]
            let v0 = transmute(vrev32q_u8(transmute(vld1q_u32(chunk.as_ptr() as *const u32))));
            let v1 = transmute(vrev32q_u8(transmute(vld1q_u32(chunk[16..].as_ptr() as *const u32))));
            let v2 = transmute(vrev32q_u8(transmute(vld1q_u32(chunk[32..].as_ptr() as *const u32))));
            let v3 = transmute(vrev32q_u8(transmute(vld1q_u32(chunk[48..].as_ptr() as *const u32))));

            vst2q_u32(w.as_ptr() as *mut u32, uint32x4x2_t(v0, veorq_u32(v0, v1)));
            round!(0, w[0], w[1], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(1, w[2], w[3], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(2, w[4], w[5], c, d, a, b, g, h, e, f, ff0, gg0);
            round!(3, w[6], w[7], b, c, d, a, f, g, h, e, ff0, gg0);

            vst2q_u32(w.as_ptr() as *mut u32, uint32x4x2_t(v1, veorq_u32(v1, v2)));
            round!(4, w[0], w[1], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(5, w[2], w[3], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(6, w[4], w[5], c, d, a, b, g, h, e, f, ff0, gg0);
            round!(7, w[6], w[7], b, c, d, a, f, g, h, e, ff0, gg0);

            vst2q_u32(w.as_ptr() as *mut u32, uint32x4x2_t(v2, veorq_u32(v2, v3)));
            round!(8,  w[0], w[1], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(9,  w[2], w[3], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(10, w[4], w[5], c, d, a, b, g, h, e, f, ff0, gg0);
            round!(11, w[6], w[7], b, c, d, a, f, g, h, e, ff0, gg0);

            //  v3: W[15], W[14], W[13], W[12]
            //  v2: W[11], W[10], W[9],  W[8]
            //  v1: W[7],  W[6],  W[5],  W[4]
            //  v0: W[3],  W[2],  W[1],  W[0]

            let w0 = vextq_u32(v0, v0, 2); // w0: x, W0,  x,   x
            let w1 = vextq_u32(v0, v0, 1); // w1: x, W3,  W2,  W1
            let w2 = v1; // w2: x, W6,  W5,  W4
            let w3 = vextq_u32(v1, v2, 3); // w3: x, W9,  W8,  W7
            let w4 = vextq_u32(v2, v3, 2); // w4: x, W12, W11, W10
            let w5 = vextq_u32(v3, v3, 1); // w5: x, W15, W14, W13

            let w0 = msg_sched(w0, w1, w2, w3, w4, w5);
            store(v, w0, w4, w5);
            round!(12, v[0], v[1], a, b, c, d, e, f, g, h, ff0, gg0);
            round!(13, v[2], v[3], d, a, b, c, h, e, f, g, ff0, gg0);
            round!(14, v[4], v[5], c, d, a, b, g, h, e, f, ff0, gg0);

            let w1 = msg_sched(w1, w2, w3, w4, w5, w0);
            store(w, w1, w5, w0);
            round!(15, w[0], w[1], b, c, d, a, f, g, h, e, ff0, gg0);
            round!(16, w[2], w[3], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(17, w[4], w[5], d, a, b, c, h, e, f, g, ff1, gg1);

            let w2 = msg_sched(w2, w3, w4, w5, w0, w1);
            store(v, w2, w0, w1);
            round!(18, v[0], v[1], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(19, v[2], v[3], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(20, v[4], v[5], a, b, c, d, e, f, g, h, ff1, gg1);

            let w3 = msg_sched(w3, w4, w5, w0, w1, w2);
            store(w, w3, w1, w2);
            round!(21, w[0], w[1], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(22, w[2], w[3], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(23, w[4], w[5], b, c, d, a, f, g, h, e, ff1, gg1);

            let w4 = msg_sched(w4, w5, w0, w1, w2, w3);
            store(v, w4, w2, w3);
            round!(24, v[0], v[1], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(25, v[2], v[3], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(26, v[4], v[5], c, d, a, b, g, h, e, f, ff1, gg1);

            let w5 = msg_sched(w5, w0, w1, w2, w3, w4);
            store(w, w5, w3, w4);
            round!(27, w[0], w[1], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(28, w[2], w[3], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(29, w[4], w[5], d, a, b, c, h, e, f, g, ff1, gg1);

            let w0 = msg_sched(w0, w1, w2, w3, w4, w5);
            store(v, w0, w4, w5);
            round!(30, v[0], v[1], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(31, v[2], v[3], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(32, v[4], v[5], a, b, c, d, e, f, g, h, ff1, gg1);

            let w1 = msg_sched(w1, w2, w3, w4, w5, w0);
            store(w, w1, w5, w0);
            round!(33, w[0], w[1], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(34, w[2], w[3], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(35, w[4], w[5], b, c, d, a, f, g, h, e, ff1, gg1);

            let w2 = msg_sched(w2, w3, w4, w5, w0, w1);
            store(v, w2, w0, w1);
            round!(36, v[0], v[1], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(37, v[2], v[3], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(38, v[4], v[5], c, d, a, b, g, h, e, f, ff1, gg1);

            let w3 = msg_sched(w3, w4, w5, w0, w1, w2);
            store(w, w3, w1, w2);
            round!(39, w[0], w[1], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(40, w[2], w[3], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(41, w[4], w[5], d, a, b, c, h, e, f, g, ff1, gg1);

            let w4 = msg_sched(w4, w5, w0, w1, w2, w3);
            store(v, w4, w2, w3);
            round!(42, v[0], v[1], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(43, v[2], v[3], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(44, v[4], v[5], a, b, c, d, e, f, g, h, ff1, gg1);

            let w5 = msg_sched(w5, w0, w1, w2, w3, w4);
            store(w, w5, w3, w4);
            round!(45, w[0], w[1], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(46, w[2], w[3], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(47, w[4], w[5], b, c, d, a, f, g, h, e, ff1, gg1);

            let w0 = msg_sched(w0, w1, w2, w3, w4, w5);
            store(v, w0, w4, w5);
            round!(48, v[0], v[1], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(49, v[2], v[3], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(50, v[4], v[5], c, d, a, b, g, h, e, f, ff1, gg1);

            let w1 = msg_sched(w1, w2, w3, w4, w5, w0);
            store(w, w1, w5, w0);
            round!(51, w[0], w[1], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(52, w[2], w[3], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(53, w[4], w[5], d, a, b, c, h, e, f, g, ff1, gg1);

            let w2 = msg_sched(w2, w3, w4, w5, w0, w1);
            store(v, w2, w0, w1);
            round!(54, v[0], v[1], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(55, v[2], v[3], b, c, d, a, f, g, h, e, ff1, gg1);
            round!(56, v[4], v[5], a, b, c, d, e, f, g, h, ff1, gg1);

            let w3 = msg_sched(w3, w4, w5, w0, w1, w2);
            store(w, w3, w1, w2);
            round!(57, w[0], w[1], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(58, w[2], w[3], c, d, a, b, g, h, e, f, ff1, gg1);
            round!(59, w[4], w[5], b, c, d, a, f, g, h, e, ff1, gg1);

            let w4 = msg_sched(w4, w5, w0, w1, w2, w3);
            store(v, w4, w2, w3);
            round!(60, v[0], v[1], a, b, c, d, e, f, g, h, ff1, gg1);
            round!(61, v[2], v[3], d, a, b, c, h, e, f, g, ff1, gg1);
            round!(62, v[4], v[5], c, d, a, b, g, h, e, f, ff1, gg1);

            let w5 = msg_sched(w5, w0, w1, w2, w3, w4);
            store(w, w5, w3, w4);
            round!(63, w[0], w[1], b, c, d, a, f, g, h, e, ff1, gg1);
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
        compress_aarch64_neon(&mut iv, p.as_slice());
        assert_eq!(iv, expect);
    }
    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_compress(b: &mut Bencher) {
        let mut iv = [
            0x7380166fu32,
            0x4914b2b9,
            0x172442d7,
            0xda8a0600,
            0xa96f30bc,
            0x163138aa,
            0xe38dee4d,
            0xb0fb0e4e,
        ];
        let p: [u8; 64] = [1; 64];

        // 173.87 ns/iter
        b.iter(|| {
            test::black_box(compress_aarch64_neon(&mut iv, p.as_slice()));
        });
    }
}