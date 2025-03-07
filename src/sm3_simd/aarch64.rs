use crate::{aligned16, sm3::{util::*, BLOCK_SIZE}};
use core::{arch::aarch64::*, cmp::min, iter::zip, mem::transmute};

//    input             output
// 0, 1, 2, 3       0, 4, 8, 12
// 4, 5, 6, 7       1, 5, 9, 13
// 8, 9, 10,11      2, 6, 10, 14
// 12,13,14,15      3, 7, 11, 15
#[inline(always)]
fn transpose4(a: uint32x4_t, b: uint32x4_t, c: uint32x4_t, d: uint32x4_t) -> (uint32x4_t, uint32x4_t, uint32x4_t, uint32x4_t) {
    unsafe {
        let t0 = vtrn1q_u32(a, b); // 0, 4, 2, 6
        let t1 = vtrn2q_u32(a, b); // 1, 5, 3, 7
        let t2 = vtrn1q_u32(c, d); // 8, 12, 10, 14
        let t3 = vtrn2q_u32(c, d); // 9, 13, 11, 15
        (
            transmute(vtrn1q_u64(transmute(t0), transmute(t2))), // 0, 4, 8, 12
            transmute(vtrn1q_u64(transmute(t1), transmute(t3))), // 1, 5, 9, 13
            transmute(vtrn2q_u64(transmute(t0), transmute(t2))), // 2, 6, 10, 14
            transmute(vtrn2q_u64(transmute(t1), transmute(t3))), // 3, 7, 11, 15
        )
    }
}

// for x = 0b0101
const MASK: [uint32x4_t; 16] = [
    unsafe { transmute([0, 0, 0, 0]) },
    unsafe { transmute([-1, 0, 0, 0]) },
    unsafe { transmute([0, -1, 0, 0]) },
    unsafe { transmute([-1, -1, 0, 0]) },
    unsafe { transmute([0, 0, -1, 0]) },
    unsafe { transmute([-1, 0, -1, 0]) },
    unsafe { transmute([0, -1, -1, 0]) },
    unsafe { transmute([-1, -1, -1, 0]) },
    unsafe { transmute([0, 0, 0, -1]) },
    unsafe { transmute([-1, 0, 0, -1]) },
    unsafe { transmute([0, -1, 0, -1]) },
    unsafe { transmute([-1, -1, 0, -1]) },
    unsafe { transmute([0, 0, -1, -1]) },
    unsafe { transmute([-1, 0, -1, -1]) },
    unsafe { transmute([0, -1, -1, -1]) },
    unsafe { transmute([-1, -1, -1, -1]) },
];

#[inline(always)]
fn get_mask(b: u32) -> uint32x4_t {
    MASK[b as usize % 16]
}

macro_rules! vrolq_u32 {
    ($v: expr, $n: literal) => {{
        const M: i32 = (32 - $n) as i32;
        veorq_u32(vshlq_n_u32::<$n>($v), vshrq_n_u32::<M>($v))
    }};
}

#[inline(always)]
fn ff0(x: uint32x4_t, y: uint32x4_t, z: uint32x4_t) -> uint32x4_t {
    unsafe { veorq_u32(veorq_u32(x, y), z) }
}

#[inline(always)]
fn gg0(x: uint32x4_t, y: uint32x4_t, z: uint32x4_t) -> uint32x4_t {
    unsafe { veorq_u32(veorq_u32(x, y), z) }
}

#[inline(always)]
fn ff1(x: uint32x4_t, y: uint32x4_t, z: uint32x4_t) -> uint32x4_t {
    // ((x | z) & y) | (x & z)
    unsafe { vorrq_u32(vandq_u32(vorrq_u32(x, z), y), vandq_u32(x, z)) }
}

#[inline(always)]
fn gg1(x: uint32x4_t, y: uint32x4_t, z: uint32x4_t) -> uint32x4_t {
    // z ^ (x & (y ^ z))
    unsafe { veorq_u32(vandq_u32(veorq_u32(y, z), x), z) }
}

#[inline(always)]
fn p0(x: uint32x4_t) -> uint32x4_t {
    unsafe {
        let y = vrolq_u32!(x, 9);
        let z = vrolq_u32!(x, 17);
        veorq_u32(veorq_u32(x, y), z)
    }
}

#[inline(always)]
fn p1(x: uint32x4_t) -> uint32x4_t {
    // x ^ x.rotate_left(15) ^ x.rotate_left(23)
    unsafe {
        let y = vrolq_u32!(x, 15);
        let z = vrolq_u32!(x, 23);
        veorq_u32(veorq_u32(x, y), z)
    }
}

macro_rules! RoundNeon {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {{
        let x = vrolq_u32!($a, 12);
        let ss1 = vaddq_u32(vaddq_u32(x, $e), vdupq_n_u32(T[$i]));
        let ss1 = vrolq_u32!(ss1, 7);
        let ss2 = veorq_u32(ss1, x);
        let tt1 = vaddq_u32(
            vaddq_u32(vaddq_u32($ff($a, $b, $c), $d), ss2),
            veorq_u32($w[$i % 16], $w[($i + 4) % 16]),
        );
        let tt2 = vaddq_u32(vaddq_u32(vaddq_u32($gg($e, $f, $g), $h), ss1), $w[$i % 16]);
        $b = vrolq_u32!($b, 9);
        $d = tt1;
        $f = vrolq_u32!($f, 19);
        $h = p0(tt2);
    }};
}

macro_rules! RoundWithMsgScheNeon {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {
        RoundNeon!($i, $w, $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        $w[$i % 16] = msg_sched_neon($w, $i);
    };
}

macro_rules! Round4Neon {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {{
        RoundNeon!($i + 0, $w, $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        RoundNeon!($i + 1, $w, $d, $a, $b, $c, $h, $e, $f, $g, $ff, $gg);
        RoundNeon!($i + 2, $w, $c, $d, $a, $b, $g, $h, $e, $f, $ff, $gg);
        RoundNeon!($i + 3, $w, $b, $c, $d, $a, $f, $g, $h, $e, $ff, $gg);
    }};
}

macro_rules! Round4WithMsgScheNeon {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {{
        RoundWithMsgScheNeon!($i + 0, $w, $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        RoundWithMsgScheNeon!($i + 1, $w, $d, $a, $b, $c, $h, $e, $f, $g, $ff, $gg);
        RoundWithMsgScheNeon!($i + 2, $w, $c, $d, $a, $b, $g, $h, $e, $f, $ff, $gg);
        RoundWithMsgScheNeon!($i + 3, $w, $b, $c, $d, $a, $f, $g, $h, $e, $ff, $gg);
    }};
}

// p1(w0 ^ w7 ^ w13.rotate_left(15)) ^ w3.rotate_left(7) ^ w10
#[inline(always)]
unsafe fn msg_sched_neon(w: &[uint32x4_t], i: usize) -> uint32x4_t {
    let t0 = veorq_u32(veorq_u32(w[(i + 0) % 16], w[(i + 7) % 16]), vrolq_u32!(w[(i + 13) % 16], 15));
    let t1 = veorq_u32(vrolq_u32!(w[(i + 3) % 16], 7), w[(i + 10) % 16]);
    veorq_u32(t1, p1(t0))
}

// compress one block for each pi.
#[target_feature(enable = "neon")]
pub unsafe fn load_message(m0: &[u8], m1: &[u8], m2: &[u8], m3: &[u8], mask: u32) -> [uint32x4_t; 16] {
    unsafe {
        if true {
            let mut w  = [transmute([0; 4]); 16];
            const DUMMY: &[u8] = aligned16!([0u8; 64]);

            let p0 = if mask & 1 == 0 { DUMMY } else { m0 };
            let p1 = if mask & 2 == 0 { DUMMY } else { m1 };
            let p2 = if mask & 4 == 0 { DUMMY } else { m2 };
            let p3 = if mask & 8 == 0 { DUMMY } else { m3 };
            let a: uint32x4x4_t = transmute::<[u8; 64], uint32x4x4_t>(p0[0..64].try_into().unwrap());
            let b: uint32x4x4_t = transmute::<[u8; 64], uint32x4x4_t>(p1[0..64].try_into().unwrap());
            let c: uint32x4x4_t = transmute::<[u8; 64], uint32x4x4_t>(p2[0..64].try_into().unwrap());
            let d: uint32x4x4_t = transmute::<[u8; 64], uint32x4x4_t>(p3[0..64].try_into().unwrap());
            
            (
                (w[0], w[1], w[2], w[3]),
                (w[4], w[5], w[6], w[7]),
                (w[8], w[9], w[10], w[11]),
                (w[12], w[13], w[14], w[15]),
            ) = (
                transpose4(
                    transmute(vrev32q_u8(transmute(a.0))),
                    transmute(vrev32q_u8(transmute(b.0))),
                    transmute(vrev32q_u8(transmute(c.0))),
                    transmute(vrev32q_u8(transmute(d.0))),
                ),
                transpose4(
                    transmute(vrev32q_u8(transmute(a.1))),
                    transmute(vrev32q_u8(transmute(b.1))),
                    transmute(vrev32q_u8(transmute(c.1))),
                    transmute(vrev32q_u8(transmute(d.1))),
                ),
                transpose4(
                    transmute(vrev32q_u8(transmute(a.2))),
                    transmute(vrev32q_u8(transmute(b.2))),
                    transmute(vrev32q_u8(transmute(c.2))),
                    transmute(vrev32q_u8(transmute(d.2))),
                ),
                transpose4(
                    transmute(vrev32q_u8(transmute(a.3))),
                    transmute(vrev32q_u8(transmute(b.3))),
                    transmute(vrev32q_u8(transmute(c.3))),
                    transmute(vrev32q_u8(transmute(d.3))),
                ),
            );
            w
        } else {
            let mut w: [uint32x4_t; 16] = [transmute([0; 4]); 16];
            const DUMMY: &[u8] = aligned16!([0u8; 64]);

            let p0 = if mask & 1 == 0 { DUMMY } else { m0 };
            let p1 = if mask & 2 == 0 { DUMMY } else { m1 };
            let p2 = if mask & 4 == 0 { DUMMY } else { m2 };
            let p3 = if mask & 8 == 0 { DUMMY } else { m3 };

            for (v, (q0, (q1, (q2, q3)))) in zip(
                &mut w,
                zip(
                    p0[..64].chunks_exact(4),
                    zip(p1[..64].chunks_exact(4), zip(p2[..64].chunks_exact(4), p3[..64].chunks_exact(4))),
                ),
            ) {
                *v = transmute([
                    u32::from_be_bytes(q0.try_into().unwrap()),
                    u32::from_be_bytes(q1.try_into().unwrap()),
                    u32::from_be_bytes(q2.try_into().unwrap()),
                    u32::from_be_bytes(q3.try_into().unwrap()),
                ]);
            }
            w
        }
    }
}

// compress one block for each pi.
#[inline(always)]
pub(crate) unsafe fn compress_block_aarch64_neon(iv: &mut [uint32x4_t; 8], m0: &[u8], m1: &[u8], m2: &[u8], m3: &[u8], mask: u32) {
    unsafe {
        let mut w = load_message(m0, m1, m2, m3, mask);
        unsafe_compress_aarch64_neon(iv, &mut w, mask);
    }
}

// w[0..15], 4 lane, each lane for a message.
#[target_feature(enable = "neon")]
unsafe fn unsafe_compress_aarch64_neon(iv: &mut [uint32x4_t; 8], w: &mut [uint32x4_t], mask: u32) {
    unsafe {
        let mut a = iv[0];
        let mut b = iv[1];
        let mut c = iv[2];
        let mut d = iv[3];
        let mut e = iv[4];
        let mut f = iv[5];
        let mut g = iv[6];
        let mut h = iv[7];

        Round4WithMsgScheNeon!(0, w, a, b, c, d, e, f, g, h, ff0, gg0);
        Round4WithMsgScheNeon!(4, w, a, b, c, d, e, f, g, h, ff0, gg0);
        Round4WithMsgScheNeon!(8, w, a, b, c, d, e, f, g, h, ff0, gg0);
        Round4WithMsgScheNeon!(12, w, a, b, c, d, e, f, g, h, ff0, gg0);

        Round4WithMsgScheNeon!(16, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgScheNeon!(20, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgScheNeon!(24, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgScheNeon!(28, w, a, b, c, d, e, f, g, h, ff1, gg1);

        Round4WithMsgScheNeon!(32, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgScheNeon!(36, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgScheNeon!(40, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgScheNeon!(44, w, a, b, c, d, e, f, g, h, ff1, gg1);

        Round4WithMsgScheNeon!(48, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4Neon!(52, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4Neon!(56, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4Neon!(60, w, a, b, c, d, e, f, g, h, ff1, gg1);

        let mask = get_mask(mask);
        iv[0] = veorq_u32(iv[0], vandq_u32(mask, a));
        iv[1] = veorq_u32(iv[1], vandq_u32(mask, b));
        iv[2] = veorq_u32(iv[2], vandq_u32(mask, c));
        iv[3] = veorq_u32(iv[3], vandq_u32(mask, d));
        iv[4] = veorq_u32(iv[4], vandq_u32(mask, e));
        iv[5] = veorq_u32(iv[5], vandq_u32(mask, f));
        iv[6] = veorq_u32(iv[6], vandq_u32(mask, g));
        iv[7] = veorq_u32(iv[7], vandq_u32(mask, h));
    }
}

pub fn new() -> Compressor {
    Compressor::new()
}

// For Neon, the Digest can update four messages.
#[derive(Debug, Copy, Clone)]
pub struct Compressor([uint32x4_t; 8]);

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

impl Compressor {
    #[rustfmt::skip]
    pub fn new() -> Compressor {
        unsafe {
            Compressor([
                transmute([0x7380166fu32, 0x7380166f, 0x7380166f, 0x7380166f]), // iv0 x4
                transmute([0x4914b2b9u32, 0x4914b2b9, 0x4914b2b9, 0x4914b2b9]), // iv1 x4
                transmute([0x172442d7u32, 0x172442d7, 0x172442d7, 0x172442d7]), // iv2 x4
                transmute([0xda8a0600u32, 0xda8a0600, 0xda8a0600, 0xda8a0600]), // iv3 x4,
                transmute([0xa96f30bcu32, 0xa96f30bc, 0xa96f30bc, 0xa96f30bc]), // iv4 x4
                transmute([0x163138aau32, 0x163138aa, 0x163138aa, 0x163138aa]), // iv5 x4
                transmute([0xe38dee4du32, 0xe38dee4d, 0xe38dee4d, 0xe38dee4d]), // iv6 x4
                transmute([0xb0fb0e4eu32, 0xb0fb0e4e, 0xb0fb0e4e, 0xb0fb0e4e]), // iv7 x4
            ])
        }
    }

    // update one block
    #[inline]
    fn write_block(&mut self, p0: &[u8], p1: &[u8], p2: &[u8], p3: &[u8]) {
        unsafe { compress_block_aarch64_neon(&mut self.0, p0, p1, p2, p3, 0b1111) };
    }

    #[inline]
    fn write_block_masked(&mut self, p0: &[u8], p1: &[u8], p2: &[u8], p3: &[u8], mask: u32) {
        unsafe { compress_block_aarch64_neon(&mut self.0, p0, p1, p2, p3, mask) };
    }

    // reverse each u32's endian
    #[target_feature(enable = "neon")]
    unsafe fn rev32(&self) -> [uint32x4_t; 8] {
        let mut reversed = [transmute([0; 4]); 8];
        for (i, x) in &mut self.0.iter().enumerate() {
            reversed[i] = unsafe { transmute(vrev32q_u8(transmute(*x))) };
        }
        reversed
    }
    
    #[inline(always)]
    fn dump(&self) -> [[u8; 32]; 4] {
        unsafe { self.dump_unsafe() }
    }

    #[target_feature(enable = "neon")]
    unsafe fn dump_unsafe(&self) -> [[u8; 32]; 4] {
        unsafe {
            let w = self.rev32();
            let digest = [[0; 32]; 4];
            let (a, b, c, d) = transpose4(w[0], w[2], w[4], w[6]);
            let (e, f, g, h) = transpose4(w[1], w[3], w[5], w[7]);
            vst2q_u32(digest[0].as_ptr() as *mut u32, uint32x4x2_t(a, e));
            vst2q_u32(digest[1].as_ptr() as *mut u32, uint32x4x2_t(b, f));
            vst2q_u32(digest[2].as_ptr() as *mut u32, uint32x4x2_t(c, g));
            vst2q_u32(digest[3].as_ptr() as *mut u32, uint32x4x2_t(d, h));

            digest
        }
    }
}

// computes digest of four messages with equal length.
pub fn sum_equal4(m0: &[u8], m1: &[u8], m2: &[u8], m3: &[u8]) -> [[u8; 32]; 4] {
    let mut compressor = Compressor::new();
    // length must be equal.
    let length = m0.len();
    assert_eq!(length, m1.len());
    assert_eq!(length, m2.len());
    assert_eq!(length, m3.len());

    let blocks = length / 64;
    for i in 0..blocks {
        compressor.write_block(&m0[i * 64..], &m1[i * 64..], &m2[i * 64..], &m3[i * 64..]);
    }

    // handle the tails
    let mut buf = [[0; 128]; 4];
    let total_len = m0.len() as u64 * 8;
    let mut n = m0.len() - blocks * 64; // there have tail_len bytes to go.
    buf[0][..n].copy_from_slice(&m0[blocks * 64..]);
    buf[1][..n].copy_from_slice(&m1[blocks * 64..]);
    buf[2][..n].copy_from_slice(&m2[blocks * 64..]);
    buf[3][..n].copy_from_slice(&m3[blocks * 64..]);

    buf[0][n] = 0x80u8;
    buf[1][n] = 0x80u8;
    buf[2][n] = 0x80u8;
    buf[3][n] = 0x80u8;

    n += 1;
    let b = total_len.to_be_bytes();
    if n + 8 <= BLOCK_SIZE {
        buf[0][56..64].copy_from_slice(&b);
        buf[1][56..64].copy_from_slice(&b);
        buf[2][56..64].copy_from_slice(&b);
        buf[3][56..64].copy_from_slice(&b);
        compressor.write_block(&buf[0], &buf[1], &buf[2], &buf[3]);
    } else {
        buf[0][120..128].copy_from_slice(&b);
        buf[1][120..128].copy_from_slice(&b);
        buf[2][120..128].copy_from_slice(&b);
        buf[3][120..128].copy_from_slice(&b);
        compressor.write_block(&buf[0], &buf[1], &buf[2], &buf[3]);
        compressor.write_block(&buf[0][64..], &buf[1][64..], &buf[2][64..], &buf[3][64..]);
    }

    unsafe { compressor.rev32() };
    compressor.dump()
}

// computes digest of four messages.
// The four messages are better whose difference of lengthes are not significant
pub fn sum4(m0: &[u8], m1: &[u8], m2: &[u8], m3: &[u8]) -> [[u8; 32]; 4] {
    let mut compressor = Compressor::new();
    // length must be equal.
    let l0 = m0.len();
    let l1 = m1.len();
    let l2 = m2.len();
    let l3 = m3.len();
    let ml = min(l0, min(l1, min(l2, l3)));

    // update the common
    let blocks = ml / 64;
    for i in 0..blocks {
        compressor.write_block(&m0[i * 64..], &m1[i * 64..], &m2[i * 64..], &m3[i * 64..]);
    }
    let mut p = [&m0[blocks * 64..], &m1[blocks * 64..], &m2[blocks * 64..], &m3[blocks * 64..]];

    // tail bytes to go
    loop {
        // TODO: use neon
        let mut mask = 0u32;
        for i in 0..4 {
            if p[i].len() >= 64 {
                mask |= 1 << i;
            }
        }
        if mask == 0 {
            break;
        }
        compressor.write_block_masked(p[0], p[1], p[2], p[3], mask);
        for i in 0..4 {
            p[i] = &p[i][64 * ((mask >> i) & 1) as usize..];
        }
    }

    // Now all four message has less than 64 bytes.

    // handle the tails
    let mut buf = [[0; 128]; 4];
    for i in 0..4 {
        buf[i][..p[i].len()].copy_from_slice(p[i]);
        buf[i][p[i].len()] = 0x80u8;
    }
    let n = [p[0].len() + 1, p[1].len() + 1, p[2].len() + 1, p[3].len() + 1];
    let total_len = [m0.len() * 8, m1.len() * 8, m2.len() * 8, m3.len() * 8];
    let mut mask = 0;
    for i in 0..4 {
        let b = total_len[i].to_be_bytes();
        if n[i] + 8 <= BLOCK_SIZE {
            buf[i][56..64].copy_from_slice(&b);
        } else {
            mask |= 1 << i;
            buf[i][120..128].copy_from_slice(&b);
        }
    }
    compressor.write_block(&buf[0], &buf[1], &buf[2], &buf[3]);
    compressor.write_block_masked(&buf[0][64..], &buf[1][64..], &buf[2][64..], &buf[3][64..], mask);

    compressor.dump()
}

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use rand::{rng, Rng};

    use crate::sm3;

    use super::*;
    extern crate test;

    #[test]
    fn test_sum() {
        let msg = "abc".as_bytes();
        let digests = sum_equal4(&msg, &msg, &msg, &msg);
        let expect: [u8; 32] = [
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10, 0xe4, 0xe2, 0x41, 0x67, 0xc4, 0x87, 0x5c,
            0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b, 0x8f, 0x4b, 0xa8, 0xe0,
        ];
        for i in 0..4 {
            for j in 0..32 {
                assert_eq!(digests[i][j], expect[j]);
            }
        }
    }

    #[test]
    fn test_sum4() {
        let msg = "abc".as_bytes();
        let digests = sum4(&msg, &msg, &msg, &msg);
        let expect: [u8; 32] = [
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10, 0xe4, 0xe2, 0x41, 0x67, 0xc4, 0x87, 0x5c,
            0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b, 0x8f, 0x4b, 0xa8, 0xe0,
        ];
        for i in 0..4 {
            for j in 0..32 {
                assert_eq!(digests[i][j], expect[j]);
            }
        }
    }

    #[test]
    fn test_sum_fuzz() {
        let mut rng = rng();
        let mut msg = [[0u8; 100]; 4];
        for i in 0..4 {
            rng.fill(&mut msg[i][..]);
        }

        let digests = sum_equal4(&msg[0], &msg[1], &msg[2], &msg[3]);
        for i in 0..4 {
            assert_eq!(digests[i], sm3!(&msg[i]));
        }
    }

    #[test]
    fn test_sum4_fuzz() {
        let mut rng = rng();
        const N: [usize; 4] = [1, 11, 120, 12];
        let mut msg = [vec![0u8; N[0]], vec![0u8; N[1]], vec![0u8; N[2]], vec![0u8; N[3]]];
        for i in 0..4 {
            rng.fill(&mut msg[i][..]);
        }

        let digests = sum4(&msg[0], &msg[1], &msg[2], &msg[3]);
        for i in 0..4 {
            assert_eq!(digests[i], sm3!(&msg[i]));
        }
    }

    // cargo test --release --package opengm_crypto --lib -- sm3_simd::aarch64::tests::test_bench --exact --show-output
    // 630 MBps vs 350 MBps
    #[test]
    fn test_bench() {
        extern crate std;
        use std::time::*;
        const TOTAL_BYTES: usize = 10 * 1024 * 1024;
        const COUNT: usize = 100;
        let msg = vec![
            vec![0u8; TOTAL_BYTES],
            vec![0u8; TOTAL_BYTES],
            vec![0u8; TOTAL_BYTES],
            vec![0u8; TOTAL_BYTES],
        ];
        let mut d = sum_equal4(&msg[0], &msg[1], &msg[2], &msg[3]);

        let start = Instant::now();
        for _ in 0..COUNT {
            test::black_box(d = sum_equal4(&msg[0], &msg[1], &msg[2], &msg[3]));
        }
        println!("{:?}", d);
        let d = (Instant::now() - start).as_micros() as f64 / 1000000.0;
        println!("{:.2} MB/s", TOTAL_BYTES as f64 * COUNT as f64 * 4.0 / 1024.0 / 1024.0 / d);
    }

    fn equal_u32x4(a: uint32x4_t, b: uint32x4_t) -> bool {
        let bufa = [0; 4];
        let bufb = [0; 4];
        unsafe { vst1q_u32((&bufa).as_ptr() as *mut u32, a) };
        unsafe { vst1q_u32((&bufb).as_ptr() as *mut u32, b) };
        bufa == bufb
    }

    #[test]
    fn test_compress_x4() {
        #[rustfmt::skip]
        let mut iv: [uint32x4_t; 8] = unsafe {
            [
                transmute([0x7380166fu32, 0x7380166f, 0x7380166f, 0x7380166f]),
                transmute([0x4914b2b9u32, 0x4914b2b9, 0x4914b2b9, 0x4914b2b9]),
                transmute([0x172442d7u32, 0x172442d7, 0x172442d7, 0x172442d7]),
                transmute([0xda8a0600u32, 0xda8a0600, 0xda8a0600, 0xda8a0600]),
                transmute([0xa96f30bcu32, 0xa96f30bc, 0xa96f30bc, 0xa96f30bc]),
                transmute([0x163138aau32, 0x163138aa, 0x163138aa, 0x163138aa]),
                transmute([0xe38dee4du32, 0xe38dee4d, 0xe38dee4d, 0xe38dee4d]),
                transmute([0xb0fb0e4eu32, 0xb0fb0e4e, 0xb0fb0e4e, 0xb0fb0e4e]),
            ]
        };
        let mut w: [uint32x4_t; 16] = unsafe { [transmute([0x01010101, 0x01010101, 0x01010101, 0x01010101]); 16] };
        let expect: [uint32x4_t; 8] = unsafe {
            [
                transmute([0xb9122804u32, 0xb9122804, 0xb9122804, 0xb9122804]),
                transmute([0xc515b3c2u32, 0xc515b3c2, 0xc515b3c2, 0xc515b3c2]),
                transmute([0xb34a42f1u32, 0xb34a42f1, 0xb34a42f1, 0xb34a42f1]),
                transmute([0x06edad4eu32, 0x06edad4e, 0x06edad4e, 0x06edad4e]),
                transmute([0x52ecd5c7u32, 0x52ecd5c7, 0x52ecd5c7, 0x52ecd5c7]),
                transmute([0x8545dd67u32, 0x8545dd67, 0x8545dd67, 0x8545dd67]),
                transmute([0xf42b4275u32, 0xf42b4275, 0xf42b4275, 0xf42b4275]),
                transmute([0x900ed3adu32, 0x900ed3ad, 0x900ed3ad, 0x900ed3ad]),
            ]
        };
        unsafe { unsafe_compress_aarch64_neon(&mut iv, &mut w, 0b1111) };
        for i in 0..8 {
            assert!(equal_u32x4(iv[i], expect[i]));
        }
    }

    use test::Bencher;
    #[bench]
    fn bench_compressx4(b: &mut Bencher) {
        #[rustfmt::skip]
        let mut iv: [uint32x4_t; 8] = unsafe {
            [
                transmute([0x7380166fu32, 0x7380166f, 0x7380166f, 0x7380166f]),
                transmute([0x4914b2b9u32, 0x4914b2b9, 0x4914b2b9, 0x4914b2b9]),
                transmute([0x172442d7u32, 0x172442d7, 0x172442d7, 0x172442d7]),
                transmute([0xda8a0600u32, 0xda8a0600, 0xda8a0600, 0xda8a0600]),
                transmute([0xa96f30bcu32, 0xa96f30bc, 0xa96f30bc, 0xa96f30bc]),
                transmute([0x163138aau32, 0x163138aa, 0x163138aa, 0x163138aa]),
                transmute([0xe38dee4du32, 0xe38dee4d, 0xe38dee4d, 0xe38dee4d]),
                transmute([0xb0fb0e4eu32, 0xb0fb0e4e, 0xb0fb0e4e, 0xb0fb0e4e]),
            ]
        };
        let mut w: [uint32x4_t; 16] = unsafe { [transmute([0x01010101, 0x01010101, 0x01010101, 0x01010101]); 16] };

        // 389.27 ns
        b.iter(|| {
            test::black_box(unsafe { unsafe_compress_aarch64_neon(&mut iv, &mut w, 0b1111) });
        });
    }


    #[test]
    fn test_load_message() {
        let p0 = (0..64).collect::<Vec<_>>();
        let p1 = (0..64).collect::<Vec<_>>();
        let p2 = (0..64).collect::<Vec<_>>();
        let p3 = (0..64).collect::<Vec<_>>();

        let w = unsafe { load_message(&p0, &p1, &p2, &p3, 0b1111) };
        println!("{:#08x?}", w);
    }
}
