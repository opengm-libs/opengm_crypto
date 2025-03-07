use super::{util::*, BLOCK_SIZE};
use core::iter::zip;

#[inline(always)]
fn sched_w(w0: u32, w7: u32, w13: u32, w3: u32, w10: u32) -> u32 {
    p1(w0 ^ w7 ^ w13.rotate_left(15)) ^ w3.rotate_left(7) ^ w10
}

macro_rules! round {
    ($i:expr, $w:expr, $ww:expr, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident,$ff:ident, $gg: ident) => {
        let x = $a.rotate_left(12);
        let ss1 = x.wrapping_add($e).wrapping_add(T[$i]);
        let ss1 = ss1.rotate_left(7);
        let ss2 = ss1 ^ x;
        let tt1 = $ff($a, $b, $c).wrapping_add($d).wrapping_add(ss2).wrapping_add($ww);
        let tt2 = $gg($e, $f, $g).wrapping_add($h).wrapping_add(ss1).wrapping_add($w);
        $b = $b.rotate_left(9);
        $d = tt1;
        $f = $f.rotate_left(19);
        $h = p0(tt2);
    };
}

macro_rules! round_with_msg_sched {
    ($i:expr,$w:ident, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff:ident, $gg: ident) => {
        round!($i, $w[$i % 16], $w[$i % 16] ^ $w[($i + 4) % 16], $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        $w[$i % 16] = sched_w($w[($i) % 16], $w[($i + 7) % 16], $w[($i + 13) % 16], $w[($i + 3) % 16], $w[($i + 10) % 16]);
    };
}

macro_rules! round4_with_msg_sched {
    ($i:expr,$w:ident, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff:ident, $gg: ident) => {
        round_with_msg_sched!($i, $w, $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        round_with_msg_sched!($i + 1, $w, $d, $a, $b, $c, $h, $e, $f, $g, $ff, $gg);
        round_with_msg_sched!($i + 2, $w, $c, $d, $a, $b, $g, $h, $e, $f, $ff, $gg);
        round_with_msg_sched!($i + 3, $w, $b, $c, $d, $a, $f, $g, $h, $e, $ff, $gg);
    };
}

macro_rules! round4 {
    ($i:expr,$w:ident, $a: ident,$b: ident,$c: ident,$d: ident,$e: ident,$f: ident,$g: ident,$h: ident, $ff:ident, $gg: ident) => {
        round!($i, $w[($i + 0) % 16], $w[($i + 0) % 16] ^ $w[($i + 4) % 16], $a, $b, $c, $d, $e, $f, $g, $h, $ff, $gg);
        round!($i + 1, $w[($i + 1) % 16], $w[($i + 1) % 16] ^ $w[($i + 5) % 16], $d, $a, $b, $c, $h, $e, $f, $g, $ff, $gg);
        round!($i + 2, $w[($i + 2) % 16], $w[($i + 2) % 16] ^ $w[($i + 6) % 16], $c, $d, $a, $b, $g, $h, $e, $f, $ff, $gg);
        round!($i + 3, $w[($i + 3) % 16], $w[($i + 3) % 16] ^ $w[($i + 7) % 16], $b, $c, $d, $a, $f, $g, $h, $e, $ff, $gg);
    };
}

pub(crate) use round;
pub(crate) use round4;

// compress as much bytes as possible of p. return the tail of p which did not
// compress.
#[inline]
pub(crate) fn compress_generic<'a>(iv: &mut [u32; 8], p: &'a [u8]) -> &'a [u8] {
    let mut w = [0u32; 16];
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
        for (wi, bytes) in zip(&mut w, chunk.as_chunks::<4>().0) {
            *wi = u32::from_be_bytes(*bytes);
        }

        round4_with_msg_sched!(0, w, a, b, c, d, e, f, g, h, ff0, gg0);
        round4_with_msg_sched!(4, w, a, b, c, d, e, f, g, h, ff0, gg0);
        round4_with_msg_sched!(8, w, a, b, c, d, e, f, g, h, ff0, gg0);
        round4_with_msg_sched!(12, w, a, b, c, d, e, f, g, h, ff0, gg0);
        round4_with_msg_sched!(16, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(20, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(24, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(28, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(32, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(36, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(40, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(44, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4_with_msg_sched!(48, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4!(52, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4!(56, w, a, b, c, d, e, f, g, h, ff1, gg1);
        round4!(60, w, a, b, c, d, e, f, g, h, ff1, gg1);

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
