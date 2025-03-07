#![cfg_attr(
    any(target_arch = "x86", target_arch = "x86_64"),
    feature(avx512_target_feature),
    feature(stdarch_x86_avx512),
    feature(stdarch_x86_mm_shuffle)
)]

#![allow(dead_code)]

#![feature(portable_simd)]
#![allow(incomplete_features)]
#![feature(test)]
#![feature(const_trait_impl)]
#![feature(slice_as_chunks)]
#![feature(bigint_helper_methods)]
#![feature(generic_const_exprs)]
#![feature(stmt_expr_attributes)]

#![no_std]
#![warn(clippy::std_instead_of_alloc, clippy::std_instead_of_core)]


pub mod sm4;
#[macro_use]
pub mod sm3;

#[macro_use]
pub mod sm3_simd;

pub mod rand;
pub mod sm2;
pub mod cms;
pub mod blockmode;
pub mod mac;
pub mod traits;
pub mod cryptobyte;

// the non-GMT algorithm: SHA256, AES, and RSA.
pub mod x;


#[macro_use]
mod internal;

#[allow(unused_imports)]
#[macro_use]
extern crate alloc;

#[cfg(any(feature = "std", test))]
#[macro_use]
extern crate std;





