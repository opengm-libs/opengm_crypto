#[cfg(target_arch = "aarch64")]
mod gfp_aarch64;

use core::fmt::Display;

use crate::sm2::U256;

use super::primitive::*;
use super::arith::*;
use super::*;

const P0: LIMB = 0xFFFFFFFFFFFFFFFF;
const P1: LIMB = 0xFFFFFFFF00000000;
const P2: LIMB = 0xFFFFFFFFFFFFFFFF;
const P3: LIMB = 0xFFFFFFFEFFFFFFFF;

/// Montgomery represented elements in GF(p),
/// The limbs is in [0,p).
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct GFpElement {
    pub limbs: [LIMB; 4],
}

impl From<[LIMB; 4]> for GFpElement {
    fn from(value: [LIMB; 4]) -> Self {
        let mut res = GFpElement { limbs: value };
        res.mul(&GFpElement::RR);
        res
    }
}

impl From<U256> for GFpElement {
    fn from(v: U256) -> Self {
        let mut res = GFpElement { limbs: v.v };
        res.mul(&GFpElement::RR);
        res
    }
}

impl From<GFpElement> for U256 {
    fn from(value: GFpElement) -> Self {
        let mut value = value;
        // TODO: mul_one()
        value.mul(&GFpElement::ONE);
        U256 { v: value.limbs }
    }
}

impl Display for GFpElement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[0x{:016x}, 0x{:016x}, 0x{:016x}, 0x{:016x}]", self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3])
    }
}

// The GFpElement are inner type, and we do not overload the core::ops for simplicity.
// The ops_move is defined for chain operations:
// eg: c = 1/(a+b)^2:
// let c: GFpElement = GFpElement::new_from_mul(&a, &b).square_move().invert_move();
impl GFpElement {
    pub const ONE: GFpElement = GFpElement { limbs: [1, 0, 0, 0] };
    pub const ZERO: GFpElement = GFpElement { limbs: [0; 4] };

    // Montgomery representation of [1] = R mod p
    pub const R: GFpElement = GFpElement {
        limbs: [0x0000000000000001, 0x00000000ffffffff, 0x0000000000000000, 0x0000000100000000],
    };

    // Montgomery representation of [R] = R^2 mod p
    pub const RR: GFpElement = GFpElement {
        limbs: [0x0000000200000003, 0x00000002ffffffff, 0x0000000100000001, 0x0000000400000002],
    };

    pub const PRIME: GFpElement = GFpElement { limbs: [P0, P1, P2, P3] };

    #[inline]
    pub fn new_from_add(a: &GFpElement, b: &GFpElement) -> GFpElement {
        let mut out = GFpElement::default();
        out.from_add(&a, &b);
        out
    }

    #[inline]
    pub fn new_from_sub(a: &GFpElement, b: &GFpElement) -> GFpElement {
        let mut out = GFpElement::default();
        out.from_sub(&a, &b);
        out
    }

    #[inline]
    pub fn new_from_double(b: &GFpElement) -> GFpElement {
        let mut out = GFpElement::default();
        out.from_double(&b);
        out
    }

    #[inline]
    pub fn new_from_mul(a: &GFpElement, b: &GFpElement) -> GFpElement {
        let mut out = GFpElement::default();
        out.from_mul(&a, &b);
        out
    }
    #[inline]
    pub fn new_from_square(b: &GFpElement) -> GFpElement {
        let mut out = GFpElement::default();
        out.from_square(&b);
        out
    }
    #[inline]
    pub fn new_from_square_n(b: &GFpElement, n: usize) -> GFpElement {
        let mut out = *b;
        out.square_n(n);
        out
    }

    #[inline]
    pub fn new_from_invert(b: &GFpElement) -> GFpElement {
        let mut out = *b;
        out.invert();
        out
    }
    #[inline]
    pub fn new_from_invert2(b: &GFpElement) -> GFpElement {
        let mut out = *b;
        out.invert2();
        out
    }
    #[inline]
    pub fn new_from_invert3(b: &GFpElement) -> GFpElement {
        let mut out = *b;
        out.invert3();
        out
    }

    /// Returns if self is zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        // Does compare with p necessary?
        constant_eq256(&self.limbs, &Self::ZERO.limbs) || constant_eq256(&self.limbs, &Self::PRIME.limbs)
    }

    // #[inline]
    // pub fn from(&mut self, b: &GFpElement) -> &mut Self {
    //     *self = *b;
    //     self
    // }

    /// self = self + 0 mod N
    /// i.e., self - p if self >= p
    #[inline]
    pub fn add_zero(&mut self) -> &mut Self {
        let a = &self.limbs;
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = sub256_conditional(a[0], a[1], a[2], a[3], 0, P0, P1, P2, P3);
        self
    }

    #[inline]
    pub fn mul_rr(&mut self) -> &mut Self {
        self.mul(&GFpElement::RR)
    }

    #[inline]
    pub fn transform_to_mont(&mut self) -> &mut Self {
        self.mul_rr()
    }

    #[inline]
    pub fn transform_from_mont(&mut self) -> &mut Self {
        self.mul_one()
    }
    
    #[inline]
    pub fn mul_one(&mut self) -> &mut Self {
        let (a0, a1, a2, a3) = (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]);
        montgomery_reduce(&mut self.limbs, a0, a1, a2, a3, 0, 0, 0, 0);
        self
    }

    /// self = a + b, assume a, b in [0, p)
    /// Note: the result is in [0,p) if a+b < 2p.
    #[inline]
    pub fn from_add(&mut self, a: &GFpElement, b: &GFpElement) -> &mut Self {
        self.clone_from(a);
        self.add(b)
    }

    /// self += b
    /// assume self and b are in [0,p)
    #[inline]
    pub fn add(&mut self, b: &GFpElement) -> &mut Self {
        let a = &self.limbs;
        let b = &b.limbs;
        let (acc0, acc1, acc2, acc3, carry) = add256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);

        let p = &Self::PRIME.limbs;
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = sub256_conditional(acc0, acc1, acc2, acc3, carry as LIMB, p[0], p[1], p[2], p[3]);
        self
    }

    #[inline]
    pub fn add_move(mut self, b: &GFpElement) -> Self {
        self.add(b);
        self
    }

    /// self = a + b, assume a, b in [0, p)
    /// Note: the result is in [0,p) if a+b < 2p.
    #[inline]
    pub fn from_double(&mut self, b: &GFpElement) -> &mut Self {
        self.from_add(b, b)
    }

    /// self += b
    /// assume self and b are in [0,p)
    #[inline]
    pub fn double(&mut self) -> &mut Self {
        let a = &self.limbs;
        let b = &self.limbs;
        let (acc0, acc1, acc2, acc3, carry) = add256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);

        let p = &Self::PRIME.limbs;
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = sub256_conditional(acc0, acc1, acc2, acc3, carry as LIMB, p[0], p[1], p[2], p[3]);
        self
    }

    #[inline]
    pub fn double_move(mut self) -> Self {
        self.double();
        self
    }

    /// self = a - b mod p, assume a, b in [0,p).
    #[inline]
    pub fn from_sub(&mut self, a: &GFpElement, b: &GFpElement) -> &mut Self {
        self.from_neg(b).add(a)
    }

    /// self = (self - b) mod p
    #[inline]
    pub fn sub(&mut self, b: &GFpElement) -> &mut Self {
        let b = &b.limbs;
        let p = &Self::PRIME.limbs;

        let (t0, t1, t2, t3, borrow) = sub256(self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3], b[0], b[1], b[2], b[3]);
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = add256_conditional(t0, t1, t2, t3, borrow, p[0], p[1], p[2], p[3]);
        self
    }

    #[inline]
    pub fn sub_move(mut self, b: &GFpElement) -> Self {
        self.sub(b);
        self
    }
    /// self = p - b, assume b in [0,p).
    #[inline]
    pub fn from_neg(&mut self, b: &GFpElement) -> &mut Self {
        self.clone_from(b);
        self.neg()
    }

    /// self = p - self, assume self in [0,p).
    #[inline]
    pub fn neg(&mut self) -> &mut Self {
        let p = &Self::PRIME.limbs;
        let b = &self.limbs;
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3], _) = sub256(p[0], p[1], p[2], p[3], b[0], b[1], b[2], b[3]);
        self
    }

    #[inline]
    pub fn neg_move(mut self) -> Self {
        self.neg();
        self
    }

    // mul works for ab < pR.

    // self = a * b
    // Let b = b0 + b1*B + b2*B + b3*B, B = 2^64, then
    //   a * b / B^4
    // = ((((a*b0/B + a*b1)/B + a*b2)/B + a*b3)/B
    #[inline(always)]
    pub fn from_mul(&mut self, a: &GFpElement, b: &GFpElement) -> &mut Self {
        let a = &a.limbs;
        let b = &b.limbs;
        let (acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7) = mul256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);
        montgomery_reduce(&mut self.limbs, acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7);
        self
    }

    #[inline(never)]
    pub fn mul(&mut self, b: &GFpElement) -> &mut Self {
        if true {
            // adc: 20
            // sbb: 21
            // mac: 16
            let a = &self.limbs;
            let b = &b.limbs;
            let (acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7) = mul256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);
            montgomery_reduce(&mut self.limbs, acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7);
            self
        } else {
            // adc: 19
            // sbb: 25
            // mac: 16
            // but use less reg, it may be faster on x86_64.

            let a = &self.limbs;
            let b = &b.limbs;

            // a * b[0]
            let (acc0, carry) = mac(0, a[0], b[0], 0);
            let (acc1, carry) = mac(0, a[1], b[0], carry);
            let (acc2, carry) = mac(0, a[2], b[0], carry);
            let (acc3, acc4) = mac(0, a[3], b[0], carry);
            let (acc1, acc2, acc3, acc4, acc5) = montgomery_reduce_narrow(acc0, acc1, acc2, acc3, acc4);
            // [acc1, acc2, acc3, acc4, acc5] = a*b[0]/B

            let (acc1, carry) = mac(acc1, a[0], b[1], 0);
            let (acc2, carry) = mac(acc2, a[1], b[1], carry);
            let (acc3, carry) = mac(acc3, a[2], b[1], carry);
            let (acc4, carry) = mac(acc4, a[3], b[1], carry);
            let (acc5, _) = adc(acc5, carry, false); // no carry, why/
            let (acc2, acc3, acc4, acc5, acc6) = montgomery_reduce_narrow(acc1, acc2, acc3, acc4, acc5);

            let (acc2, carry) = mac(acc2, a[0], b[2], 0);
            let (acc3, carry) = mac(acc3, a[1], b[2], carry);
            let (acc4, carry) = mac(acc4, a[2], b[2], carry);
            let (acc5, carry) = mac(acc5, a[3], b[2], carry);
            let (acc6, _) = adc(acc6, carry, false);
            let (acc3, acc4, acc5, acc6, acc7) = montgomery_reduce_narrow(acc2, acc3, acc4, acc5, acc6);

            let (acc3, carry) = mac(acc3, a[0], b[3], 0);
            let (acc4, carry) = mac(acc4, a[1], b[3], carry);
            let (acc5, carry) = mac(acc5, a[2], b[3], carry);
            let (acc6, carry) = mac(acc6, a[3], b[3], carry);
            let (acc7, _) = adc(acc7, carry, false);
            let (acc4, acc5, acc6, acc7, carry) = montgomery_reduce_narrow(acc3, acc4, acc5, acc6, acc7);

            // self.set_conditional_sub_p(acc4, acc5, acc6, acc7, carry)
            let p = &GFpElement::PRIME.limbs;
            (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = sub256_conditional(acc4, acc5, acc6, acc7, carry, p[0], p[1], p[2], p[3]);
            self
        }
    }

    #[inline]
    pub fn mul_move(mut self, b: &GFpElement) -> Self {
        self.mul(b);
        self
    }

    // square works if a^2 < pR

    #[inline]
    pub fn from_square(&mut self, b: &GFpElement) -> &mut Self {
        let b = &b.limbs;
        let (acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7) = square256(b[0], b[1], b[2], b[3]);
        montgomery_reduce(&mut self.limbs, acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7);
        self
    }

    #[inline]
    pub fn square(&mut self) -> &mut Self {
        let b = &mut self.limbs;
        let (acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7) = square256(b[0], b[1], b[2], b[3]);
        montgomery_reduce(b, acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7);
        self
    }

    #[inline]
    pub fn square_move(mut self) -> Self {
        self.square();
        self
    }

    #[inline]
    pub fn from_square_n(&mut self, b: &GFpElement, n: usize) -> &mut Self {
        self.clone_from(b);
        self.square_n(n)
    }

    #[inline]
    pub fn square_n(&mut self, n: usize) -> &mut Self {
        let b = &mut self.limbs;
        for _ in 0..n {
            let (acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7) = square256(b[0], b[1], b[2], b[3]);
            montgomery_reduce(b, acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7);
        }
        self
    }

    #[inline]
    pub fn square_n_move(mut self, n: usize) -> Self {
        self.square_n(n);
        self
    }

    /// self = b^-1 = b^{p-2} mod p.
    /// Note that self = 0 if b = 0.
    #[inline]
    pub fn from_invert(&mut self, b: &GFpElement) -> &mut Self {
        self.clone_from(b);
        self.invert()
    }
    #[inline]
    pub fn from_invert2(&mut self, b: &GFpElement) -> &mut Self {
        self.clone_from(b);
        self.invert2()
    }
    #[inline]
    pub fn from_invert3(&mut self, b: &GFpElement) -> &mut Self {
        self.clone_from(b);
        self.invert3()
    }

    /// self = self^-1
    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let _1 = *self;
        self.invert2().mul(&_1)
    }

    /// self = self^-2
    #[inline]
    pub fn invert2(&mut self) -> &mut Self {
        let _10 = GFpElement::new_from_square(self);
        let _11 = GFpElement::new_from_mul(&_10, self);
        let _110 = GFpElement::new_from_square(&_11);
        let _111 = GFpElement::new_from_mul(&_110, self);
        let _111000 = GFpElement::new_from_square_n(&_111, 3);
        let _111111 = GFpElement::new_from_mul(&_111000, &_111);
        let _1111110 = GFpElement::new_from_square(&_111111);
        let _1111111 = GFpElement::new_from_mul(&_1111110, self);
        let _x12 = GFpElement::new_from_square_n(&_1111110, 5).mul_move(&_111111);
        let _x24 = GFpElement::new_from_square_n(&_x12, 12).mul_move(&_x12);
        let _x31 = GFpElement::new_from_square_n(&_x24, 7).mul_move(&_1111111);
        let _i39 = GFpElement::new_from_square_n(&_x31, 2);
        let _i68 = GFpElement::new_from_square_n(&_i39, 29);
        let _x62 = GFpElement::new_from_mul(&_i68, &_x31);
        let _i71 = GFpElement::new_from_square_n(&_i68, 2);
        let _x64 = GFpElement::new_from_mul(&_i71, &_i39).mul_move(&_11);
        let _i265 = GFpElement::new_from_square_n(&_i71, 32);
        self.from_mul(&_i265, &_x64).square_n(64).mul(&_x64).square_n(94).mul(&_x62).square_n(2)
    }
    /// self = self^-3
    #[inline]
    pub fn invert3(&mut self) -> &mut Self {
        let _10 = GFpElement::new_from_square(self);
        let _11 = GFpElement::new_from_mul(&_10, self);
        let _110 = GFpElement::new_from_square(&_11);
        let _111 = GFpElement::new_from_mul(&_110, self);
        let _111000 = GFpElement::new_from_square_n(&_111, 3);
        let _111111 = GFpElement::new_from_mul(&_111000, &_111);
        let _1111110 = GFpElement::new_from_square(&_111111);
        let _1111111 = GFpElement::new_from_mul(&_1111110, self);
        let _x12 = GFpElement::new_from_square_n(&_1111110, 5).mul_move(&_111111);
        let _x24 = GFpElement::new_from_square_n(&_x12, 12).mul_move(&_x12);
        let _x30 = GFpElement::new_from_square_n(&_x24, 6).mul_move(&_111111);
        let _x31 = GFpElement::new_from_square(&_x30).mul_move(&self);
        let _i67 = GFpElement::new_from_square_n(&_x31, 30);
        let _x61 = GFpElement::new_from_mul(&_i67, &_x30);

        self.from_square_n(&_i67, 32)
            .mul(&_x61)
            .square_n(61)
            .mul(&_x61)
            .square_n(6)
            .mul(&_111111)
            .square_n(93)
            .mul(&_x61)
            .square_n(3)
            .mul(&_11)
    }
    #[inline]
    pub fn invert_move(mut self) -> Self {
        self.invert();
        self
    }
    #[inline]
    pub fn invert2_move(mut self) -> Self {
        self.invert2();
        self
    }
    #[inline]
    pub fn invert3_move(mut self) -> Self {
        self.invert3();
        self
    }
}

// returns a/B mod p = (a + a0*p)>>64
// Note that (a + a0*p) <= (B^4-1) + (B-1)^2 = B^3 + B - 2 < B^4.
#[inline(always)]
pub fn montgomery_reduce_limb(a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB) -> (LIMB, LIMB, LIMB, LIMB) {
    #[cfg(target_arch = "aarch64")]
    return gfp_aarch64::montgomery_reduce_limb_aarch64(a0, a1, a2, a3);
    
    #[cfg(not(any(target_arch = "aarch64")))]
    {
        let lo = a0 << 32;
        let hi = a0 >> 32;
        let (a1, carry) = adc(a1, a0, false);
        let (a2, carry) = adc(a2, 0, carry);
        let (a3, carry) = adc(a3, 0, carry);
        let (a0, _) = adc(a0, 0, carry);

        let (a1, borrow) = sbb(a1, lo, false);
        let (a2, borrow) = sbb(a2, hi, borrow);
        let (a3, borrow) = sbb(a3, lo, borrow);
        let (a0, _) = sbb(a0, hi, borrow);

        (a1, a2, a3, a0)
    }
}

// returns a/R = a/(B^4) mod p
// The returned result < B^4, but can the result >= p?
#[inline(always)]
fn montgomery_reduce(out: &mut [LIMB; 4], a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB, a4: LIMB, a5: LIMB, a6: LIMB, a7: LIMB) {
    // let (a1, a2, a3, a0) = montgomery_reduce_limb(a0, a1, a2, a3);
    // let (a2, a3, a4, a1) = montgomery_reduce_limb(a1, a2, a3, a4);
    // let (a3, a4, a5, a2) = montgomery_reduce_limb(a2, a3, a4, a5);
    // let (a4, a5, a6, a3) = montgomery_reduce_limb(a3, a4, a5, a6);
    // The same thing:
    let (a1, a2, a3, a0) = montgomery_reduce_limb(a0, a1, a2, a3);
    let (a2, a3, a0, a1) = montgomery_reduce_limb(a1, a2, a3, a0);
    let (a3, a0, a1, a2) = montgomery_reduce_limb(a2, a3, a0, a1);
    let (a0, a1, a2, a3) = montgomery_reduce_limb(a3, a0, a1, a2);

    (out[0], out[1], out[2], out[3]) = add256_mod(a0, a1, a2, a3, a4, a5, a6, a7,P0, P1, P2, P3);

    // let (acc0, acc1, acc2, acc3, carry) = add256(a0, a1, a2, a3, a4, a5, a6, a7);
    // (out[0], out[1], out[2], out[3]) = sub256_conditional(acc0, acc1, acc2, acc3, carry as LIMB, P0, P1, P2, P3)
}

// returns [a0, a1, a2, a3, a4]/B.
// Note that return = [(a0 + a1*B + ... + a4*B^4) + a0*p]/B.
// For p = -1 mod B, thus the division is shifting right by 64.
// Note: The result may exceed the B^4.
#[inline(always)]
fn montgomery_reduce_narrow(a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB, a4: LIMB) -> (LIMB, LIMB, LIMB, LIMB, LIMB) {
    let (a1, carry) = adc(a1, a0, false);
    let (a2, carry) = adc(a2, 0, carry);
    let (a3, carry) = adc(a3, 0, carry);
    let (a4, carry) = adc(a4, a0, carry);

    let lo = a0 << 32;
    let hi = a0 >> 32;

    let (a1, borrow) = sbb(a1, lo, false);
    let (a2, borrow) = sbb(a2, hi, borrow);
    let (a3, borrow) = sbb(a3, lo, borrow);
    let (a4, borrow) = sbb(a4, hi, borrow);
    let (a5, _) = sbb(carry as u64, 0, borrow);

    (a1, a2, a3, a4, a5)
}

#[cfg(test)]
mod test {

    use super::*;
    use hex_literal::hex;
    use num::BigUint;
    use rand::Rng;
    use std::time::SystemTime;

    fn get_prime() -> BigUint {
        BigUint::from_bytes_be(hex!("FFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000FFFFFFFFFFFFFFFF").as_slice())
    }

    fn to_bigint(el: &[u64]) -> BigUint {
        let mut res = BigUint::default();
        let n = el.len();
        for i in 0..n {
            res <<= 64;
            res += el[n - 1 - i];
        }
        res
    }

    fn random() -> GFpElement {
        let mut rng = rand::rng();
        let p = &GFpElement::PRIME.limbs;
        let (a0, a1, a2, a3) = (rng.random(), rng.random(), rng.random(), rng.random());
        let (a0, a1, a2, a3) = sub256_conditional(a0, a1, a2, a3, 0, p[0], p[1], p[2], p[3]);
        GFpElement { limbs: [a0, a1, a2, a3] }
    }

    #[test]
    fn test_add() {
        let a = GFpElement {
            limbs: [0, 0xFFFFFFFF00000001, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF],
        };
        let b = GFpElement {
            limbs: [0xFFFFFFFFFFFFFFFD, 0xFFFFFFFF00000000, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF],
        };
        let mut c = GFpElement { limbs: [0; 4] };
        c.from_add(&a, &b);
        assert_eq!(
            c,
            GFpElement {
                limbs: [0xFFFFFFFFFFFFFFFE, 0xFFFFFFFF00000000, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF]
            }
        );
        c.from_add(&a, &GFpElement::PRIME);
        assert_eq!(c, a);
    }

    #[test]
    fn test_add_exception() {
        let a = GFpElement {
            limbs: [0, 0xFFFFFFFF00000001, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF], // p+1
        };
        let b = a;
        let mut c = GFpElement { limbs: [0; 4] };
        c.from_add(&a, &b);
        assert_eq!(
            c,
            GFpElement {
                limbs: [1, 0xFFFFFFFF00000001, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF] // p+2
            }
        );
        c.from_add(&a, &GFpElement::PRIME);
        assert_eq!(c, a);
    }

    // 290 MTPS vs 530 MTPS(fincrypto) on M1.
    #[test]
    fn test_add_fuzzy() {
        for _ in 0..100000 {
            let a = random();
            let b = random();
            let mut c = GFpElement::default();
            c.from_add(&a, &b);
            let c = to_bigint(&c.limbs);

            let aa = to_bigint(&a.limbs);
            let bb = to_bigint(&b.limbs);
            let mut cc = aa + bb;
            let p = get_prime();
            if cc >= p {
                cc -= p;
            }

            assert_eq!(c, cc);
        }
    }

    #[test]
    fn test_sub_add() {
        for _ in 0..100000 {
            let a = random();
            let b = random();
            let mut c = GFpElement::default();

            c.from_add(&a, &b);
            c.sub(&a);
            assert_eq!(c, b);
        }
    }

    #[test]
    fn test_invert() {
        let mut c = GFpElement::default();
        let mut c2 = GFpElement::default();
        let mut c3 = GFpElement::default();
        for _ in 0..1000 {
            let a = random();
            c.from_invert(&a).mul(&a);
            assert_eq!(c, GFpElement::R);
        }

        for _ in 0..1000 {
            let a = random();
            c2.from_invert2(&a);
            c.from_invert(&a).square();
            assert_eq!(c, c2);
        }
        for _ in 0..1000 {
            let a = random();
            c3.from_invert3(&a);
            c.from_invert2(&a).square().mul(&a);
            assert_eq!(c, c3);
        }
    }

    #[test]
    fn test_montgomery_reduce_limb() {
        let mut rng = rand::rng();
        let mut binv = BigUint::from_slice(&[1]);
        binv <<= 64;
        binv = binv.modinv(&get_prime()).unwrap();
        // for _ in 0..100000000 {
        loop {
            let data: [u64; 4] = [rng.random(), rng.random(), rng.random(), rng.random()];
            let a = montgomery_reduce_limb(data[0], data[1], data[2], data[3]);
            let a = to_bigint(&[a.0, a.1, a.2, a.3]);
            let aa = (to_bigint(&data) * &binv) % get_prime();
            assert_eq!(a, aa);
        }
    }

    #[test]
    fn test_montgomery_reduce() {
        let mut rng = rand::rng();
        let mut rinv = BigUint::from_slice(&[1]);
        rinv <<= 256;
        rinv = rinv.modinv(&get_prime()).unwrap();

        // for _ in 0..1000000 {
        let mut i: u128 = 0;
        loop {
            if i % (100 * 1000 * 1000) == 0 {
                println!("test: {}亿", i / (100 * 1000 * 1000));
            }
            i += 1;
            let data: [u64; 8] = [rng.random(), rng.random(), rng.random(), rng.random(), rng.random(), rng.random(), rng.random(), rng.random()];
            let mut a = [0; 4];
            montgomery_reduce(&mut a, data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]);
            let a = to_bigint(&a);

            let aa = (to_bigint(&data) * &rinv) % get_prime();
            assert_eq!(a, aa);
            if a != aa {
                println!("data: {:?}", data);
                println!("aa  : {:?}", aa);
                println!("a   : {:?}", a);
                break;
            }
        }
    }

    #[test]
    fn test_mul() {
        {
            let a = GFpElement { limbs: [1, 0, 0, 0] };
            let b = GFpElement {
                limbs: [0x0000000000000001, 0x00000000ffffffff, 0x0000000000000000, 0x0000000100000000],
            };
            let wanted = GFpElement { limbs: [1, 0, 0, 0] };
            let c = GFpElement::new_from_mul(&a, &b);
            println!("{}", c);

            assert_eq!(c.limbs, wanted.limbs);
        }
        {
            let a = GFpElement::PRIME;
            let b = GFpElement {
                limbs: [0, 0xFFFFFFFF00000001, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF],
            }; // p+1
            let wanted = GFpElement { limbs: [0, 0, 0, 0] };
            let c = GFpElement::new_from_mul(&a, &b);
            println!("{}", c);

            assert_eq!(c.limbs, wanted.limbs);
        }
        {
            // let a = GFpElement{limbs:[0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF]}; // R-1
            let a = GFpElement::PRIME;
            let b = GFpElement {
                limbs: [0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF],
            }; // R-1
            let wanted = GFpElement { limbs: [0, 0, 0, 0] };
            let c = GFpElement::new_from_mul(&a, &b);
            println!("{}", c);

            assert_eq!(c, wanted);
        }
    }

    #[test]
    fn test_mul_fuzz() {
        let rinv = to_bigint(&GFpElement::R.limbs).modinv(&get_prime()).unwrap();
        for _i in 0..10000 {
            let a = random();
            let b = random();
            let c = GFpElement::new_from_mul(&a, &b);
            let c = to_bigint(&c.limbs);
            let aa = to_bigint(&a.limbs);
            let bb = to_bigint(&b.limbs);
            let cc = aa * bb * &rinv % get_prime();
            assert_eq!(c, cc);
        }
    }

    #[test]
    fn test_sqr_fuzz() {
        let rinv = to_bigint(&GFpElement::R.limbs).modinv(&get_prime()).unwrap();
        for _i in 0..10000 {
            let a = random();
            let mut c = GFpElement::default();
            c.from_square(&a);
            let c = to_bigint(&c.limbs);

            let aa = to_bigint(&a.limbs);
            let cc = &aa * &aa * &rinv % get_prime();
            assert_eq!(c, cc);
        }
    }
    #[test]
    fn test_mont() {
        for _ in 0..10000 {
            let a = random();
            let mut b = a;
            b.transform_to_mont().transform_from_mont();
            assert_eq!(a, b);
        }
    }

    extern crate test;
    // cargo test --release --package opengm_crypto --lib -- sm2::ec::gfp::test::test_add_speed --exact --show-output
    #[test]
    fn test_add_speed() {
        // 614 MTPS vs 400 MTPS(fincrypto) on M1.
        let a = GFpElement {
            limbs: [0xFFFFFFFFFFFFFFFE, 0xFFFFFFFF00000000, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF],
        };
        let b = a;
        let mut c = a;

        let loops = 1000000000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(c.from_add(&a, &b));
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", c);
        println!("{} MTPS", (loops as u128 * 1000000000) / (1000000 * elapsed));
    }

    #[test]
    fn test_mul_speed() {
        // 62 MTPS vs 72 MTPS(fincrypto) on M1.
        let a = GFpElement {
            limbs: [0xFFFFFFFFFFFFFFFE, 0xFFFFFFFF00000000, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF],
        };
        let b = a;
        let mut c = a;

        let loops = 100000000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(c.mul(&b));
            // test::black_box(c.from_mul(&a, &b));
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", c);
        println!("{} MTPS", (loops as u128 * 1000000000) / (1000000 * elapsed));
    }

    #[test]
    fn test_sqr_speed() {
        // 62 MTPS vs 72 MTPS(fincrypto) on M1.
        let mut a = random();
        let mut c = a;

        let loops = 10000000u64;

        let now = SystemTime::now();
        for _ in 0..loops / 2 {
            test::black_box(c.from_square(&a));
            test::black_box(a.from_square(&c));
            // test::black_box(c.square_assign());
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", c);
        println!("{} MTPS", (loops as u128 * 1000000000) / (1000000 * elapsed));
    }

    #[test]
    fn test_invert_speed() {
        // 62 MTPS vs 72 MTPS(fincrypto) on M1.
        let a = random();
        let mut c = a;

        let loops = 1000000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            c.from_invert(&a);
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", c);
        println!("{}K TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }

    use rand::rng;
    use test::Bencher;
    #[bench]
    fn bench_mul256(b: &mut Bencher) {
        let mut e0 = GFpElement{
            limbs:  rng().random(),
        };
        let e1 = GFpElement{
            limbs:  rng().random(),
        };
        // general: 21.74ns
        // aarch64: 16.05ns
        b.iter(|| {
            // test::black_box(e0.mul(&e1));
            test::black_box({
                for _ in 0..10{
                    e0.mul(&e1);
                }
            })
        });
    }
    #[bench]
    fn bench_mont_reduce(b: &mut Bencher) {
        let mut a = GFpElement{
            limbs:  rng().random(),
        };
        let aa:[u64;8] = rng().random();

        // general: 21.74ns
        // aarch64: 7.01 ns
        b.iter(|| {
            test::black_box({
                montgomery_reduce(&mut a.limbs, aa[0], aa[1], aa[2], aa[3], aa[4], aa[5],aa[6], aa[7]);
            });
        });
    }

}
