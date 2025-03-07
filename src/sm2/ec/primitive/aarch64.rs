// #[cfg(target_arch = "aarch64")]
// use core::arch::aarch64 as arch;

use super::generic;

/// Computes `a + b + carry`, returning the result along with the new carry. 64-bit version.
#[inline(always)]
pub fn adc(a: u64, b: u64, carry: bool) -> (u64, bool) {
    generic::adc(a, b, carry)
}

/// Computes `a - (b + borrow)`, returning the result along with the new borrow. 64-bit version.
#[inline(always)]
pub fn sbb(a: u64, b: u64, borrow: bool) -> (u64, bool) {
    generic::sbb(a, b, borrow)
}

/// Computes `a + (b * c) + carry`, returning the result along with the new carry.
#[inline(always)]
pub fn mac(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64) {
    generic::mac(a, b, c, carry)
}
