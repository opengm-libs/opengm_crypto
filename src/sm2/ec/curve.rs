use core::fmt::Display;

#[cfg(feature = "std")]
use std::thread::{self, spawn};

#[cfg(test)]
use std::time::SystemTime;

use crate::sm2::U256;

use super::gfp::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AffinePoint {
    pub x: GFpElement,
    pub y: GFpElement,
    pub infinity: bool,
}

impl Default for AffinePoint {
    fn default() -> Self {
        Self::INFINITY
    }
}

impl AffinePoint {
    const INFINITY: AffinePoint = AffinePoint {
        x: GFpElement { limbs: [0, 0, 0, 0] },
        y: GFpElement { limbs: [0, 0, 0, 0] },
        infinity: true,
    };

    //The base point G in montgomery representation.
    const BASE: AffinePoint = AffinePoint {
        x: GFpElement {
            limbs: [0x61328990f418029e, 0x3e7981eddca6c050, 0xd6a1ed99ac24c3c3, 0x91167a5ee1c13b05],
        },
        y: GFpElement {
            limbs: [0xc1354e593c2d0ddd, 0xc1f5e5788d3295fa, 0x8d4cfb066e2a48f8, 0x63cd65d481d735bd],
        },
        infinity: false,
    };

    pub fn new(x: U256, y: U256)-> AffinePoint {
        AffinePoint{
            x: GFpElement::from(x),
            y: GFpElement::from(y),
            infinity: false,
        }
    }

    pub fn new_from_scalar_base_mul(scalar: &[u64;4])-> AffinePoint{
        JacobianPoint::new_from_scalar_base_mul(scalar).into()
    }

    pub fn equal(&self, other: &AffinePoint) -> bool {
        if self.infinity && self.infinity {
            return true;
        }
        return self.x == other.x && self.y == other.y;
    }

    pub fn scalar_mul(&mut self, scalar: &[u64; 4]) -> &mut Self {
        let mut p:JacobianPoint = (*self).into();
        p.scalar_mul(scalar);
        *self = p.into();
        self
    }

    pub fn scalar_base_mul(&mut self, scalar: &[u64; 4]) -> &mut Self {
        let mut p:JacobianPoint = (*self).into();
        p.scalar_mul(scalar);
        *self = p.into();
        self
    }
}

impl From<&JacobianPoint> for AffinePoint {
    fn from(jp: &JacobianPoint) -> Self {
        match jp.is_infinity() {
            true => AffinePoint::INFINITY,
            false => {
                let zinv = GFpElement::new_from_invert(&jp.z);
                let zinv2 = GFpElement::new_from_square(&zinv);
                let x = GFpElement::new_from_mul(&jp.x, &zinv2);
                let zinv3 = zinv2.mul_move(&zinv);
                let y = GFpElement::new_from_mul(&jp.y, &zinv3);
                AffinePoint { x: x, y: y, infinity: false }
            }
        }
    }
}

impl From<JacobianPoint> for AffinePoint {
    fn from(jp: JacobianPoint) -> Self {
        match jp.is_infinity() {
            true => AffinePoint::INFINITY,
            false => {
                let zinv = GFpElement::new_from_invert(&jp.z);
                let zinv2 = GFpElement::new_from_square(&zinv);
                let x = GFpElement::new_from_mul(&jp.x, &zinv2);
                let zinv3 = zinv2.mul_move(&zinv);
                let y = GFpElement::new_from_mul(&jp.y, &zinv3);
                AffinePoint { x: x, y: y, infinity: false }
            }
        }
    }
}

impl From<(U256, U256)> for AffinePoint {
    fn from(value: (U256, U256)) -> Self {
        AffinePoint { 
            x: value.0.into(), 
            y: value.1.into(), 
            infinity: false,
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub struct JacobianPoint {
    pub x: GFpElement,
    pub y: GFpElement,
    pub z: GFpElement,
}

impl Default for JacobianPoint {
    fn default() -> Self {
        Self::INFINITY
    }
}

impl From<&AffinePoint> for JacobianPoint {
    fn from(value: &AffinePoint) -> Self {
        JacobianPoint::from_affine(value)
    }
}

impl From<AffinePoint> for JacobianPoint {
    fn from(value: AffinePoint) -> Self {
        JacobianPoint::from_affine(&value)
    }
}
impl From<(U256, U256)> for JacobianPoint {
    fn from(value: (U256, U256)) -> Self {
        JacobianPoint { 
            x: value.0.into(), 
            y: value.1.into(), 
            z: GFpElement::R,
        }
    }
}



impl Display for JacobianPoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "x: {}\ny: {}\nz: {}\n", self.x, self.y, self.z)
    }
}

const TABLE_SIZE: usize = 32768 * 15 + 65536;
fn init_base_table() {
    #[cfg(not(feature = "std"))]
    init_table_st();

    #[cfg(feature = "std")]
    init_table_mt();
}

static mut TABLE: [AffinePoint; TABLE_SIZE] = [AffinePoint::INFINITY; TABLE_SIZE];
fn init_table_st() {
    #[cfg(test)]
    let now = SystemTime::now();

    let mut p = JacobianPoint::BASE;
    let mut q;
    let mut tbl_index = 0;
    #[allow(static_mut_refs)]
    let tbl = unsafe { &mut TABLE };
    for _ in 0..15 {
        q = p;

        tbl[tbl_index] = AffinePoint::from(&p);
        tbl_index += 1;
        q.from_double(&p);
        tbl[tbl_index] = AffinePoint::from(&q);
        tbl_index += 1;

        for _ in 3..=32768 {
            q.add(&p);
            tbl[tbl_index] = AffinePoint::from(&q);
            tbl_index += 1;
        }
        p.from_double(&q);
    }
    q = p;

    tbl[tbl_index] = AffinePoint::from(&p);
    tbl_index += 1;
    q.from_double(&p);
    tbl[tbl_index] = AffinePoint::from(&q);
    tbl_index += 1;

    for _ in 3..=65536 {
        q.add(&p);
        tbl[tbl_index] = AffinePoint::from(&q);
        tbl_index += 1;
    }

    #[cfg(test)]
    println!("init sm2 base scale mul table used {:?} seconds", now.elapsed().unwrap().as_secs_f64());
}

#[cfg(feature = "std")]
fn init_table_mt() {
    #[cfg(test)]
    let now = SystemTime::now();

    #[allow(static_mut_refs)]
    let tbl = unsafe { &mut TABLE };

    // G, [65536^3]G, [65536^6]G, [65536^9]G, [65536^12]G, [65536^15]G
    const START_POINTS: [JacobianPoint; 6] = [
        //G
        JacobianPoint {
            x: GFpElement {
                limbs: [0x61328990f418029e, 0x3e7981eddca6c050, 0xd6a1ed99ac24c3c3, 0x91167a5ee1c13b05],
            },
            y: GFpElement {
                limbs: [0xc1354e593c2d0ddd, 0xc1f5e5788d3295fa, 0x8d4cfb066e2a48f8, 0x63cd65d481d735bd],
            },
            z: GFpElement::R,
        },
        //[256^6]G
        JacobianPoint {
            x: GFpElement {
                limbs: [0x8487eb9068755cf3, 0x1887394e7fe12541, 0x2e4c65d446af8ca8, 0x72aae645b9e119dc],
            },
            y: GFpElement {
                limbs: [0x958e00941ec6ad73, 0x84a7eec48ce4573e, 0x3d6d00d4f9254b96, 0x4ef44f588e421732],
            },
            z: GFpElement::R,
        },
        //[256^12]G
        JacobianPoint {
            x: GFpElement {
                limbs: [0x4599b8941abd31f0, 0xdb34198d9a1da7d3, 0xa8b89523a0f0217d, 0x2014cc43e56b884e],
            },
            y: GFpElement {
                limbs: [0x6fb94f8849efd4ee, 0xf1b81710287f4ae0, 0x89d38a9a99fd2deb, 0x8179277a72b67a53],
            },
            z: GFpElement::R,
        },
        //[256^18]G
        JacobianPoint {
            x: GFpElement {
                limbs: [0xee0315879c34971b, 0x5829eb07e76545cf, 0xb7a3a6ae33a81bb9, 0xff42daff49c9f710],
            },
            y: GFpElement {
                limbs: [0x894eae85bffb951b, 0x815fe3e2ce70f324, 0x636564cb428b1f12, 0x722e0050a029b0bd],
            },
            z: GFpElement::R,
        },
        //[256^24]G
        JacobianPoint {
            x: GFpElement {
                limbs: [0xfb3992a4202bde39, 0x2549f5643d6bab98, 0x0b56464287712512, 0xd52442b47fde7e50],
            },
            y: GFpElement {
                limbs: [0xa6cefd08a3d3e16e, 0x5b194f0ac83b29bd, 0x6db0edd8906dec8c, 0x7a09095902570c1e],
            },
            z: GFpElement::R,
        },
        //[256^30]G
        JacobianPoint {
            x: GFpElement {
                limbs: [0x6f154f09a9e0eeae, 0x2246e6feab05a657, 0x4d7c1c811045b85d, 0xde99ea37d3bb7432],
            },
            y: GFpElement {
                limbs: [0x058f818763184ff4, 0x2a223421d134bfc3, 0x1560dbed23120320, 0x37243c9576a3de9c],
            },
            z: GFpElement::R,
        },
    ];
    let mut threads = std::vec::Vec::new();
    const N: usize = 3 * 32768;
    let (chunks, tail) = tbl.as_chunks_mut::<N>();
    for (i, tbl) in chunks.iter_mut().enumerate() {
        let start_point = START_POINTS[i];
        threads.push(spawn(move || {
            let mut p = start_point;
            let mut q = JacobianPoint::default();
            let mut tbl_index = 0;
            for _ in 0..3 {
                tbl[tbl_index] = AffinePoint::from(&p);
                tbl_index += 1;
                q.from_double(&p);
                tbl[tbl_index] = AffinePoint::from(&q);
                tbl_index += 1;

                for _ in 3..=32768 {
                    q.add(&p);
                    tbl[tbl_index] = AffinePoint::from(&q);
                    tbl_index += 1;
                }
                p.from_double(&q);
            }
        }));
    }
    let p = START_POINTS[5];
    let mut q = p;
    let mut tail_index = 0;
    tail[tail_index] = AffinePoint::from(&p);
    tail_index += 1;

    q.from_double(&p); // q = 2p

    tail[tail_index] = AffinePoint::from(&q);
    tail_index += 1;
    for _ in 3..=65536 {
        q.add(&p);
        tail[tail_index] = AffinePoint::from(&q);
        tail_index += 1;
    }

    let mut success = true;
    for t in threads {
        if let Err(_) = t.join() {
            success = false;
        }
    }

    // when unwrap() returns error, use init_table_st.
    if !success {
        init_table_st();
    }

    #[cfg(test)]
    println!("init sm2 base scale mul table used {:?} seconds", now.elapsed().unwrap().as_secs_f64());
}

impl JacobianPoint {
    const INFINITY: JacobianPoint = JacobianPoint {
        x: GFpElement::ZERO,
        y: GFpElement::ZERO,
        // y: GFpElement::R,
        z: GFpElement::ZERO,
    };
    const BASE: JacobianPoint = JacobianPoint {
        x: GFpElement {
            limbs: [0x61328990f418029e, 0x3e7981eddca6c050, 0xd6a1ed99ac24c3c3, 0x91167a5ee1c13b05],
        },
        y: GFpElement {
            limbs: [0xc1354e593c2d0ddd, 0xc1f5e5788d3295fa, 0x8d4cfb066e2a48f8, 0x63cd65d481d735bd],
        },
        z: GFpElement::R,
    };
    #[inline]
    pub fn is_infinity(&self) -> bool {
        self.z.is_zero()
    }

    #[inline]
    fn copy_from(&mut self, other: &JacobianPoint) -> &mut Self {
        *self = *other;
        self
    }

    pub const fn from_affine(p: &AffinePoint) -> Self {
        match p.infinity {
            true => JacobianPoint::INFINITY,
            false => JacobianPoint { x: p.x, y: p.y, z: GFpElement::R },
        }
    }

    /// Normalize the points to z = 1, or (0,0,0) if z = 0.
    ///
    /// Note that instead (0,1,0), we returns (0,0,0) when z = 0 (to void the if z.is_zero()
    /// conditional part), which is irrelevante - we consider a point be infinity only
    /// if z = 0.
    #[inline]
    pub fn normalize(&mut self) {
        // x' = x / z^2
        // y' = y / z^3
        let zinv = GFpElement::new_from_invert(&self.z);
        let zinv2 = GFpElement::new_from_square(&zinv);
        self.x.mul(&zinv2);
        let zinv3 = zinv2.mul_move(&zinv);
        self.y.mul(&zinv3);
        self.z.mul(&zinv);
    }

    /// Return's the affine coordinate of the point.
    /// Returns None only if the point is infinite.
    #[inline]
    pub fn get_affine_x(&self) -> Option<GFpElement> {
        match self.is_infinity() {
            true => None,
            false => Some(GFpElement::new_from_invert2(&self.z).mul_move(&self.x)),
        }
    }

    #[inline]
    pub fn new_from_scalar_base_mul(scalar: &[u64; 4]) -> Self {
        crate::once_or!(
            {
                init_base_table();
            },
            {
                let mut res = JacobianPoint::BASE;
                res.scalar_mul(scalar);
                res
            },
            {
                #[allow(static_mut_refs)]
                let base_table = unsafe { &TABLE };
                let mut res = Self::default();
                let mut zero = 0;
                let mut sel;
                let mut sign = 0;
                let mut tbl_index = 0;
                for i in 0..15 {
                    sel = ((scalar[i / 4] >> (16 * (i % 4))) & 0xffff) + sign;
                    sign = match sel > 32768 {
                        true => 1,
                        false => 0,
                    };
                    sel = match sel > 32768 {
                        true => 65536 - sel,
                        false => sel,
                    };
                    let p = match sel == 0 {
                        true => &AffinePoint::INFINITY,
                        false => &base_table[tbl_index + (sel as usize) - 1],
                    };
                    res.add_affine(p, sign, sel, zero);
                    zero |= sel;
                    tbl_index += 32768;
                }
                sel = (scalar[3] >> 48) + sign;
                let p = match sel == 0 {
                    true => &AffinePoint::INFINITY,
                    false => &base_table[tbl_index + (sel as usize) - 1],
                };
                // assert_ne!(&AffinePoint::from(&res), p);
                res.add_affine(p, 0, sel, zero);
                res
            }
        );
    }

    #[inline]
    fn add_affine(&mut self, p2: &AffinePoint, sign: u64, sel: u64, zero: u64) {
        let mut jp2 = JacobianPoint::from(p2);
        if sel == 0 {
            return;
        }
        if sign != 0 {
            jp2.y.neg();
        }
        if zero == 0 {
            *self = jp2;
            return;
        }
        self.add(&jp2);
    }

    /// self = p + q if p != q.
    /// Returns true if p == q, false otherwise.
    #[inline]
    pub fn from_add(&mut self, p: &JacobianPoint, q: &JacobianPoint) -> bool {
        self.copy_from(p).add(&q)
    }

    #[inline]
    pub fn add(&mut self, p: &JacobianPoint) -> bool {
        let (x1, y1, z1) = (&p.x, &p.y, &p.z);
        let (x2, y2, z2) = (&mut self.x, &mut self.y, &mut self.z);

        let z2sqr = GFpElement::new_from_square(z2);
        let s1 = GFpElement::new_from_mul(z2, &z2sqr).mul_move(y1);
        let z1sqr = GFpElement::new_from_square(z1);
        let s2 = GFpElement::new_from_mul(z1, &z1sqr).mul_move(y2);
        let r = GFpElement::new_from_sub(&s2, &s1);
        let u1 = GFpElement::new_from_mul(&z2sqr, x1);
        let u2 = GFpElement::new_from_mul(&z1sqr, x2);
        let h = GFpElement::new_from_sub(&u2, &u1);

        let point_eq = r.is_zero() & h.is_zero();

        let rsqr = GFpElement::new_from_square(&r);
        let hsqr = GFpElement::new_from_square(&h);
        let hcub = GFpElement::new_from_mul(&hsqr, &h);
        let s1hcub = GFpElement::new_from_mul(&hcub, &s1);
        z2.mul(z1).mul(&h);
        let u2 = GFpElement::new_from_mul(&hsqr, &u1);
        let u2x2 = GFpElement::new_from_add(&u2, &u2);
        x2.from_sub(&rsqr, &u2x2).sub(&hcub);
        y2.from_sub(&u2, x2).mul(&r).sub(&s1hcub);

        point_eq
    }

    #[inline]
    pub fn from_double(&mut self, p: &JacobianPoint) {
        self.copy_from(p).double()
    }
    #[inline]
    pub fn double(&mut self) {
        let (x, y, z) = (&mut self.x, &mut self.y, &mut self.z);

        let zsqr = GFpElement::new_from_square(&z);
        z.mul(&y).double();
        let m = GFpElement::new_from_add(&zsqr, &x).mul_move(&GFpElement::new_from_sub(&x, &zsqr));
        let m = m.add_move(&GFpElement::new_from_double(&m));
        let double_ysqr = GFpElement::new_from_square(&y).double_move(); // 2y^2;
        let mut s = GFpElement::new_from_double(&double_ysqr).mul_move(&x); // 4xy^2
        y.from_square(&double_ysqr).double(); // 8y^4
        x.from_square(&m).sub(&s).sub(&s);
        y.sub(s.sub(x).mul(&m)).neg();
    }

    // The double-and-add method.
    // Just for testing.
    #[inline]
    #[cfg(test)]
    pub fn scalar_mul_naive(&mut self, scalar: &[u64; 4]) {
        let mut p = Self::INFINITY;
        for i in [3, 2, 1, 0] {
            for j in 0..64 {
                p.double();
                let bit = (scalar[i] >> (63 - j)) & 1;
                if bit == 1 {
                    if p.is_infinity() {
                        p = *self;
                    } else {
                        p.add(self);
                    }
                }
            }
        }
        self.copy_from(&p);
    }

    // self = [scalar] * self
    // scalar为小端表示的256bit integer.
    // adc: 92571
    // sbb: 90707
    // mac: 40952
    // How about use windows size = 6?
    #[inline]
    pub fn scalar_mul(&mut self, scalar: &[u64; 4]) -> &mut Self {
        // let p = self;
        let mut precomp = [JacobianPoint::default(); 16];
        let mut t0 = JacobianPoint::default();
        let mut t1 = JacobianPoint::default();
        let mut t2 = JacobianPoint::default();
        let mut t3 = JacobianPoint::default();

        precomp[0] = *self; // 1
        t0.from_double(self);
        t1.from_double(&t0);
        t2.from_double(&t1);
        t3.from_double(&t2);

        precomp[1] = t0; // 2
        precomp[3] = t1; // 4
        precomp[7] = t2; // 8
        precomp[15] = t3; // 16

        t0.add(self);
        t1.add(self);
        t2.add(self);
        precomp[2] = t0; // 3
        precomp[4] = t1; // 5
        precomp[8] = t2; // 9

        t0.double();
        t1.double();
        precomp[5] = t0; // 6
        precomp[9] = t1; // 10

        t2.from_add(&t0, self);
        t1.add(self);
        precomp[6] = t2; // 7
        precomp[10] = t1; // 11

        t0.double();
        t2.double();
        precomp[11] = t0; // 12
        precomp[13] = t2; // 14

        t0.add(self);
        t2.add(self);
        precomp[12] = t0; // 13
        precomp[14] = t2; // 15

        /*
            从高位到低位，每次处理5bit
            255 | 254 ... 250 | 249 ... 245 | ... | 4 ... 0 |
                 index
            每次额外读取下一个5bit的首位，如果是1，则多加一个p.
        */
        let mut index = 254;
        let mut wvalue = (scalar[index / 64] >> (index % 64)) & 0x3f;
        let (sel, _) = booth::<5>(wvalue);
        let mut p = point_select(&precomp, sel as usize);
        let mut zero = sel;
        while index > 4 {
            index -= 5;
            p.double();
            p.double();
            p.double();
            p.double();
            p.double();

            // Note: C 里面 a<<64 等价于 a<<0
            // go 里面 a<<64等价于a = 0
            if index < 192 && index != 64 {
                wvalue = ((scalar[index / 64] >> (index % 64)) + (scalar[index / 64 + 1] << (64 - (index % 64)))) & 0x3f;
            } else {
                wvalue = (scalar[index / 64] >> (index % 64)) & 0x3f;
            }

            let (sel, sign) = booth::<5>(wvalue);

            let mut t0 = point_select(&precomp, sel as usize);

            // c256NegCond(&t0[4], sign);
            if sign == 1 {
                t0.y.neg();
            }

            t1.from_add(&p, &t0);

            // pointMovCond(t1, t1, q, sel);
            if sel == 0 {
                t1 = p;
            }

            // pointMovCond(q, t1, t0, zero);
            p = match zero == 0 {
                true => t0,
                false => t1,
            };
            zero |= sel;
        }
        p.double();
        p.double();
        p.double();
        p.double();
        p.double();

        wvalue = (scalar[0] << 1) & 0x3f;
        let (sel, sign) = booth::<5>(wvalue);

        let mut t0 = point_select(&precomp, sel as usize);

        // c256NegCond(&t0[4], sign);
        if sign == 1 {
            t0.y.neg();
        }

        t1.from_add(&p, &t0);

        // pointMovCond(t1, t1, q, sel);
        if sel == 0 {
            t1 = p;
        }

        // pointMovCond(q, t1, t0, zero);
        *self = match zero == 0 {
            true => t0,
            false => t1,
        };
        self
    }
}

#[inline]
fn booth<const N: usize>(input: u64) -> (u64, u64) {
    let s = !((input >> N).wrapping_sub(1));
    let d = (1u64 << (N + 1)).wrapping_sub(input).wrapping_sub(1);
    let d = (d & s) | (input & (!s));
    let d = (d >> 1) + (d & 1);
    let s = s & 1;
    (d, s)
}

#[inline]
fn point_select(precomp: &[JacobianPoint; 16], i: usize) -> JacobianPoint {
    match i == 0 {
        true => JacobianPoint::INFINITY,
        false => precomp[i - 1],
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;
    use std::thread::spawn;
    use std::time::SystemTime;
    use std::vec::Vec;

    use rand::Rng;

    use super::AffinePoint;
    use super::GFpElement;
    use super::JacobianPoint;

    fn get_test_points() -> (JacobianPoint, JacobianPoint, JacobianPoint) {
        // G - affine
        let mut g1 = JacobianPoint {
            x: GFpElement {
                limbs: [0x715A4589334C74C7, 0x8FE30BBFF2660BE1, 0x5F9904466A39C994, 0x32C4AE2C1F198119],
            },
            y: GFpElement {
                limbs: [0x02DF32E52139F0A0, 0xD0A9877CC62A4740, 0x59BDCEE36B692153, 0xBC3736A2F4F6779C],
            },
            z: GFpElement { limbs: [1, 0, 0, 0] },
        };

        let mut g2: JacobianPoint = JacobianPoint {
            x: GFpElement {
                limbs: [0x495c2e1da3f2bd52, 0x9c0dfa08c08a7331, 0x0d58ef57fa73ba4d, 0x56cefd60d7c87c00],
            },
            y: GFpElement {
                limbs: [0x6f780d3a970a23c3, 0x6de84c182f6c8e71, 0x68535ce0f8eaf1bd, 0x31b7e7e6cc8189f6],
            },
            z: GFpElement { limbs: [1, 0, 0, 0] },
        };
        let mut g3: JacobianPoint = JacobianPoint {
            x: GFpElement {
                limbs: [0xe26918f1d0509ebf, 0xa13f6bd945302244, 0xbe2daa8cdb41e24c, 0xa97f7cd4b3c993b4],
            },
            y: GFpElement {
                limbs: [0xaaacdd037458f6e6, 0x7c400ee5cd045292, 0xccc5cec08a72150f, 0x530b5dd88c688ef5],
            },
            z: GFpElement { limbs: [1, 0, 0, 0] },
        };
        g1.x.transform_to_mont();
        g1.y.transform_to_mont();
        g1.z.transform_to_mont();

        g2.x.transform_to_mont();
        g2.y.transform_to_mont();
        g2.z.transform_to_mont();

        g3.x.transform_to_mont();
        g3.y.transform_to_mont();
        g3.z.transform_to_mont();
        (g1, g2, g3)
    }

    #[test]
    fn test_point_add() {
        let (g1, g2, g3) = get_test_points();
        let mut triple = g1;
        let eq = triple.add(&g2);
        let triple = AffinePoint::from(&triple);
        assert_eq!(eq, false);
        assert_eq!(triple, AffinePoint::from(&g3));
    }

    #[test]
    fn test_point_double() {
        let (g1, g2, g3) = get_test_points();
        let mut p = JacobianPoint::default();
        p.from_double(&g1);
        let ap = AffinePoint::from(&p);
        assert_eq!(ap, (&g2).into());

        p.from_add(&g3, &g1);
        let ap = AffinePoint::from(&p);
        let mut q = JacobianPoint::default();
        q.from_double(&g2);
        assert_eq!(ap, AffinePoint::from(&q));
    }

    extern crate test;
    #[test]
    fn test_point_add_bench() {
        let (g1, g2, _) = get_test_points();
        let mut triple = JacobianPoint::default();

        let loops = 10000000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(triple.from_add(&g1, &g2));
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", triple);
        println!("{}k TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }

    #[test]
    fn test_point_double_bench() {
        let (g1, _g2, _) = get_test_points();
        let mut p = JacobianPoint::default();

        let loops = 10000000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(p.from_double(&g1));
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", p);
        println!("{}k TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }

    #[test]
    fn test_scalar_mul() {
        let (g1, _, _) = get_test_points();

        // N=FFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFF7203DF6B21C6052B53BBF40939D54123
        // let scalar = [0x53BBF40939D54123 + 16094, 0x7203DF6B21C6052B, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF];
        let scalar = [0x937c716d4119321f, 0xee98b34dca313edd, 0x9b814363cce92038, 0x5ab217f8013ca70e];
        let r = JacobianPoint::new_from_scalar_base_mul(&scalar);

        let mut p = g1;
        p.scalar_mul(&scalar);
        assert_eq!(AffinePoint::from(&r), AffinePoint::from(&p));
    }

    #[test]
    fn test_scalar_base_mul() {
        let (_, _, g3) = get_test_points();
        let scalar = [0x53BBF40939D54126, 0x7203DF6B21C6052B, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF];
        let p = JacobianPoint::new_from_scalar_base_mul(&scalar);
        assert_eq!(AffinePoint::from(&p), AffinePoint::from(&g3));
    }

    #[test]
    fn test_scalar_mul_fuzz() {
        let mut tasks: Vec<_> = Vec::new();
        for _ in 0..16 {
            tasks.push(spawn(|| {
                let (g1, _, _) = get_test_points();
                let mut rng = rand::rng();
                let mut p0 = JacobianPoint::default();
                let mut p1;
                let mut p2 = JacobianPoint::default();

                for _ in 0..100000 {
                    let scalar = [rng.random(), rng.random(), rng.random(), rng.random()];
                    p0.copy_from(&g1).scalar_mul_naive(&scalar);
                    p1 = JacobianPoint::new_from_scalar_base_mul(&scalar);
                    p2.copy_from(&g1).scalar_mul(&scalar);
                    let ap0 = AffinePoint::from(&p0);
                    let ap1 = AffinePoint::from(&p1);
                    let ap2 = AffinePoint::from(&p2);

                    if ap0 != ap1 {
                        panic!("scalar base mul: [0x{:016x}, 0x{:016x}, 0x{:016x}, 0x{:016x}]", scalar[0], scalar[1], scalar[2], scalar[3]);
                    }
                    if ap0 != ap2 {
                        panic!("scalar mul: [0x{:016x}, 0x{:016x}, 0x{:016x}, 0x{:016x}]", scalar[0], scalar[1], scalar[2], scalar[3]);
                    }
                }
            }));
        }
        for t in tasks {
            t.join().unwrap();
        }
    }

    #[test]
    fn test_point_scalar_mul_bench() {
        let (g1, _g2, _) = get_test_points();
        let mut p = g1;
        let scalar = [0x53BBF40939D54124, 0x7203DF6B21C6052B, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF];
        let loops = 100000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(p.scalar_mul(&scalar));
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", p);
        println!("{}k TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }

    #[test]
    fn test_point_scalar_base_mul_bench() {
        let mut p;
        let mut scalar = [0x53BBF40939D54124, 0x7203DF6B21C6052B, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFEFFFFFFFF];
        test::black_box(p = JacobianPoint::new_from_scalar_base_mul(&scalar));
        // wait for the base table finish initialization.
        std::thread::sleep(Duration::from_secs(5));

        let loops = 1000000u64;
        let mut q:AffinePoint = p.into();
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(q = JacobianPoint::new_from_scalar_base_mul(&scalar).into());
            scalar[0] += 1;
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        println!("{:?}", p);
        println!("{}k TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }

    #[test]
    fn test_add_affine() {
        let mut q = JacobianPoint {
            x: GFpElement {
                limbs: [0xb8cfa37cd19f73a7, 0x2911a5e7f983c084, 0x9f7f235f1dba5c61, 0x6f4cdadb8fa72d0e],
            },
            y: GFpElement {
                limbs: [0x83e7fe894bdf2c5a, 0x7cff504a3ff3371c, 0x66af53c173a8721c, 0xafc8d923d0f90f04],
            },
            z: GFpElement::R,
        };
        let p = AffinePoint {
            x: GFpElement {
                limbs: [0x0034a9e946188f17, 0x1e8eb884e9174e64, 0x916602d7cb7a8acf, 0x6be3c8914ad03eb4],
            },
            y: GFpElement {
                limbs: [0xdb0a0617bd0a1b14, 0xe9416f9ebcd7c018, 0x3342ed0d3105fb41, 0x636bb8feb9f4042a],
            },
            infinity: false,
        };

        // p.add_affine(&q, 0, 1, 1);
        q.add(&JacobianPoint::from_affine(&p));
        q.normalize();
        // 0x565d865a900f4d16, 0xd9ab3bbcc979f23b, 0x6fb81b9c2fe33773, 0x701792542160725d,
        // 0x7834979092d87521, 0x9f7a416c98e0ab8e, 0x96aa697133d10486, 0x5869b08683e1cc49
        println!("res.x: {}", &q.x);
        println!("res.y: {}", &q.y);
        println!("res.z: {}", &q.z);

        // 0x151369bb19990981
        // 0x45b3fc611b85da09
        // 0x7ea14e3427e76f55
        // 0x209ba5cf0c4928c3
    }
}
