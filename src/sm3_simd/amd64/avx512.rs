use crate::sm3::{BLOCK_SIZE, util::T};

#[cfg(target_arch = "x86")]
use core::arch::x86::*;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;
use core::{iter::zip, mem::transmute};


#[inline(always)]
fn ff0(x: __m512i, y: __m512i, z: __m512i) -> __m512i {
    // x^y^z
    unsafe{
        _mm512_xor_si512(_mm512_xor_si512(x,y),z)
    }

}

#[inline(always)]
fn gg0(x: __m512i, y: __m512i, z: __m512i) -> __m512i {
    ff0(x,y,z)
}

#[inline(always)]
fn ff1(x: __m512i, y: __m512i, z: __m512i) -> __m512i {
    // ((x | z) & y) | (x & z)
    unsafe { _mm512_or_si512(_mm512_and_si512(_mm512_or_si512(x, z), y), _mm512_and_si512(x, z)) }
}

#[inline(always)]
fn gg1(x: __m512i, y: __m512i, z: __m512i) -> __m512i {
    // z ^ (x & (y ^ z))
    unsafe { _mm512_xor_si512(_mm512_and_si512(_mm512_xor_si512(y, z), x), z) }
}

#[inline(always)]
fn p0(x: __m512i) -> __m512i {
    unsafe {
        let y = _mm512_rol_epi32(x, 9);
        let z = _mm512_rol_epi32(x, 17);
        _mm512_xor_si512(_mm512_xor_si512(x, y), z)
    }
}

#[inline(always)]
fn p1(x: __m512i) -> __m512i {
    // x ^ x.rotate_left(15) ^ x.rotate_left(23)
    unsafe {
        let y = _mm512_rol_epi32(x, 15);
        let z = _mm512_rol_epi32(x, 23);
        _mm512_xor_si512(_mm512_xor_si512(x, y), z)
    }
}



macro_rules! Round {
    ($i: expr, $w: expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff: expr, $gg: expr) => {{
            let x = _mm512_rol_epi32($a, 12);
            let ss1 = _mm512_add_epi32(_mm512_add_epi32(x, $e), _mm512_set1_epi32(T[$i] as i32));
            let ss1 = _mm512_rol_epi32(ss1, 7);
            let ss2 = _mm512_xor_si512(ss1, x);
            let tt1 = _mm512_add_epi32(_mm512_add_epi32(_mm512_add_epi32($ff($a, $b, $c), $d), ss2), _mm512_xor_si512($w[$i % 16], $w[($i + 4) % 16]));
            let tt2 = _mm512_add_epi32(_mm512_add_epi32(_mm512_add_epi32($gg($e, $f, $g), $h), ss1), $w[$i % 16]);
            $b = _mm512_rol_epi32($b, 9);
            $d = tt1;
            $f = _mm512_rol_epi32($f, 19);
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
unsafe fn msg_sched(w: &[__m512i], i: usize) -> __m512i {
    let t0 = _mm512_xor_si512(_mm512_xor_si512(w[(i + 0) % 16], w[(i + 7) % 16]), _mm512_rol_epi32(w[(i + 13) % 16], 15));
    let t1 = _mm512_xor_si512(_mm512_rol_epi32(w[(i + 3) % 16], 7), w[(i + 10) % 16]);
    _mm512_xor_si512(t1, p1(t0))
}

macro_rules! transpose {
    ($r0: expr, $r1: expr, $r2: expr, $r3: expr, $r4: expr, $r5: expr, $r6: expr, $r7: expr, $r8: expr, $r9: expr, $ra: expr, $rb: expr, $rc: expr, $rd: expr, $re: expr, $rf: expr) => {{
        let mut t0 = _mm512_unpacklo_epi32($r0,$r1); //   0  16   1  17   4  20   5  21   8  24   9  25  12  28  13  29 
        let mut t1 = _mm512_unpackhi_epi32($r0,$r1); //   2  18   3  19   6  22   7  23  10  26  11  27  14  30  15  31
        let mut t2 = _mm512_unpacklo_epi32($r2,$r3); //  32  48  33  49 ...
        let mut t3 = _mm512_unpackhi_epi32($r2,$r3); //  34  50  35  51 ...
        let mut t4 = _mm512_unpacklo_epi32($r4,$r5); //  64  80  65  81 ...  
        let mut t5 = _mm512_unpackhi_epi32($r4,$r5); //  66  82  67  83 ...
        let mut t6 = _mm512_unpacklo_epi32($r6,$r7); //  96 112  97 113 ...
        let mut t7 = _mm512_unpackhi_epi32($r6,$r7); //  98 114  99 115 ...
        let mut t8 = _mm512_unpacklo_epi32($r8,$r9); // 128 ...
        let mut t9 = _mm512_unpackhi_epi32($r8,$r9); // 130 ...
        let mut ta = _mm512_unpacklo_epi32($ra,$rb); // 160 ...
        let mut tb = _mm512_unpackhi_epi32($ra,$rb); // 162 ...
        let mut tc = _mm512_unpacklo_epi32($rc,$rd); // 196 ...
        let mut td = _mm512_unpackhi_epi32($rc,$rd); // 198 ...
        let mut te = _mm512_unpacklo_epi32($re,$rf); // 228 ...
        let mut tf = _mm512_unpackhi_epi32($re,$rf); // 230 ...

        $r0 = _mm512_unpacklo_epi64(t0,t2); //   0  16  32  48 ...
        $r1 = _mm512_unpackhi_epi64(t0,t2); //   1  17  33  49 ...
        $r2 = _mm512_unpacklo_epi64(t1,t3); //   2  18  34  49 ...
        $r3 = _mm512_unpackhi_epi64(t1,t3); //   3  19  35  51 ...
        $r4 = _mm512_unpacklo_epi64(t4,t6); //  64  80  96 112 ...  
        $r5 = _mm512_unpackhi_epi64(t4,t6); //  65  81  97 114 ...
        $r6 = _mm512_unpacklo_epi64(t5,t7); //  66  82  98 113 ...
        $r7 = _mm512_unpackhi_epi64(t5,t7); //  67  83  99 115 ...
        $r8 = _mm512_unpacklo_epi64(t8,ta); // 128 144 160 176 ...  
        $r9 = _mm512_unpackhi_epi64(t8,ta); // 129 145 161 178 ...
        $ra = _mm512_unpacklo_epi64(t9,tb); // 130 146 162 177 ... 
        $rb = _mm512_unpackhi_epi64(t9,tb); // 131 147 163 179 ...
        $rc = _mm512_unpacklo_epi64(tc,te); // 192 208 228 240 ... 
        $rd = _mm512_unpackhi_epi64(tc,te); // 193 209 229 241 ...
        $re = _mm512_unpacklo_epi64(td,tf); // 194 210 230 242 ...
        $rf = _mm512_unpackhi_epi64(td,tf); // 195 211 231 243 ...

        t0 = _mm512_shuffle_i32x4($r0, $r4, 0x88); //   0  16  32  48   8  24  40  56  64  80  96  112 ...
        t1 = _mm512_shuffle_i32x4($r1, $r5, 0x88); //   1  17  33  49 ...
        t2 = _mm512_shuffle_i32x4($r2, $r6, 0x88); //   2  18  34  50 ...
        t3 = _mm512_shuffle_i32x4($r3, $r7, 0x88); //   3  19  35  51 ...
        t4 = _mm512_shuffle_i32x4($r0, $r4, 0xdd); //   4  20  36  52 ...
        t5 = _mm512_shuffle_i32x4($r1, $r5, 0xdd); //   5  21  37  53 ...
        t6 = _mm512_shuffle_i32x4($r2, $r6, 0xdd); //   6  22  38  54 ...
        t7 = _mm512_shuffle_i32x4($r3, $r7, 0xdd); //   7  23  39  55 ...
        t8 = _mm512_shuffle_i32x4($r8, $rc, 0x88); // 128 144 160 176 ...
        t9 = _mm512_shuffle_i32x4($r9, $rd, 0x88); // 129 145 161 177 ...
        ta = _mm512_shuffle_i32x4($ra, $re, 0x88); // 130 146 162 178 ...
        tb = _mm512_shuffle_i32x4($rb, $rf, 0x88); // 131 147 163 179 ...
        tc = _mm512_shuffle_i32x4($r8, $rc, 0xdd); // 132 148 164 180 ...
        td = _mm512_shuffle_i32x4($r9, $rd, 0xdd); // 133 149 165 181 ...
        te = _mm512_shuffle_i32x4($ra, $re, 0xdd); // 134 150 166 182 ...
        tf = _mm512_shuffle_i32x4($rb, $rf, 0xdd); // 135 151 167 183 ...

        $r0 = _mm512_shuffle_i32x4(t0, t8, 0x88); //   0  16  32  48  64  80  96 112 ... 240
        $r1 = _mm512_shuffle_i32x4(t1, t9, 0x88); //   1  17  33  49  66  81  97 113 ... 241
        $r2 = _mm512_shuffle_i32x4(t2, ta, 0x88); //   2  18  34  50  67  82  98 114 ... 242
        $r3 = _mm512_shuffle_i32x4(t3, tb, 0x88); //   3  19  35  51  68  83  99 115 ... 243
        $r4 = _mm512_shuffle_i32x4(t4, tc, 0x88); //   4 ...
        $r5 = _mm512_shuffle_i32x4(t5, td, 0x88); //   5 ...
        $r6 = _mm512_shuffle_i32x4(t6, te, 0x88); //   6 ...
        $r7 = _mm512_shuffle_i32x4(t7, tf, 0x88); //   7 ...
        $r8 = _mm512_shuffle_i32x4(t0, t8, 0xdd); //   8 ...
        $r9 = _mm512_shuffle_i32x4(t1, t9, 0xdd); //   9 ...
        $ra = _mm512_shuffle_i32x4(t2, ta, 0xdd); //  10 ...
        $rb = _mm512_shuffle_i32x4(t3, tb, 0xdd); //  11 ...
        $rc = _mm512_shuffle_i32x4(t4, tc, 0xdd); //  12 ...
        $rd = _mm512_shuffle_i32x4(t5, td, 0xdd); //  13 ...
        $re = _mm512_shuffle_i32x4(t6, te, 0xdd); //  14 ...
        $rf = _mm512_shuffle_i32x4(t7, tf, 0xdd); //  15  31  47  63  79  96 111 127 ... 255
    }};
}

// 调整端序
// t0 = _mm_shuffle_epi8(t0, flp);
// 将t0中保存的4个32比特的整数转换端序
const FLIP32: __m512i = unsafe {
    transmute([
        0x0405060700010203u64, 0x0C0D0E0F08090A0B, 
        0x1415161710111213, 0x1C1D1E1F18191A1B, 
        0x2425262720212223, 0x2C2D2E2F28292A2B, 
        0x3435363730313233, 0x3C3D3E3F38393A3B
    ])
};


// compress one block for each pi.
// #[inline(always)]
#[target_feature(enable = "ssse3", enable = "sse2", enable = "avx512f", enable = "avx512vl", enable = "avx512bw")]
pub unsafe fn load_message(m: &[&[u8];16]) -> [__m512i; 16] {
    unsafe {
        let mut w: [__m512i; 16] = [transmute([0u32; 16]); 16];
        
        // load messages to w[0..16]
        for (u, v) in zip(&mut w,m) {
            *u = _mm512_shuffle_epi8(_mm512_loadu_epi32(v.as_ptr() as *const i32), FLIP32);
        }
        transpose!(w[0],w[1],w[2],w[3],w[4],w[5],w[6],w[7],w[8],w[9],w[10],w[11],w[12],w[13],w[14],w[15]);
        w
    }
}


// compress one block for each pi.
#[inline(always)]
pub fn compress(iv: &mut [__m512i; 8], m: &[&[u8];16]) {
    unsafe {
        let mut w = load_message(&m);
        unsafe_compress(iv, &mut w);
    }
}

// w[0..15], 4 lane, each lane for a message.
#[target_feature(enable = "ssse3", enable = "sse2", enable = "avx512f", enable = "avx512vl", enable = "avx512bw")]
unsafe fn unsafe_compress(iv: &mut [__m512i; 8], w: &mut [__m512i;16]) {
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

        iv[0] = _mm512_xor_si512(iv[0],  a);
        iv[1] = _mm512_xor_si512(iv[1],  b);
        iv[2] = _mm512_xor_si512(iv[2],  c);
        iv[3] = _mm512_xor_si512(iv[3],  d);
        iv[4] = _mm512_xor_si512(iv[4],  e);
        iv[5] = _mm512_xor_si512(iv[5],  f);
        iv[6] = _mm512_xor_si512(iv[6],  g);
        iv[7] = _mm512_xor_si512(iv[7],  h);
    }
}


pub fn new() -> Compressor {
    Compressor::new()
}

// For Neon, the Digest can update four messages.
#[derive(Debug, Copy, Clone)]
pub struct Compressor([__m512i; 8]);

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
                _mm512_set1_epi32(0x7380166fu32 as i32), // iv0
                _mm512_set1_epi32(0x4914b2b9u32 as i32), // iv1
                _mm512_set1_epi32(0x172442d7u32 as i32), // iv2
                _mm512_set1_epi32(0xda8a0600u32 as i32), // iv3,
                _mm512_set1_epi32(0xa96f30bcu32 as i32), // iv4
                _mm512_set1_epi32(0x163138aau32 as i32), // iv5
                _mm512_set1_epi32(0xe38dee4du32 as i32), // iv6
                _mm512_set1_epi32(0xb0fb0e4eu32 as i32), // iv7
            ])
        }
    }

    // update one block
    #[inline]
    fn write_block(&mut self, p: &[&[u8];16]) {
        compress(&mut self.0, p);
    }

    // #[inline]
    // fn write_block_masked(&mut self, p0: &[u8], p1: &[u8], p2: &[u8], p3: &[u8], mask: u32) {
    //     compress_block_aarch64_neon(&mut self.0, p0, p1, p2, p3, mask);
    // }

    // reverse each u32's endian
    fn rev32(&mut self) {
        for x in &mut self.0 {
            *x = unsafe { _mm512_shuffle_epi8(*x, FLIP32) };
        }
    }

    fn dump(&self) -> [[u8; 32]; 16] {
        unsafe {
            let mut d = [[0; 32]; 16];
            let mut buf = [[0u32;16];8];
            for i in 0..8 {
                _mm512_store_epi32((&mut buf[i]) as *mut u32 as *mut i32, self.0[i]);
            }
            for i in 0..16{
                for j in 0..8{
                    d[i][4*j..4*j+4].copy_from_slice(&buf[j][i].to_be_bytes())
                }
            }
            d
        }
    }
}


// computes digest of four messages with equal length.
pub fn sum_equal16(m: &[&[u8];16]) -> [[u8; 32]; 16] {
    let mut compressor = Compressor::new();
    // length must be equal.
    let length = m[0].len();

    let blocks = length / 64;
    for _i in 0..blocks {
        // compressor.write_block(&[
        //     &m[0][i * 64..], &m[1][i * 64..], &m[2][i * 64..], &m[3][i * 64..],
        //     &m[4][i * 64..], &m[5][i * 64..], &m[6][i * 64..], &m[7][i * 64..],
        //     &m[8][i * 64..], &m[9][i * 64..], &m[10][i * 64..], &m[11][i * 64..],
        //     &m[12][i * 64..], &m[13][i * 64..], &m[14][i * 64..], &m[15][i * 64..],
        // ]);
        compressor.write_block(&[
            &m[0], &m[1], &m[2], &m[3],
            &m[4], &m[5], &m[6], &m[7],
            &m[8], &m[9], &m[10], &m[11],
            &m[12], &m[13], &m[14], &m[15],
        ]);
    }

    // handle the tails
    let mut buf = [[0; 128]; 16];
    let total_len = m[0].len() as u64 * 8;
    let mut n = m[0].len() - blocks * 64; // there have tail_len bytes to go.
    for i in 0..16{
        buf[i][..n].copy_from_slice(&m[i][blocks * 64..]);
        buf[i][n] = 0x80u8;
    }

    n += 1;
    let b = total_len.to_be_bytes();
    if n + 8 <= BLOCK_SIZE {
        for i in 0..16{
            buf[i][56..64].copy_from_slice(&b);
        }
        compressor.write_block(&[
            &buf[0], &buf[1], &buf[2], &buf[3],
            &buf[4], &buf[5], &buf[6], &buf[7],
            &buf[8], &buf[9], &buf[10], &buf[11],
            &buf[12], &buf[13], &buf[14], &buf[15],
        ]);
    } else {
        for i in 0..16{
            buf[i][120..128].copy_from_slice(&b);
        }
        compressor.write_block(&[
            &buf[0], &buf[1], &buf[2], &buf[3],
            &buf[4], &buf[5], &buf[6], &buf[7],
            &buf[8], &buf[9], &buf[10], &buf[11],
            &buf[12], &buf[13], &buf[14], &buf[15],
        ]);

        compressor.write_block(&[
            &buf[0][64..], &buf[1][64..], &buf[2][64..], &buf[3][64..],
            &buf[4][64..], &buf[5][64..], &buf[6][64..], &buf[7][64..],
            &buf[8][64..], &buf[9][64..], &buf[10][64..], &buf[11][64..],
            &buf[12][64..], &buf[13][64..], &buf[14][64..], &buf[15][64..],
        ]);
    }

    // compressor.rev32();
    compressor.dump()
}

#[cfg(test)]
mod tests {
    
    use super::*;
    extern crate test;

    fn equal_mm512i(a: __m512i, b: __m512i) -> bool {
        let bufa = [0; 16];
        let bufb = [0; 16];
        unsafe { _mm512_store_epi32((&bufa).as_ptr() as *mut i32, a) };
        unsafe { _mm512_store_epi32((&bufb).as_ptr() as *mut i32, b) };
        bufa == bufb
    }
    #[test]
    fn test_sum16() {
        let msg = "abc".as_bytes();
        let digests = sum_equal16(&[
            &msg, &msg, &msg, &msg,
            &msg, &msg, &msg, &msg,
            &msg, &msg, &msg, &msg,
            &msg, &msg, &msg, &msg,
        ]);

        let expect: [u8; 32] = [
            0x66, 0xc7, 0xf0, 0xf4, 0x62, 0xee, 0xed, 0xd9, 0xd1, 0xf2, 0xd4, 0x6b, 0xdc, 0x10, 0xe4, 0xe2, 0x41, 0x67, 0xc4, 0x87, 0x5c,
            0xf2, 0xf7, 0xa2, 0x29, 0x7d, 0xa0, 0x2b, 0x8f, 0x4b, 0xa8, 0xe0,
        ];
        for i in 0..16 {
            for j in 0..32 {
                assert_eq!(digests[i][j], expect[j]);
            }
        }
    }


    #[test]
    fn test_compress_x4() {
        #[rustfmt::skip]
        let mut iv: [__m512i; 8] = unsafe {
            [
                _mm512_set1_epi32(0x7380166fu32 as i32),
                _mm512_set1_epi32(0x4914b2b9u32 as i32),
                _mm512_set1_epi32(0x172442d7u32 as i32),
                _mm512_set1_epi32(0xda8a0600u32 as i32),
                _mm512_set1_epi32(0xa96f30bcu32 as i32),
                _mm512_set1_epi32(0x163138aau32 as i32),
                _mm512_set1_epi32(0xe38dee4du32 as i32),
                _mm512_set1_epi32(0xb0fb0e4eu32 as i32),
            ]
        };
        let mut w: [__m512i; 16] = unsafe { [_mm512_set1_epi32(0x01010101); 16] };
        let expect: [__m512i; 8] = unsafe {
            [
                _mm512_set1_epi32(0xb9122804u32 as i32),
                _mm512_set1_epi32(0xc515b3c2u32 as i32),
                _mm512_set1_epi32(0xb34a42f1u32 as i32),
                _mm512_set1_epi32(0x06edad4eu32 as i32),
                _mm512_set1_epi32(0x52ecd5c7u32 as i32),
                _mm512_set1_epi32(0x8545dd67u32 as i32),
                _mm512_set1_epi32(0xf42b4275u32 as i32),
                _mm512_set1_epi32(0x900ed3adu32 as i32),
            ]
        };
        unsafe { unsafe_compress(&mut iv, &mut w)};
        for i in 0..8 {
            equal_mm512i(iv[i], expect[i]);
        }
    }


    // cargo test --release --package opengm_crypto --lib -- sm3_simd::amd64::avx512::tests::test_bench --exact --show-output
    // 4446.67 MBps
    #[test]
    fn test_bench() {
        extern crate std;
        use std::time::*;
        const TOTAL_BYTES: usize = 10*1024*1024;
        const COUNT: usize = 100;
        let msg = vec![vec![0u8; TOTAL_BYTES];16];
        let msg16 = &[
            msg[0].as_slice(), &msg[1], &msg[2], &msg[3],
            msg[4].as_slice(), &msg[5], &msg[6], &msg[7],
            msg[8].as_slice(), &msg[9], &msg[10], &msg[11],
            msg[12].as_slice(), &msg[13], &msg[14], &msg[15],
        ];

        let start = Instant::now();
        for _ in 0..COUNT {
            test::black_box(sum_equal16(msg16));
        }

        let d = (Instant::now() - start).as_micros() as f64 / 1000000.0;
        println!("{:.2} MBps", TOTAL_BYTES as f64 * COUNT as f64 * 16.0 / 1024.0 / 1024.0 / d);
    }


    use test::Bencher;
    #[bench]
    fn bench_compress(b: &mut Bencher) {
        #[rustfmt::skip]
        let mut iv: [__m512i; 8] = unsafe {
            [
                _mm512_set1_epi32(0x7380166fu32 as i32),
                _mm512_set1_epi32(0x4914b2b9u32 as i32),
                _mm512_set1_epi32(0x172442d7u32 as i32),
                _mm512_set1_epi32(0xda8a0600u32 as i32),
                _mm512_set1_epi32(0xa96f30bcu32 as i32),
                _mm512_set1_epi32(0x163138aau32 as i32),
                _mm512_set1_epi32(0xe38dee4du32 as i32),
                _mm512_set1_epi32(0xb0fb0e4eu32 as i32),
            ]
        };
        let mut w: [__m512i; 16] = unsafe { [_mm512_set1_epi32(0x01010101); 16] };

        // 185.27 ns
        b.iter(|| {
            test::black_box(unsafe { unsafe_compress(&mut iv, &mut w) });
        });
    }

}