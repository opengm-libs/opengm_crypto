pub mod byteorder;
mod tables;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod amd64;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub(crate) use amd64::*;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

pub mod generic;
pub use generic::*;

