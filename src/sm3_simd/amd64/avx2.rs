use crate::sm3::{util::T, BLOCK_SIZE};

#[cfg(target_arch = "x86")]
use core::arch::x86::*;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;
use core::{iter::zip, mem::transmute};

macro_rules! _mm256_rol_epi32 {
    ($a:expr, $n:literal) => {{
        _mm256_xor_si256(_mm256_slli_epi32($a, $n), _mm256_srli_epi32($a, 32 - $n))
    }};
}

// AVX512_VBMI2 for _mm256_shldi_epi32
#[inline(always)]
fn ff0(x: __m256i, y: __m256i, z: __m256i) -> __m256i {
    // x^y^z
    unsafe { _mm256_xor_si256(_mm256_xor_si256(x, y), z) }
}

#[inline(always)]
fn gg0(x: __m256i, y: __m256i, z: __m256i) -> __m256i {
    ff0(x, y, z)
}

#[inline(always)]
fn ff1(x: __m256i, y: __m256i, z: __m256i) -> __m256i {
    // ((x | z) & y) | (x & z)
    unsafe {
        _mm256_or_si256(
            _mm256_and_si256(_mm256_or_si256(x, z), y),
            _mm256_and_si256(x, z),
        )
    }
}

#[inline(always)]
fn gg1(x: __m256i, y: __m256i, z: __m256i) -> __m256i {
    // z ^ (x & (y ^ z))
    unsafe { _mm256_xor_si256(_mm256_and_si256(_mm256_xor_si256(y, z), x), z) }
}

#[inline(always)]
fn p0(x: __m256i) -> __m256i {
    unsafe {
        let y = _mm256_rol_epi32!(x, 9);
        let z = _mm256_rol_epi32!(x, 17);
        _mm256_xor_si256(_mm256_xor_si256(x, y), z)
    }
}

#[inline(always)]
fn p1(x: __m256i) -> __m256i {
    // x ^ x.rotate_left(15) ^ x.rotate_left(23)
    unsafe {
        let y = _mm256_rol_epi32!(x, 15);
        let z = _mm256_rol_epi32!(x, 23);
        _mm256_xor_si256(_mm256_xor_si256(x, y), z)
    }
}

macro_rules! Round {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {{
        let x = _mm256_rol_epi32!($a, 12);
        let ss1 = _mm256_add_epi32(_mm256_add_epi32(x, $e), _mm256_set1_epi32(T[$i] as i32));
        let ss1 = _mm256_rol_epi32!(ss1, 7);
        let ss2 = _mm256_xor_si256(ss1, x);
        let tt1 = _mm256_add_epi32(
            _mm256_add_epi32(_mm256_add_epi32($ff($a, $b, $c), $d), ss2),
            _mm256_xor_si256($w[$i % 16], $w[($i + 4) % 16]),
        );
        let tt2 = _mm256_add_epi32(
            _mm256_add_epi32(_mm256_add_epi32($gg($e, $f, $g), $h), ss1),
            $w[$i % 16],
        );
        $b = _mm256_rol_epi32!($b, 9);
        $d = tt1;
        $f = _mm256_rol_epi32!($f, 19);
        $h = p0(tt2);
    }};
}

macro_rules! RoundWithMsgSche {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {
        Round!($i, $w, $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        $w[$i % 16] = msg_sched($w, $i);
    };
}

macro_rules! Round4 {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {{
        Round!($i + 0, $w, $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        Round!($i + 1, $w, $d, $a, $b, $c, $h, $e, $f, $g, $ff, $gg);
        Round!($i + 2, $w, $c, $d, $a, $b, $g, $h, $e, $f, $ff, $gg);
        Round!($i + 3, $w, $b, $c, $d, $a, $f, $g, $h, $e, $ff, $gg);
    }};
}

macro_rules! Round4WithMsgSche {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {{
        RoundWithMsgSche!($i + 0, $w, $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        RoundWithMsgSche!($i + 1, $w, $d, $a, $b, $c, $h, $e, $f, $g, $ff, $gg);
        RoundWithMsgSche!($i + 2, $w, $c, $d, $a, $b, $g, $h, $e, $f, $ff, $gg);
        RoundWithMsgSche!($i + 3, $w, $b, $c, $d, $a, $f, $g, $h, $e, $ff, $gg);
    }};
}

// p1(w0 ^ w7 ^ w13.rotate_left(15)) ^ w3.rotate_left(7) ^ w10
#[inline(always)]
unsafe fn msg_sched(w: &[__m256i], i: usize) -> __m256i {
    let t0 = _mm256_xor_si256(
        _mm256_xor_si256(w[(i + 0) % 16], w[(i + 7) % 16]),
        _mm256_rol_epi32!(w[(i + 13) % 16], 15),
    );
    let t1 = _mm256_xor_si256(_mm256_rol_epi32!(w[(i + 3) % 16], 7), w[(i + 10) % 16]);
    _mm256_xor_si256(t1, p1(t0))
}

macro_rules! transpose {
    ($r0: expr, $r1: expr, $r2: expr, $r3: expr, $r4: expr, $r5: expr, $r6: expr, $r7: expr) => {{
        let t0 = _mm256_unpacklo_epi32($r0, $r1);
        let t1 = _mm256_unpackhi_epi32($r0, $r1);
        let t2 = _mm256_unpacklo_epi32($r2, $r3);
        let t3 = _mm256_unpackhi_epi32($r2, $r3);
        let t4 = _mm256_unpacklo_epi32($r4, $r5);
        let t5 = _mm256_unpackhi_epi32($r4, $r5);
        let t6 = _mm256_unpacklo_epi32($r6, $r7);
        let t7 = _mm256_unpackhi_epi32($r6, $r7);
        let tt0 = _mm256_shuffle_ps(transmute(t0), transmute(t2), _MM_SHUFFLE(1, 0, 1, 0));
        let tt1 = _mm256_shuffle_ps(transmute(t0), transmute(t2), _MM_SHUFFLE(3, 2, 3, 2));
        let tt2 = _mm256_shuffle_ps(transmute(t1), transmute(t3), _MM_SHUFFLE(1, 0, 1, 0));
        let tt3 = _mm256_shuffle_ps(transmute(t1), transmute(t3), _MM_SHUFFLE(3, 2, 3, 2));
        let tt4 = _mm256_shuffle_ps(transmute(t4), transmute(t6), _MM_SHUFFLE(1, 0, 1, 0));
        let tt5 = _mm256_shuffle_ps(transmute(t4), transmute(t6), _MM_SHUFFLE(3, 2, 3, 2));
        let tt6 = _mm256_shuffle_ps(transmute(t5), transmute(t7), _MM_SHUFFLE(1, 0, 1, 0));
        let tt7 = _mm256_shuffle_ps(transmute(t5), transmute(t7), _MM_SHUFFLE(3, 2, 3, 2));
        $r0 = transmute(_mm256_permute2f128_ps(tt0, tt4, 0x20));
        $r1 = transmute(_mm256_permute2f128_ps(tt1, tt5, 0x20));
        $r2 = transmute(_mm256_permute2f128_ps(tt2, tt6, 0x20));
        $r3 = transmute(_mm256_permute2f128_ps(tt3, tt7, 0x20));
        $r4 = transmute(_mm256_permute2f128_ps(tt0, tt4, 0x31));
        $r5 = transmute(_mm256_permute2f128_ps(tt1, tt5, 0x31));
        $r6 = transmute(_mm256_permute2f128_ps(tt2, tt6, 0x31));
        $r7 = transmute(_mm256_permute2f128_ps(tt3, tt7, 0x31));
    }};
}

// 调整端序
// t0 = _mm_shuffle_epi8(t0, flp);
// 将t0中保存的4个32比特的整数转换端序
const FLIP32: __m256i = unsafe {
    transmute([
        0x0405060700010203u64,
        0x0C0D0E0F08090A0B,
        0x1415161710111213,
        0x1C1D1E1F18191A1B,
    ])
};

// compress one block for each pi.
// #[inline(always)]
#[target_feature(enable = "avx2", enable = "avx")]
pub unsafe fn load_message(m: &[&[u8]; 8]) -> [__m256i; 16] {
    unsafe {
        let mut w: [__m256i; 16] = [transmute([0u32; 8]); 16];

        // load messages to w[0..16]
        for i in 0..8 {
            w[i] = _mm256_shuffle_epi8(_mm256_loadu_epi32(m[i].as_ptr() as *const i32), FLIP32);
            w[8 + i] = _mm256_shuffle_epi8(
                _mm256_loadu_epi32(m[i].as_ptr().offset(32) as *const i32),
                FLIP32,
            );
        }
        transpose!(w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]);
        transpose!(w[8], w[9], w[10], w[11], w[12], w[13], w[14], w[15]);
        w
    }
}

// compress one block for each pi.
#[inline(always)]
pub fn compress(iv: &mut [__m256i; 8], m: &[&[u8]; 8]) {
    unsafe {
        let mut w = load_message(&m);
        unsafe_compress(iv, &mut w);
    }
}

// w[0..15], 4 lane, each lane for a message.
#[target_feature(enable = "avx", enable = "avx2")]
unsafe fn unsafe_compress(iv: &mut [__m256i; 8], w: &mut [__m256i; 16]) {
    unsafe {
        let mut a = iv[0];
        let mut b = iv[1];
        let mut c = iv[2];
        let mut d = iv[3];
        let mut e = iv[4];
        let mut f = iv[5];
        let mut g = iv[6];
        let mut h = iv[7];

        Round4WithMsgSche!(0, w, a, b, c, d, e, f, g, h, ff0, gg0);
        Round4WithMsgSche!(4, w, a, b, c, d, e, f, g, h, ff0, gg0);
        Round4WithMsgSche!(8, w, a, b, c, d, e, f, g, h, ff0, gg0);
        Round4WithMsgSche!(12, w, a, b, c, d, e, f, g, h, ff0, gg0);

        Round4WithMsgSche!(16, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgSche!(20, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgSche!(24, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgSche!(28, w, a, b, c, d, e, f, g, h, ff1, gg1);

        Round4WithMsgSche!(32, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgSche!(36, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgSche!(40, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4WithMsgSche!(44, w, a, b, c, d, e, f, g, h, ff1, gg1);

        Round4WithMsgSche!(48, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4!(52, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4!(56, w, a, b, c, d, e, f, g, h, ff1, gg1);
        Round4!(60, w, a, b, c, d, e, f, g, h, ff1, gg1);

        iv[0] = _mm256_xor_si256(iv[0], a);
        iv[1] = _mm256_xor_si256(iv[1], b);
        iv[2] = _mm256_xor_si256(iv[2], c);
        iv[3] = _mm256_xor_si256(iv[3], d);
        iv[4] = _mm256_xor_si256(iv[4], e);
        iv[5] = _mm256_xor_si256(iv[5], f);
        iv[6] = _mm256_xor_si256(iv[6], g);
        iv[7] = _mm256_xor_si256(iv[7], h);
    }
}

pub fn new() -> Compressor {
    Compressor::new()
}

// For Neon, the Digest can update four messages.
#[derive(Debug, Copy, Clone)]
pub struct Compressor([__m256i; 8]);

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
                _mm256_set1_epi32(0x7380166fu32 as i32), // iv0
                _mm256_set1_epi32(0x4914b2b9u32 as i32), // iv1
                _mm256_set1_epi32(0x172442d7u32 as i32), // iv2
                _mm256_set1_epi32(0xda8a0600u32 as i32), // iv3,
                _mm256_set1_epi32(0xa96f30bcu32 as i32), // iv4
                _mm256_set1_epi32(0x163138aau32 as i32), // iv5
                _mm256_set1_epi32(0xe38dee4du32 as i32), // iv6
                _mm256_set1_epi32(0xb0fb0e4eu32 as i32), // iv7
            ])
        }
    }

    // update one block
    #[inline]
    fn write_block(&mut self, p: &[&[u8]; 8]) {
        compress(&mut self.0, p);
    }

    // #[inline]
    // fn write_block_masked(&mut self, p0: &[u8], p1: &[u8], p2: &[u8], p3: &[u8], mask: u32) {
    //     compress_block_aarch64_neon(&mut self.0, p0, p1, p2, p3, mask);
    // }

    // reverse each u32's endian
    fn rev32(&mut self) {
        for x in &mut self.0 {
            *x = unsafe { _mm256_shuffle_epi8(*x, FLIP32) };
        }
    }

    fn dump(&self) -> [[u8; 32]; 8] {
        unsafe {
            let mut d = [[0; 32]; 8];
            let mut buf = [[0u32; 16]; 8];
            for i in 0..8 {
                _mm256_store_epi32((&mut buf[i]) as *mut u32 as *mut i32, self.0[i]);
            }
            for i in 0..8 {
                for j in 0..8 {
                    d[i][4 * j..4 * j + 4].copy_from_slice(&buf[j][i].to_be_bytes())
                }
            }
            d
        }
    }
}

// computes digest of four messages with equal length.
pub fn sum_equal8(m: &[&[u8]; 8]) -> [[u8; 32]; 8] {
    let mut compressor = Compressor::new();
    // length must be equal.
    let length = m[0].len();

    let blocks = length / 64;
    for _i in 0..blocks {
        compressor.write_block(&[&m[0], &m[1], &m[2], &m[3], &m[4], &m[5], &m[6], &m[7]]);
    }

    // handle the tails
    let mut buf = [[0; 128]; 8];
    let total_len = m[0].len() as u64 * 8;
    let mut n = m[0].len() - blocks * 64; // there have tail_len bytes to go.
    for i in 0..8 {
        buf[i][..n].copy_from_slice(&m[i][blocks * 64..]);
        buf[i][n] = 0x80u8;
    }

    n += 1;
    let b = total_len.to_be_bytes();
    if n + 8 <= BLOCK_SIZE {
        for i in 0..8 {
            buf[i][56..64].copy_from_slice(&b);
        }
        compressor.write_block(&[
            &buf[0], &buf[1], &buf[2], &buf[3], &buf[4], &buf[5], &buf[6], &buf[7],
        ]);
    } else {
        for i in 0..8 {
            buf[i][120..128].copy_from_slice(&b);
        }
        compressor.write_block(&[
            &buf[0], &buf[1], &buf[2], &buf[3], &buf[4], &buf[5], &buf[6], &buf[7],
        ]);

        compressor.write_block(&[
            &buf[0][64..],
            &buf[1][64..],
            &buf[2][64..],
            &buf[3][64..],
            &buf[4][64..],
            &buf[5][64..],
            &buf[6][64..],
            &buf[7][64..],
        ]);
    }

    // compressor.rev32();
    compressor.dump()
}

#[cfg(test)]
mod tests {

    use super::*;
    extern crate test;

    fn equal_mm256i(a: __m256i, b: __m256i) -> bool {
        let bufa = [0; 8];
        let bufb = [0; 8];
        unsafe { _mm256_store_epi32((&bufa).as_ptr() as *mut i32, a) };
        unsafe { _mm256_store_epi32((&bufb).as_ptr() as *mut i32, b) };
        bufa == bufb
    }

    #[test]
    fn test_sum8() {
        let msg = [0u8;100];
        let digests = sum_equal8(&[&msg, &msg, &msg, &msg, &msg, &msg, &msg, &msg]);

        let expect: [u8; 32] = sm3!(&msg);
        for i in 0..8 {
            assert_eq!(digests[i], expect);
        }
    }

    #[test]
    fn test_compress_x4() {
        #[rustfmt::skip]
        let mut iv: [__m256i; 8] = unsafe {
            [
                _mm256_set1_epi32(0x7380166fu32 as i32),
                _mm256_set1_epi32(0x4914b2b9u32 as i32),
                _mm256_set1_epi32(0x172442d7u32 as i32),
                _mm256_set1_epi32(0xda8a0600u32 as i32),
                _mm256_set1_epi32(0xa96f30bcu32 as i32),
                _mm256_set1_epi32(0x163138aau32 as i32),
                _mm256_set1_epi32(0xe38dee4du32 as i32),
                _mm256_set1_epi32(0xb0fb0e4eu32 as i32),
            ]
        };
        let mut w: [__m256i; 16] = unsafe { [_mm256_set1_epi32(0x01010101); 16] };
        let expect: [__m256i; 8] = unsafe {
            [
                _mm256_set1_epi32(0xb9122804u32 as i32),
                _mm256_set1_epi32(0xc515b3c2u32 as i32),
                _mm256_set1_epi32(0xb34a42f1u32 as i32),
                _mm256_set1_epi32(0x06edad4eu32 as i32),
                _mm256_set1_epi32(0x52ecd5c7u32 as i32),
                _mm256_set1_epi32(0x8545dd67u32 as i32),
                _mm256_set1_epi32(0xf42b4275u32 as i32),
                _mm256_set1_epi32(0x900ed3adu32 as i32),
            ]
        };
        unsafe { unsafe_compress(&mut iv, &mut w) };
        for i in 0..8 {
            equal_mm256i(iv[i], expect[i]);
        }
    }

    // cargo test --release --package opengm_crypto --lib -- sm3_simd::amd64::avx2::tests::test_bench --exact --show-output
    // 1756.78 MBps
    #[test]
    fn test_bench() {
        extern crate std;
        use std::time::*;
        const TOTAL_BYTES: usize = 10 * 1024 * 1024;
        const COUNT: usize = 100;
        let msg = vec![vec![0u8; TOTAL_BYTES]; 8];
        let msg8 = &[
            msg[0].as_slice(),
            &msg[1],
            &msg[2],
            &msg[3],
            msg[4].as_slice(),
            &msg[5],
            &msg[6],
            &msg[7],
        ];

        let start = Instant::now();
        for _ in 0..COUNT {
            test::black_box(sum_equal8(msg8));
        }

        let d = (Instant::now() - start).as_micros() as f64 / 1000000.0;
        println!(
            "{:.2} MBps",
            TOTAL_BYTES as f64 * COUNT as f64 * 8.0 / 1024.0 / 1024.0 / d
        );
    }

    use test::Bencher;
    #[bench]
    fn bench_compress(b: &mut Bencher) {
        #[rustfmt::skip]
        let mut iv: [__m256i; 8] = unsafe {
            [
                _mm256_set1_epi32(0x7380166fu32 as i32),
                _mm256_set1_epi32(0x4914b2b9u32 as i32),
                _mm256_set1_epi32(0x172442d7u32 as i32),
                _mm256_set1_epi32(0xda8a0600u32 as i32),
                _mm256_set1_epi32(0xa96f30bcu32 as i32),
                _mm256_set1_epi32(0x163138aau32 as i32),
                _mm256_set1_epi32(0xe38dee4du32 as i32),
                _mm256_set1_epi32(0xb0fb0e4eu32 as i32),
            ]
        };
        let mut w: [__m256i; 16] = unsafe { [_mm256_set1_epi32(0x01010101); 16] };

        // 185.27 ns
        b.iter(|| {
            test::black_box(unsafe { unsafe_compress(&mut iv, &mut w) });
        });
    }
}
