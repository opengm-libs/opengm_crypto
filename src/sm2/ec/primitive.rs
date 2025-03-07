#[cfg(target_arch = "x86_64")]
mod amd64;
#[cfg(target_arch = "x86_64")]
pub use amd64::*;

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub use generic::*;

pub mod generic {

    /// Computes `a + b + carry`, returning the result along with the new carry. 64-bit version.
    /// Carry = 0 or 1
    #[inline(always)]
    pub const fn adc(a: u64, b: u64, carry: bool) -> (u64, bool) {
        a.carrying_add(b, carry)
    }

    /// Computes `a - (b + borrow)`, returning the result along with the new borrow. 64-bit version.
    /// The returned borrow is 1 if a < b+borrow, 0 otherwise.
    #[inline(always)]
    pub const fn sbb(a: u64, b: u64, borrow: bool) -> (u64, bool) {
        a.borrowing_sub(b, borrow)
    }

    /// Computes `a + (b * c) + carry`, returning the result along with the new carry.
    #[inline(always)]
    pub const fn mac(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64) {
        let ret = (a as u128) + ((b as u128) * (c as u128)) + (carry as u128);
        (ret as u64, (ret >> 64) as u64)

        // let (l, h) = b.carrying_mul(c, carry);
        // let (l, carry) = l.carrying_add(a, false);
        // let (h, _) = h.carrying_add(0, carry);
        // (l, h)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adc() {
        let mut a = [!0, !0, !0, !0];
        let b = [1, 1, 1, 1];
        let mut carry = false;
        for i in 0..4 {
            (a[i], carry) = adc(a[i], b[i], carry);
        }
        assert_eq!(a, [0, 1, 1, 1]);
        assert_eq!(carry, true);
    }

    #[test]
    fn test_sbb() {
        let mut a = [0, 0, 0, 0];
        let b = [1, 0, 0, 0];
        let mut borrow = false;
        for i in 0..4 {
            (a[i], borrow) = sbb(a[i], b[i], borrow);
        }
        assert_eq!(a, [!0, !0, !0, !0]);
        assert_eq!(borrow, true);
    }
}
