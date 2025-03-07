mod prelude {
    #[cfg(target_arch = "x86")]
    pub use core::arch::x86::*;

    #[cfg(target_arch = "x86_64")]
    pub use core::arch::x86_64::*;

    pub use core::mem::transmute;
}

mod block_aesni;
mod block_avx2;
mod block_avx512;
mod block_gfni;
mod block_vaes;
mod mem;

use block_aesni::*;
use block_avx512::*;
use block_gfni::*;
use block_vaes::*;

// Benchmark results (11th Gen Intel(R) Core(TM) i7-1165G7 @ 2.80GHz):
//
// test sm4::block::amd64::block_gfni::tests::bench_block16_gfni     ... bench:         214 ns/iter (+/- 5)
// test sm4::block::amd64::block_vaes::tests::bench_block16_vaes     ... bench:         346 ns/iter (+/- 13)
// test sm4::block::amd64::block_avx512::tests::bench_block16_avx512 ... bench:         623 ns/iter (+/- 62)
// test sm4::block::amd64::block_aesni::tests::bench_block4_aesni    ... bench:         749 ns/iter (+/- 95)
// test sm4::block::amd64::block_avx2::tests::bench_block8_avx2      ... bench:       1,102 ns/iter (+/- 26)
// test sm4::block::generic::tests::bench_block16_generic            ... bench:         707 ns/iter (+/- 81)
// test sm4::block::generic::tests::bench_block8_generic             ... bench:         708 ns/iter (+/- 14)
// test sm4::block::generic::tests::bench_block4_generic             ... bench:         708 ns/iter (+/- 36)
// test sm4::block::generic::tests::bench_block2_generic             ... bench:       1,049 ns/iter (+/- 55)
// test sm4::block::generic::tests::bench_block_generic              ... bench:       1,405 ns/iter (+/- 28)
//
// use gfni -> vaes ->  aesni -> avx512 -> generic
// for gfni, vaes, aesni no table lookup.

use crate::{internal::cpuid::x86_64::*, sm4::Blocks};
#[inline(always)]
fn gfni_avaliable() -> bool {
    support_gfni() && support_avx512f() && support_avx512bw()
}

#[inline(always)]
fn vaes_avaliable() -> bool {
    support_vaes() && support_avx512f() && support_avx512bw()
}

#[inline(always)]
fn aesni_avaliable() -> bool {
    support_aes() && support_sse2() && support_ssse3()
}

#[inline(always)]
fn avx512_avaliable() -> bool {
    support_avx512f() && support_avx512bw()
}

#[inline(always)]
fn avx2_avaliable() -> bool {
    support_avx2() && support_avx()
}




fn block4x4_aesni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
        block4_aesni(&mut dst[..64], &src[..64], rk) ;
        block4_aesni(&mut dst[64..128], &src[64..128], rk) ;
        block4_aesni(&mut dst[128..192], &src[128..192], rk) ;
        block4_aesni(&mut dst[192..256], &src[192..256], rk) ;
   }
fn block4x4_aesni_inplace(dst_src: &mut [u8], rk: &[u32]) {
       block4_aesni_inplace(&mut dst_src[..64], rk) ;
       block4_aesni_inplace(&mut dst_src[64..128], rk) ;
       block4_aesni_inplace(&mut dst_src[128..192], rk) ;
       block4_aesni_inplace(&mut dst_src[192..256], rk) ;
}

fn block4x2_aesni(dst: &mut [u8], src: &[u8], rk: &[u32]) {
    block4_aesni(&mut dst[..64], &src[..64], rk) ;
    block4_aesni(&mut dst[64..128], &src[64..128], rk) ;
}
fn block4x2_aesni_inplace(dst_src: &mut [u8], rk: &[u32]) {
   block4_aesni_inplace(&mut dst_src[..64], rk) ;
   block4_aesni_inplace(&mut dst_src[64..128], rk) ;
}

#[inline(always)]
pub(crate) fn new_blocks_amd64() -> Blocks {
    let mut blocks = Blocks::default();

    // set block16 and block8
    if gfni_avaliable() {
        blocks.block16 = block16_gfni;
        blocks.block16_inplace =  block16_gfni_inplace;
        blocks.block8 = block8_gfni;
        blocks.block8_inplace = block8_gfni_inplace;
    }else if vaes_avaliable() {
        blocks.block16 = block16_vaes;
        blocks.block16_inplace =  block16_vaes_inplace;
        blocks.block8 = block8_vaes;
        blocks.block8_inplace = block8_vaes_inplace;
    } else if aesni_avaliable() {
        blocks.block16 = block4x4_aesni;
        blocks.block16_inplace =  block4x4_aesni_inplace;
        blocks.block8 = block4x2_aesni;
        blocks.block8_inplace = block4x2_aesni_inplace;
    } else if avx512_avaliable() {
        blocks.block16 = block16_avx512;
        blocks.block16_inplace =  block16_avx512_inplace;
    };

    // set block4
    // block4_generic may a little faster.
    // but aesni has no table lookup.
    if aesni_avaliable() {
        blocks.block4 = block4_aesni;
        blocks.block4_inplace = block4_aesni_inplace;
    }

    blocks
}

#[cfg(test)]
mod tests {}
