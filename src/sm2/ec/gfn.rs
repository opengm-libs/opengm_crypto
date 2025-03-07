use gfp::GFpElement;
use crate::sm2::{ec::{primitive::*, arith::*, *}, U256};

/// Finite field GF(n), where n is the order of group E(GF(p))
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct GFnElement {
    pub limbs: [LIMB; 4],
}    

#[inline]
pub fn new_from_sub(a: &GFnElement, b: &GFnElement) -> GFnElement {
    let mut out = *a;
    out.sub(b);
    out
}

// u = -N^{-1} mod 2^64
const U: u64 = 0x327f9e8872350975;
const N0: u64 = 0x53bbf40939d54123;
const N1: u64 = 0x7203df6b21c6052b;
const N2: u64 = 0xffffffffffffffff; // i.e., -1
const N3: u64 = 0xfffffffeffffffff;

// 2^256 % N
const R0: u64 = 0xac440bf6c62abedd;
const R1: u64 = 0x8dfc2094de39fad4;
const R2: u64 = 0;
const R3: u64 = 0x100000000;

impl From<&[u8;32]> for GFnElement {
    fn from(v: &[u8;32]) -> Self {
        let (chunks,_) = v.as_chunks::<8>();
        let mut res = GFnElement{
            limbs:[u64::from_be_bytes(chunks[3]),u64::from_be_bytes(chunks[2]),u64::from_be_bytes(chunks[1]),u64::from_be_bytes(chunks[0])]
        };
        res.add_zero();
        res
    }
}

impl From<&[u64;4]> for GFnElement {
    fn from(v: &[u64;4]) -> Self {
        GFnElement{
            limbs:*v
        }
    }
}

impl From<U256> for GFnElement {
    fn from(v: U256) -> Self {
        let mut res = GFnElement{
            limbs:v.v,
        };
        res.add_zero();
        res
    }
}

impl From<&U256> for GFnElement {
    fn from(v: &U256) -> Self {
        let mut res = GFnElement{
            limbs:v.v,
        };
        res.add_zero();
        res
    }
}

impl From<GFnElement> for U256 {
    fn from(value: GFnElement) -> Self {
        U256 { v: value.limbs }
    }
}

impl From<&GFpElement>for GFnElement{
    fn from(v: &GFpElement) -> Self {
        let mut res = GFnElement{
            limbs:v.limbs,
        };
        res.add_zero();
        res
    }
}

// impl Add<GFpElement> for GFnElement{
//     type Output = GFnElement;

//     fn add(self, rhs: GFpElement) -> Self::Output {
//         let mut out = Self::Output::default();
//         let a = &self.limbs;
//         let b = &rhs.limbs;
//         let (acc0, acc1, acc2, acc3, carry) = add256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);

//         (out.limbs[0], out.limbs[1], out.limbs[2], out.limbs[3]) = sub256_conditional(acc0, acc1, acc2, acc3, carry, N0, N1, N2, N3);
//         out
//     }
// }

// impl Add<&GFnElement> for &GFnElement{
//     type Output = GFnElement;

//     fn add(self, rhs: &GFnElement) -> Self::Output {
//         let mut out = Self::Output::default();
//         let a = &self.limbs;
//         let b = &rhs.limbs;
//         let (acc0, acc1, acc2, acc3, carry) = add256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);

//         (out.limbs[0], out.limbs[1], out.limbs[2], out.limbs[3]) = sub256_conditional(acc0, acc1, acc2, acc3, carry, N0, N1, N2, N3);
//         out
//     }
// }

// impl Mul<&GFnElement> for &GFnElement{
//     type Output = GFnElement;

//     fn mul(self, rhs: &GFnElement) -> Self::Output {
//        let mut out = Self::Output::default();
//        out.from_mul(self, rhs);
//        out
//     }
// }

impl GFnElement {
    pub const N: GFnElement = GFnElement { limbs: [N0, N1, N2, N3] };

    // 2^256 % n
    pub const R: GFnElement = GFnElement { limbs: [R0, R1, R2, R3] };
    pub const ONE: GFnElement = GFnElement { limbs: [1,0,0,0]};
    #[inline]
    pub fn copy_from(&mut self, b: &GFnElement) -> &mut Self {
        *self = *b;
        self
    }

    #[inline]
    pub fn new_from_add(a: &GFnElement, b: &GFnElement) -> GFnElement {
        let mut out = *a;
        out.add(&b);
        out
    }

    #[inline]
    pub fn new_from_mul(a: &GFnElement, b: &GFnElement) -> GFnElement {
        let mut out = *a;
        out.mul(&b);
        out
    }

    #[inline]
    pub fn new_from_square(b: &GFnElement) -> GFnElement {
        let mut out = *b;
        out.square();
        out
    }
    #[inline]
    pub fn new_from_square_n(b: &GFnElement, n: usize) -> GFnElement {
        let mut out = *b;
        out.square_n(n);
        out
    }
    
    /// self = self + 0 mod N
    /// i.e., self - p if self >= p
    #[inline]
    pub fn add_zero(&mut self) -> &mut Self {
        let a = &self.limbs;
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = sub256_conditional(a[0], a[1], a[2], a[3], 0, N0, N1, N2, N3);
        self
    }


    /// self = a + b, assume a, b in [0, p)
    /// Note: the result is in [0,p) if a+b < 2p.
    #[inline]
    pub fn from_add(&mut self, a: &GFnElement, b: &GFnElement) -> &mut Self {
        self.copy_from(a).add(b)
    }

    /// self += b
    /// assume self and b are in [0,p)
    #[inline]
    pub fn add(&mut self, b: &GFnElement) -> &mut Self {
        let a = &self.limbs;
        let b = &b.limbs;
        let (acc0, acc1, acc2, acc3, carry) = add256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);

        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = sub256_conditional(acc0, acc1, acc2, acc3, carry as LIMB, N0, N1, N2, N3);
        self
    }

    #[inline]
    pub fn add_move(mut self, b: &GFnElement) -> Self {
        self.add(b);
        self
    }

    /// self = a - b mod p, assume a, b in [0,p).
    #[inline]
    pub fn from_sub(&mut self, a: &GFnElement, b: &GFnElement) -> &mut Self {
        self.from_neg(b).add(a);
        self
    }

    /// self = (self - b) mod p
    #[inline]
    pub fn sub(&mut self, b: &GFnElement) -> &mut Self {
        let b = &b.limbs;
        let (t0, t1, t2, t3, borrow) = sub256(self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3], b[0], b[1], b[2], b[3]);
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = add256_conditional(t0, t1, t2, t3, borrow, N0, N1, N2, N3);
        self
    }

    #[inline]
    pub fn sub_move(mut self, b: &GFnElement) -> Self {
        self.sub(b);
        self
    }
    /// self = p - b, assume b in [0,p).
    #[inline]
    pub fn from_neg(&mut self, b: &GFnElement) -> &mut Self {
        self.copy_from(b).neg()
    }

    /// self = p - self, assume self in [0,p).
    #[inline]
    pub fn neg(&mut self) -> &mut Self {
        let b = &self.limbs;
        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3], _) = sub256(N0, N1, N2, N3, b[0], b[1], b[2], b[3]);
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
    #[inline]
    pub fn from_mul(&mut self, a: &GFnElement, b: &GFnElement) -> &mut Self {
        self.copy_from(a).mul(b)
    }

    #[inline]
    pub fn mul(&mut self, b: &GFnElement) -> &mut Self {
        let a = &self.limbs;
        let b = &b.limbs;
        let (acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7) = mul256(a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]);
        montgomery_reduce(&mut self.limbs, acc0, acc1, acc2, acc3, acc4, acc5, acc6, acc7);
        self
    }

    #[inline]
    pub fn mul_move(mut self, b: &GFnElement) -> Self {
        self.mul(b);
        self
    }

    #[inline]
    pub fn mul_one(&mut self) -> &mut Self {
        let a0 = self.limbs[0];
        let a1 = self.limbs[1];
        let a2 = self.limbs[2];
        let a3 = self.limbs[3];

        let (a1, a2, a3, a0) = montgomery_reduce_limb(a0, a1, a2, a3);
        let (a2, a3, a0, a1) = montgomery_reduce_limb(a1, a2, a3, a0);
        let (a3, a0, a1, a2) = montgomery_reduce_limb(a2, a3, a0, a1);
        let (a0, a1, a2, a3) = montgomery_reduce_limb(a3, a0, a1, a2);

        (self.limbs[0], self.limbs[1], self.limbs[2], self.limbs[3]) = sub256_conditional(a0, a1, a2, a3, 0, N0, N1, N2, N3);
        self
    }
    // square works if a^2 < pR

    #[inline]
    pub fn from_square(&mut self, b: &GFnElement) -> &mut Self {
        self.copy_from(b).square()
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
    pub fn from_square_n(&mut self, b: &GFnElement, n: usize) -> &mut Self {
        self.copy_from(b).square_n(n)
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

    #[inline]
    pub fn invert(&mut self) -> &mut Self {
        let _10 = GFnElement::new_from_square(self);
        let _11 = GFnElement::new_from_mul(&_10, self);
        let _100 = GFnElement::new_from_mul(&_11, self);
        let _101 = GFnElement::new_from_mul(&_100, self);
        let _111 = GFnElement::new_from_mul(&_10, &_101);
        let _1001 = GFnElement::new_from_mul(&_10, &_111);
        let _1101 = GFnElement::new_from_mul(&_1001, &_100);
        let _1111 = GFnElement::new_from_mul(&_10, &_1101);
        let _11110 = GFnElement::new_from_square(&_1111);
        let _11111 = GFnElement::new_from_mul(&_11110, self);
        let _111110 = GFnElement::new_from_square(&_11111);
        let _111111 = GFnElement::new_from_mul(&_111110, &self);
        let _1111110 = GFnElement::new_from_square(&_111111);
        let i20 = GFnElement::new_from_square_n(&_1111110, 6).mul_move(&_1111110);
        let x18 = GFnElement::new_from_square_n(&i20, 5).mul_move(&_111111);
        let x31 = GFnElement::new_from_square_n(&x18, 13).mul_move(&i20).mul_move(self);
        let i42 = GFnElement::new_from_square(&x31);
        let i44 = GFnElement::new_from_square_n(&i42, 2);
        let i140 = GFnElement::new_from_square_n(&i44, 32).mul_move(&i44).square_n_move(29).mul_move(&i42).square_n_move(33);
        let i150 = GFnElement::new_from_mul(&i44, &i140).mul_move(&_111).square_n_move(4).mul_move(&_111).square_n_move(3);
        let i170 = GFnElement::new_from_mul(&i150, self).square_n_move(11).mul_move(&_1111).square_n_move(6).mul_move(&_11111);
        let i183 = GFnElement::new_from_square_n(&i170, 5).mul_move(&_1101).square_n_move(3).mul_move(&_11).square_n_move(3);
        let i198 = GFnElement::new_from_mul(&i183, self).square_n_move(7).mul_move(&_111).square_n_move(5).mul_move(&_11);
        let i219 = GFnElement::new_from_square_n(&i198, 9).mul_move(&_101).square_n_move(5).mul_move(&_101).square_n_move(5);
        let i231 = GFnElement::new_from_mul(&i219, &_1101).square_n_move(5).mul_move(&_1001).square_n_move(4).mul_move(&_1101);
        let i244 = GFnElement::new_from_square_n(&i231, 2).mul_move(&_11).square_n_move(7).mul_move(&_111111).square_n_move(2);
        let i262 = GFnElement::new_from_mul(&i244, self).square_n_move(10).mul_move(&_1001).square_n_move(5).mul_move(&_111);
        let i277 = GFnElement::new_from_square_n(&i262, 5)
            .mul_move(&_111)
            .square_n_move(4)
            .mul_move(&_101)
            .square_n_move(4)
            .mul_move(&_101)
            .square_n_move(9)
            .mul_move(&_1001)
            .square_n_move(5);
        self.mul(&i277);
        self
    }
}

// returns a/B mod n = (a + k0 * n ) >> 64
// where k0 = u*a0 mod B, u = -n^(-1) mod B
#[inline(always)]
fn montgomery_reduce_limb(a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB) -> (LIMB, LIMB, LIMB, LIMB) {
    let k0 = a0.wrapping_mul(U);
    // a + k0 * n
    let (_, t1) = mac(a0, k0, N0, 0); // a0 + k0 * N0
    let (a1, t2) = mac(a1, k0, N1, t1); //a1 + (a1 * N1) + t1

    // let (a2, t3) = mac(a2, k0, N2, t2);
    // N2 = B-1 and t2 + a2 + k0 * N2 = t2 + a2 - k0 +  k0*B
    let (a2, carry) = adc(a2, t2, false);
    let (t3, _) = adc(k0, 0, carry);
    let (a2, borrow) = sbb(a2, k0, false);
    let (t3, _) = sbb(t3, 0, borrow);

    // let (a3, a0) = mac(a3, k0, N3, t3);
    // Note N3 = B - 2^{32} - 1
    // and a3 + t3 + k0 * N3 = a3 + t3 + k0*B - k0 * 2^{32} - k0
    // = a3 + t3 - (k0_lo << 32) - k0 + (k0 - k0_hi)*B
    let lo = k0 << 32;
    let hi = k0 >> 32;
    let (a3, carry) = adc(a3, t3, false);
    let (a0, _) = adc(k0, 0, carry);
    let (a3, borrow) = sbb(a3, lo, false);
    let (a0, _) = sbb(a0, hi, borrow);
    let (a3, borrow) = sbb(a3, k0, false);
    let (a0, _) = sbb(a0, 0, borrow);

    (a1, a2, a3, a0)
}

#[inline(always)]
fn montgomery_reduce(out: &mut [LIMB; 4], a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB, a4: LIMB, a5: LIMB, a6: LIMB, a7: LIMB) {
    let (a1, a2, a3, a0) = montgomery_reduce_limb(a0, a1, a2, a3);
    let (a2, a3, a4, a1) = montgomery_reduce_limb(a1, a2, a3, a4);
    let (a3, a4, a5, a2) = montgomery_reduce_limb(a2, a3, a4, a5);
    let (a4, a5, a6, a3) = montgomery_reduce_limb(a3, a4, a5, a6);

    let (acc0, acc1, acc2, acc3, carry) = add256(a0, a1, a2, a3, a4, a5, a6, a7);
    (out[0], out[1], out[2], out[3]) = sub256_conditional(acc0, acc1, acc2, acc3, carry as LIMB, N0, N1, N2, N3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    use num::BigUint;
    use rand::Rng;
    use std::time::SystemTime;

    fn random() -> GFnElement {
        let mut rng = rand::rng();
        let n = &GFnElement::N.limbs;
        let (a0, a1, a2, a3) = (rng.random(), rng.random(), rng.random(), rng.random());
        let (a0, a1, a2, a3) = sub256_conditional(a0, a1, a2, a3, 0, n[0], n[1], n[2], n[3]);
        GFnElement { limbs: [a0, a1, a2, a3] }
    }

    fn get_module() -> BigUint {
        BigUint::from_bytes_be(hex!("fffffffeffffffffffffffffffffffff7203df6b21c6052b53bbf40939d54123").as_slice())
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

    #[test]
    fn test_montgomery_reduce() {
        let mut rng = rand::rng();
        let mut rinv = BigUint::from_slice(&[1]);
        rinv <<= 256;
        rinv = rinv.modinv(&get_module()).unwrap();

        // for _ in 0..1000000 {
        let mut i: u128 = 0;
        while i < 100 * 10000 * 10000 {
            if i % (100 * 1000 * 1000) == 0 {
                println!("test: {}亿", i / (100 * 1000 * 1000));
            }
            i += 1;
            let data: [u64; 8] = [rng.random(), rng.random(), rng.random(), rng.random(), rng.random(), rng.random(), rng.random(), rng.random()];
            let mut a = [0; 4];
            montgomery_reduce(&mut a, data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]);
            let a = to_bigint(&a);

            let aa = (to_bigint(&data) * &rinv) % get_module();
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
    fn test_mul_fuzz() {
        let rinv = to_bigint(&GFnElement::R.limbs).modinv(&get_module()).unwrap();
        for _ in 0..100000 {
            let a = random();
            let b = random();
            let c = a.clone().mul_move(&b); // a*b
            let c = to_bigint(&c.limbs);
            let aa = to_bigint(&a.limbs);
            let bb = to_bigint(&b.limbs);
            let cc = aa * bb * &rinv % get_module();
            assert_eq!(c, cc);
        }
    }

    #[test]
    fn test_invert() {
        let mut c = GFnElement::default();
        for _ in 0..1000 {
            let a = random();
            c.copy_from(&a).invert().mul(&a);
            assert_eq!(c, GFnElement::R);
        }
    }

    #[test]
    fn test_invert_speed() {
        // 132k
        let a = random();
        let mut c = a;

        let loops = 1000000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            c.invert();
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", c);
        println!("{}K TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }
}
