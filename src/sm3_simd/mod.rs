#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod amd64;


#[cfg(target_arch = "aarch64")]
mod digest;
