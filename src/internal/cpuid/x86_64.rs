#[cfg(not(feature = "std"))]
pub use x86_64_no_std::*;

#[cfg(feature = "std")]
pub use x86_64_std::*;

#[cfg(feature = "std")]
mod x86_64_std {
    #[inline]
    pub fn support_gfni() -> bool {
        is_x86_feature_detected!("gfni")
    }

    #[inline]
    pub fn support_avx512f() -> bool {
        is_x86_feature_detected!("avx512f")
    }
    
    #[inline]
    pub fn support_avx512bw() -> bool {
        is_x86_feature_detected!("avx512bw")
    }
    
    #[inline]
    pub fn support_vaes() -> bool {
        is_x86_feature_detected!("vaes")
    }

    #[inline]
    pub fn support_aes() -> bool {
        is_x86_feature_detected!("aes")
    }

    #[inline]
    pub fn support_sse2() -> bool {
        is_x86_feature_detected!("sse2")
    }
    
    #[inline]
    pub fn support_ssse3() -> bool {
        is_x86_feature_detected!("ssse3")
    }
    
    #[inline]
    pub fn support_avx() -> bool {
        is_x86_feature_detected!("avx")
    }
    
    #[inline]
    pub fn support_avx2() -> bool {
        is_x86_feature_detected!("avx2")
    }


    #[inline]
    pub fn support_avx512vl() -> bool {
        is_x86_feature_detected!("avx512vl")
    }
}

/// TODO use cpuid instruction
// #[cfg(not(feature = "std"))]
mod x86_64_no_std {
    use core::arch::asm;

    #[inline]
    pub fn cpuid(leaf: u32, sub_leaf: u32) -> (u32, u32, u32, u32) {
        let mut a;
        let mut b;
        let mut c;
        let mut d;
        unsafe {
            asm!(
                "push rbx",
                "cpuid",
                "mov {0:e}, ebx",
                "pop rbx",
                out(reg) b,
                out("edx")  d,
                inout("eax") leaf => a,
                inout("ecx") sub_leaf => c,
            );
        }
        (a, b, c, d)
    }

    #[inline]
    pub fn support_gfni() -> bool {
        let (_, _, ecx, _) = cpuid(7,0);
        (ecx & (1 << 8)) != 0
    }

    #[inline]
    pub fn support_avx512f() -> bool {
        let (_, ebx, _, _) = cpuid(7,0);
        (ebx & (1 << 16)) != 0
    }

    #[inline]
    pub fn support_avx512bw() -> bool {
        let (_, ebx, _, _) = cpuid(7,0);
        (ebx & (1 << 30)) != 0
    }

    #[inline]
    pub fn support_vaes() -> bool {
        let (_, _, ecx, _) = cpuid(7,0);
        (ecx & (1 << 9)) != 0
    }

    #[inline]
    pub fn support_aes() -> bool {
        let (_, _, ecx, _) = cpuid(1,0);
        (ecx & (1 << 25)) != 0
    }

    #[inline]
    pub fn support_sse2() -> bool {
        let (_, ebx,_, _) = cpuid(1,0);
        (ebx & (1 << 26)) != 0
    }
    #[inline]
    pub fn support_ssse3() -> bool {
        let (_, _, ecx, _) = cpuid(1,0);
        (ecx & (1 << 9)) != 0
    }
    #[inline]
    pub fn support_avx() -> bool {
        let (_, _, ecx, _) = cpuid(1,0);
        (ecx & (1 << 28)) != 0
    }
    #[inline]
    pub fn support_avx2() -> bool {
        let (_, ebx,_, _) = cpuid(7,0);
        (ebx & (1 << 5)) != 0
    }
    #[inline]
    pub fn support_avx512vl() -> bool {
        let (_, ebx,_, _) = cpuid(7,0);
        (ebx & (1 << 31)) != 0
    }
}

#[cfg(test)]
mod tests {
    
    #[test]
    #[cfg(feature = "std")]
    fn test_support_gfni() {
        use super::*;
        assert_eq!(x86_64_std::support_gfni(),x86_64_no_std::support_gfni());
        assert_eq!(x86_64_std::support_vaes(),x86_64_no_std::support_vaes());
        assert_eq!(x86_64_std::support_avx512vl(),x86_64_no_std::support_avx512vl());
        assert_eq!(x86_64_std::support_avx2(),x86_64_no_std::support_avx2());
        assert_eq!(x86_64_std::support_avx512f(),x86_64_no_std::support_avx512f());
        assert_eq!(x86_64_std::support_avx512bw(),x86_64_no_std::support_avx512bw());
        assert_eq!(x86_64_std::support_avx(),x86_64_no_std::support_avx());
        assert_eq!(x86_64_std::support_ssse3(),x86_64_no_std::support_ssse3());
    }
}
