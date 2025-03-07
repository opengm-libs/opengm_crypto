#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as arch;

use super::generic;

// Add with carry
#[inline(always)]
pub fn adc(a: u64, b: u64, carry: bool) -> (u64, bool) {
    unsafe {
        let mut out = 0;
        let carry = arch::_addcarry_u64(carry as u8, a, b, &mut out);
        (out, carry != 0)
    }
}

// Subtract with borrow
#[inline(always)]
pub fn sbb(a: u64, b: u64, borrow: bool) -> (u64, bool) {
    unsafe {
        let mut out = 0;
        let borrow = arch::_subborrow_u64(borrow as u8, a, b, &mut out);
        (out, borrow != 0)
    }
}

/// Computes `a + (b * c) + carry`, returning the result along with the new carry.
/// Note there will be no carry because
/// a + (b * c) + carry <= B-1 + (B-1)*(B-1) + (B-1) = B^2 - 1.
#[inline(always)]
pub fn mac(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64) {
    generic::mac(a, b, c, carry)
}
