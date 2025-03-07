
mod amd64_sse2;
mod amd64_avx2;
mod amd64_avx512;

use amd64_avx2::compress_amd64_avx2;
use amd64_avx512::compress_amd64_avx512;
use amd64_sse2::compress_amd64_sse2;
use crate::internal::cpuid::x86_64::*;
use super::{generic::compress_generic, CompressFn};

#[inline]
pub fn get_compress_fn() -> CompressFn {
    if false {
        //
        // compress_generic // 338
        // compress_amd64_sse2 // 354
        compress_amd64_avx2 // 367
        // compress_amd64_avx512 // 390
    } else {
        // 由于compress_amd64_avx512要将消息扩展临时存起来,增加了内存访问,所以提升并不明显.
        if support_avx512f() && support_avx512vl() {
            return compress_amd64_avx512;
        } else if  support_avx2() && support_avx(){
            return compress_amd64_avx2;
        }else if support_sse2() && support_ssse3() {
            return compress_amd64_sse2;
        }

        compress_generic
    }
}
