#[cfg(not(feature = "std"))]
pub use aarch64_no_std::*;

#[cfg(feature = "std")]
pub use aarch64_std::*;

#[cfg(feature = "std")]
mod aarch64_std {
    use std::arch::*;

    #[inline]
    pub fn support_neon() -> bool {
        return is_aarch64_feature_detected!("neon");
    }
    
    #[inline]
    pub fn support_aes() -> bool {
        return is_aarch64_feature_detected!("aes");
    }
}

#[cfg(not(feature = "std"))]
mod aarch64_no_std {

    #[inline]
    pub fn support_neon() -> bool {
        return true;
    }

    #[inline]
    pub fn support_aes() -> bool {
        return false;
    }
}
