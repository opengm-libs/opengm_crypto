#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

