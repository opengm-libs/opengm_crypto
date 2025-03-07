mod aarch64_neon;


use super::{generic::compress_generic, CompressFn};
use aarch64_neon::compress_aarch64_neon;
use crate::internal::cpuid::aarch64::*;



#[inline]
pub fn get_compress_fn() -> CompressFn {
    if support_neon()  {
        // compress_generic
        compress_aarch64_neon
    }else{
        compress_generic
    }
}