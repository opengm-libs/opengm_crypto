use core::{arch::aarch64::*, mem::transmute};

use crate::{
    internal::cpuid::aarch64::support_aes,
    sm4::{
        block2_generic, block2_generic_inplace, block4_generic, block8_generic,
        block_generic, block_generic_inplace, Blocks, BLOCK_SIZE,
    },
};

use super::{block4_generic_inplace, block8_generic_inplace};

#[inline(always)]
pub(crate) fn new_blocks_aarch64() -> Blocks {
    if support_aes() {
        Blocks {
            block16: |output, input, rk| {
                block8_aes(
                    &mut output[..8 * BLOCK_SIZE],
                    &input[..8 * BLOCK_SIZE],
                    rk,
                );
                block8_aes(
                    &mut output[8 * BLOCK_SIZE..16 * BLOCK_SIZE],
                    &input[8 * BLOCK_SIZE..16 * BLOCK_SIZE],
                    rk,
                );
            },
            block8: block8_aes,
            block4: block4_aes,
            block2: block2_generic,
            block: block_generic,
            block16_inplace: |inout, rk| {
                block8_aes_inplace(
                    &mut inout[..8 * BLOCK_SIZE],
                    rk,
                );
                block8_aes_inplace(
                    &mut inout[8 * BLOCK_SIZE..16 * BLOCK_SIZE],
                    rk,
                );
            },
            block8_inplace: block8_aes_inplace,
            block4_inplace: block4_aes_inplace,
            block2_inplace: block2_generic_inplace,
            block_inplace: block_generic_inplace,
        }
    } else {
        Blocks::default()
    }
}

macro_rules! vrol_u32 {
    ($v: ident, $n: literal) => {{
        const M: i32 = 32 - $n;
        transmute(veorq_u32(
            vshlq_n_u32::<$n>(transmute($v)),
            vshrq_n_u32::<M>(transmute($v)),
        ))
    }};
}

#[inline(always)]
fn expand_roundkey(rk: &[u32], i: usize) -> [uint8x16_t; 4] {
    unsafe {
        [
            transmute([rk[i + 0], rk[i + 0], rk[i + 0], rk[i + 0]]),
            transmute([rk[i + 1], rk[i + 1], rk[i + 1], rk[i + 1]]),
            transmute([rk[i + 2], rk[i + 2], rk[i + 2], rk[i + 2]]),
            transmute([rk[i + 3], rk[i + 3], rk[i + 3], rk[i + 3]]),
        ]
    }
}

#[inline(always)]
unsafe fn roundx2(
    a: uint8x16_t,
    b: uint8x16_t,
    c: uint8x16_t,
    d: uint8x16_t,
    aa: uint8x16_t,
    bb: uint8x16_t,
    cc: uint8x16_t,
    dd: uint8x16_t,
    rk: uint8x16_t,
) -> (uint8x16_t, uint8x16_t) {
    let x = veorq_u8(veorq_u8(veorq_u8(b, c), d), rk);
    let xx = veorq_u8(veorq_u8(veorq_u8(bb, cc), dd), rk);

    /* A1 */
    let y = vandq_u8(x, C0F);
    let yy = vandq_u8(xx, C0F);

    let y = vqtbl1q_u8(M1L, y);
    let yy = vqtbl1q_u8(M1L, yy);

    let x = transmute(vshrq_n_u64::<4>(transmute(x)));
    let xx = transmute(vshrq_n_u64::<4>(transmute(xx)));

    let x = vandq_u8(x, C0F);
    let xx = vandq_u8(xx, C0F);

    let x = veorq_u8(vqtbl1q_u8(M1H, x), y);
    let xx = veorq_u8(vqtbl1q_u8(M1H, xx), yy);

    /* ShiftRows inverse */
    let x = vqtbl1q_u8(x, SHR);
    let xx = vqtbl1q_u8(xx, SHR); // no need?

    let x = vaeseq_u8(x, ZERO);
    let xx = vaeseq_u8(xx, ZERO);

    /* A2 */
    let y = vandq_u8(x, C0F);
    let yy = vandq_u8(xx, C0F);

    let y = vqtbl1q_u8(M2L, y);
    let yy = vqtbl1q_u8(M2L, yy);

    let x = transmute(vshrq_n_u64::<4>(transmute(x)));
    let xx = transmute(vshrq_n_u64::<4>(transmute(xx)));

    let x = vandq_u8(x, C0F);
    let xx = vandq_u8(xx, C0F);

    let x = veorq_u8(vqtbl1q_u8(M2H, x), y);
    let xx = veorq_u8(vqtbl1q_u8(M2H, xx), yy);

    let y = veorq_u8(x, vqtbl1q_u8(x, R08));
    let yy = veorq_u8(xx, vqtbl1q_u8(xx, R08));

    let y = veorq_u8(y, vqtbl1q_u8(x, R16));
    let yy = veorq_u8(yy, vqtbl1q_u8(xx, R16));

    let y = vrol_u32!(y, 2);
    let yy = vrol_u32!(yy, 2);
    (
        veorq_u8(veorq_u8(veorq_u8(vqtbl1q_u8(x, R24), y), x), a),
        veorq_u8(veorq_u8(veorq_u8(vqtbl1q_u8(xx, R24), yy), xx), aa),
    )
}

#[inline(always)]
unsafe fn round4x2(
    a: uint8x16_t,
    b: uint8x16_t,
    c: uint8x16_t,
    d: uint8x16_t,
    aa: uint8x16_t,
    bb: uint8x16_t,
    cc: uint8x16_t,
    dd: uint8x16_t,
    rk: [uint8x16_t; 4],
) -> (
    uint8x16_t,
    uint8x16_t,
    uint8x16_t,
    uint8x16_t,
    uint8x16_t,
    uint8x16_t,
    uint8x16_t,
    uint8x16_t,
) {
    let (a, aa) = roundx2(a, b, c, d, aa, bb, cc, dd, rk[0]);
    let (b, bb) = roundx2(b, c, d, a, bb, cc, dd, aa, rk[1]);
    let (c, cc) = roundx2(c, d, a, b, cc, dd, aa, bb, rk[2]);
    let (d, dd) = roundx2(d, a, b, c, dd, aa, bb, cc, rk[3]);
    (a, b, c, d, aa, bb, cc, dd)
}

#[inline]
pub fn block8_aes(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    unsafe { block8_aes_inner(dst, Some(src), rk) }
}

#[inline]
pub fn block8_aes_inplace(in_out: &mut [u8], rk: &[u32]) {
    unsafe { block8_aes_inner(in_out, None, rk) }
}

#[target_feature(enable = "aes", enable = "neon")]
pub unsafe fn block8_aes_inner(dst: &mut [u8], src: Option<&[u8]>, rk: &[u32]) {
    if true {
        match src {
            Some(src) => block8_generic(dst, src, rk),
            None => block8_generic_inplace(dst, rk),
        };
        return;
    } else {
        let src = match src {
            Some(src) => src,
            None => dst,
        };

        let uint32x4x4_t(a, b, c, d) = vld4q_u32(src.as_ptr() as *const u32);
        let uint32x4x4_t(aa, bb, cc, dd) =
            vld4q_u32((&src[64..]).as_ptr() as *const u32);
        let a = vrev32q_u8(transmute(a));
        let b = vrev32q_u8(transmute(b));
        let c = vrev32q_u8(transmute(c));
        let d = vrev32q_u8(transmute(d));
        let aa = vrev32q_u8(transmute(aa));
        let bb = vrev32q_u8(transmute(bb));
        let cc = vrev32q_u8(transmute(cc));
        let dd = vrev32q_u8(transmute(dd));

        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 0));
        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 4));
        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 8));
        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 12));
        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 16));
        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 20));
        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 24));
        let (a, b, c, d, aa, bb, cc, dd) =
            round4x2(a, b, c, d, aa, bb, cc, dd, expand_roundkey(rk, 28));

        let a = transmute(vrev32q_u8(a));
        let b = transmute(vrev32q_u8(b));
        let c = transmute(vrev32q_u8(c));
        let d = transmute(vrev32q_u8(d));
        vst4q_u32(dst.as_ptr() as *mut u32, uint32x4x4_t(d, c, b, a));
        let aa = transmute(vrev32q_u8(aa));
        let bb = transmute(vrev32q_u8(bb));
        let cc = transmute(vrev32q_u8(cc));
        let dd = transmute(vrev32q_u8(dd));
        vst4q_u32(
            (&mut dst[64..]).as_ptr() as *mut u32,
            uint32x4x4_t(dd, cc, bb, aa),
        );
    }
}

#[inline]
pub fn block4_aes(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    unsafe { block4_aes_inner(dst, Some(src), rk) }
}

#[inline]
pub fn block4_aes_inplace(in_out: &mut [u8], rk: &[u32]) {
    unsafe { block4_aes_inner(in_out, None, rk) }
}

#[target_feature(enable = "aes", enable = "neon")]
pub unsafe fn block4_aes_inner(dst: &mut [u8], src: Option<&[u8]>, rk: &[u32]) {
    if true {
        // 186ns
        // The block4_generic are faster than neon aes.
        match src {
            Some(src) => block4_generic(dst, src, rk),
            None => block4_generic_inplace(dst, rk),
        };
        return;

        // but 4 block_generic are slower
        // block_generic(&mut dst[0..16], &src[0..16], rk);
        // block_generic(&mut dst[16..32], &src[16..32], rk);
        // block_generic(&mut dst[32..48], &src[32..48], rk);
        // block_generic(&mut dst[48..64], &src[48..64], rk);

        // 2 block2_generic are almost the same with aes neon.
        // block2_generic(&mut dst[0..32], &src[0..32], rk);
        // block2_generic(&mut dst[32..64], &src[32..64], rk);
    } else {
        let src = match src {
            Some(src) => src,
            None => dst,
        };

        // 313ns
        let uint32x4x4_t(a, b, c, d) = vld4q_u32(src.as_ptr() as *const u32);
        let a = vrev32q_u8(transmute(a));
        let b = vrev32q_u8(transmute(b));
        let c = vrev32q_u8(transmute(c));
        let d = vrev32q_u8(transmute(d));

        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 0));
        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 4));
        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 8));
        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 12));
        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 16));
        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 20));
        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 24));
        let (a, b, c, d) = round4(a, b, c, d, expand_roundkey(rk, 28));

        let a = transmute(vrev32q_u8(a));
        let b = transmute(vrev32q_u8(b));
        let c = transmute(vrev32q_u8(c));
        let d = transmute(vrev32q_u8(d));
        vst4q_u32(dst.as_ptr() as *mut u32, uint32x4x4_t(d, c, b, a));
    }
}

const ZERO: uint8x16_t = unsafe { transmute([0u64; 2]) };
const C0F: uint8x16_t =
    unsafe { transmute([0x0F0F0F0F0F0F0F0Fu64, 0x0F0F0F0F0F0F0F0F]) };
const R08: uint8x16_t =
    unsafe { transmute([0x0605040702010003u64, 0x0E0D0C0F0A09080B]) };
const R16: uint8x16_t =
    unsafe { transmute([0x0504070601000302u64, 0x0D0C0F0E09080B0A]) };
const R24: uint8x16_t =
    unsafe { transmute([0x0407060500030201u64, 0x0C0F0E0D080B0A09]) };
const SHR: uint8x16_t =
    unsafe { transmute([0x0B0E0104070A0D00u64, 0x0306090C0F020508]) };
const M1L: uint8x16_t =
    unsafe { transmute([0x37bb078bb23e820eu64, 0xa82498142da11d91]) };
const M1H: uint8x16_t =
    unsafe { transmute([0x7db29f5c21eec30u64, 0xfd321fdca16e438]) };
const M2L: uint8x16_t =
    unsafe { transmute([0x40f88a327ec6b40cu64, 0x279fed5519a1d36b]) };
const M2H: uint8x16_t =
    unsafe { transmute([0x4dad1dfdd0308060u64, 0x8d6ddd3d10f040a0]) };

#[inline(always)]
unsafe fn round(
    a: uint8x16_t,
    b: uint8x16_t,
    c: uint8x16_t,
    d: uint8x16_t,
    rk: uint8x16_t,
) -> uint8x16_t {
    let x = veorq_u8(veorq_u8(veorq_u8(b, c), d), rk);

    /* A1 */
    let y = vandq_u8(x, C0F);
    let y = vqtbl1q_u8(M1L, y);
    let x = transmute(vshrq_n_u64::<4>(transmute(x)));
    let x = vandq_u8(x, C0F);
    let x = veorq_u8(vqtbl1q_u8(M1H, x), y);

    /* ShiftRows inverse */
    let x = vqtbl1q_u8(x, SHR);
    let x = vaeseq_u8(x, ZERO);

    /* A2 */
    let y = vandq_u8(x, C0F);
    let y = vqtbl1q_u8(M2L, y);
    let x = transmute(vshrq_n_u64::<4>(transmute(x)));
    let x = vandq_u8(x, C0F);
    let x = veorq_u8(vqtbl1q_u8(M2H, x), y);

    let y = veorq_u8(x, vqtbl1q_u8(x, R08));
    let y = veorq_u8(y, vqtbl1q_u8(x, R16));
    let y = vrol_u32!(y, 2);
    veorq_u8(veorq_u8(veorq_u8(vqtbl1q_u8(x, R24), y), x), a)
}

#[inline(always)]
unsafe fn round4(
    a: uint8x16_t,
    b: uint8x16_t,
    c: uint8x16_t,
    d: uint8x16_t,
    rk: [uint8x16_t; 4],
) -> (uint8x16_t, uint8x16_t, uint8x16_t, uint8x16_t) {
    let a = round(a, b, c, d, rk[0]);
    let b = round(b, c, d, a, rk[1]);
    let c = round(c, d, a, b, rk[2]);
    let d = round(d, a, b, c, rk[3]);
    (a, b, c, d)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::sm4::tests::get_tests_data;

    #[test]
    fn test_block4() {
        let (plain, wanted_cipher, rk) = get_tests_data(64);
        let mut cipher = [0u8; 64];
        block4_aes(&mut cipher, &plain[..], &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    #[test]
    fn test_block8() {
        let (plain, wanted_cipher, rk) = get_tests_data(128);
        let mut cipher = [0u8; 128];
        block8_aes(&mut cipher, &plain[..], &rk);
        assert_eq!(&wanted_cipher, &cipher);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_block4_aes(b: &mut Bencher) {
        let (plain, _wanted_cipher, rk) = get_tests_data(64);
        let mut cipher = [0; 64];
        b.iter(|| block4_aes(&mut cipher, &plain, &rk));
    }

    #[bench]
    fn bench_block8_aes(b: &mut Bencher) {
        let (plain, _wanted_cipher, rk) = get_tests_data(128);
        let mut cipher = [0; 128];
        b.iter(|| block8_aes(&mut cipher, &plain, &rk));
    }
}
