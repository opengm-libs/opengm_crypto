use crate::sm3;

use super::ec::gfn::*;
use super::*;
use subtle::ConstantTimeEq;

#[derive(Copy, Clone, Default, Debug)]
pub struct Signature {
    pub r: U256,
    pub s: U256,
}

fn d1invert(d: &mut GFnElement) -> &mut GFnElement {
    d.add(&GFnElement::ONE) //d+1
        .invert() // 1/(d+1)/R) = R/(1+d)
        .mul_one()
}

pub fn sign<T: rand::RngCore>(e: &[u8;32], d: &PrivateKey, rnd: &mut T)-> Option<Signature>{
    let k: [u64;4] = rnd.random();
    sign_inner(e, d, &k)
}

fn sign_inner(e: &[u8; 32], d: &PrivateKey, k: &[u64; 4]) -> Option<Signature> {
    let e = GFnElement::from(e);
    let mut x = JacobianPoint::new_from_scalar_base_mul(k).get_affine_x()?;
    x.transform_from_mont();
    let x = GFnElement::from(&x);

    let r = x.add_move(&e);
    let mut d1inv = GFnElement::from(d.d);
    d1invert(&mut d1inv);

    // if passing in 1/(1+d), then the speed will lift off.
    // let d1inv = *d;

    let mut s = GFnElement::from(k);
    s.add(&r).mul(&d1inv).sub(&r);
    Some(Signature { r: r.limbs.into(), s: s.limbs.into() })
}

pub fn verify(e: &[u8; 32], pk: &PublicKey, sig: &Signature) -> bool {
    let e = GFnElement::from(e);
    let r = GFnElement::from(sig.r);
    let s = GFnElement::from(sig.s);
    // t = r+s
    // let mut t:GFnElement = r;
    // t.add(&s);
    let t = GFnElement::new_from_add(&s, &r);
    let mut p = JacobianPoint::from((pk.x, pk.y));

    // p = [s]G + [t]PK
    p.scalar_mul(&t.limbs);
    p.add(&JacobianPoint::new_from_scalar_base_mul(&s.limbs));
    let x1 = GFnElement::from(&*p.get_affine_x().unwrap().transform_from_mont()).add_move(&e);

    x1.limbs.ct_eq(&r.limbs).into()
}

const ABG: [u8; 128] = [
    /* a */
    0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFC,
    /* b */
    0x28, 0xE9, 0xFA, 0x9E, 0x9D, 0x9F, 0x5E, 0x34, 0x4D, 0x5A, 0x9E, 0x4B, 0xCF, 0x65, 0x09, 0xA7, 0xF3, 0x97, 0x89, 0xF5, 0x15, 0xAB, 0x8F, 0x92, 0xDD, 0xBC, 0xBD, 0x41, 0x4D, 0x94, 0x0E, 0x93,
    /* Gx */
    0x32, 0xC4, 0xAE, 0x2C, 0x1F, 0x19, 0x81, 0x19, 0x5F, 0x99, 0x04, 0x46, 0x6A, 0x39, 0xC9, 0x94, 0x8F, 0xE3, 0x0B, 0xBF, 0xF2, 0x66, 0x0B, 0xE1, 0x71, 0x5A, 0x45, 0x89, 0x33, 0x4C, 0x74, 0xC7,
    /* Gy */
    0xBC, 0x37, 0x36, 0xA2, 0xF4, 0xF6, 0x77, 0x9C, 0x59, 0xBD, 0xCE, 0xE3, 0x6B, 0x69, 0x21, 0x53, 0xD0, 0xA9, 0x87, 0x7C, 0xC6, 0x2A, 0x47, 0x40, 0x02, 0xDF, 0x32, 0xE5, 0x21, 0x39, 0xF0, 0xA0,
];

pub fn precompute_with_id_public_key(id: Option<&[u8]>, pk: &PublicKey) -> [u8; 32] {
    let mut d;
    if id.is_none() {
        d = sm3::Digest::new_with_default_id()
    } else {
        let id = id.unwrap();
        d = sm3::Digest::new();
        d.write(&[((id.len() >> 5) & 0xff) as u8, ((id.len() << 3) & 0xff) as u8]);
        d.write(id);
        d.write(&ABG);
    }
    d.write(&pk.x.to_be_bytes());
    d.write(&pk.y.to_be_bytes());
    d.sum()
}

pub fn precompute_with_id_public_key_msg(id: Option<&[u8]>, pk: &PublicKey, msg: &[u8]) -> [u8; 32] {
    let z = precompute_with_id_public_key(id, pk);
    let mut d = sm3::Digest::new();
    d.write(&z);
    d.write(msg);
    d.sum()
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;
    extern crate test;
    use super::*;
    use rand::Rng;

    #[test]
    fn test_sign() {
        let mut rng = rand::rng();
        let sk = PrivateKey::new(&mut rng);
        let pk = sk.public();

        let e = [1u8; 32];
        let k = rng.random();
        let sig = sign_inner(&e, &sk, &k).unwrap();
        let ok = verify(&e, &pk, &sig);
        assert!(ok);
    }

    #[test]
    fn test_sign_speed() {
        let sk = PrivateKey::new(&mut rand::rng());
        let _pk = sk.public();
        
        let mut rng = rand::rng();
        let e = [1u8; 32];

        sign(&e, &sk, &mut rng).unwrap();

        let loops = 1000000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(sign(&e, &sk, &mut rng).unwrap());
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        // println!("{:?}", sig);
        println!("{}K TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }

    #[test]
    fn test_verify_speed() {
        let mut rng = rand::rng();
        let sk = PrivateKey::new(&mut rng);
        let pk = sk.public();

        let e = [1u8; 32];
        let k = rng.random();
        let sig = sign_inner(&e, &sk, &k).unwrap();

        let loops = 10000u64;
        let now = SystemTime::now();
        for _ in 0..loops {
            test::black_box(verify(&e, &pk, &sig));
        }
        let elapsed = now.elapsed().unwrap().as_nanos();
        // println!("{:?}", sig);
        println!("{}K TPS", (loops as u128 * 1000000000) / (1000 * elapsed));
    }

    use test::Bencher;
    #[bench]
    fn bench_sign(b: &mut Bencher) {
        let sk = PrivateKey::new(&mut rand::rng());
        let _pk = sk.public();
        let e = [1u8; 32];
        let k = rand::rng().random();
        // 73,460.07 ns/iter
        b.iter(|| {
            test::black_box(sign_inner(&e, &sk, &k).unwrap());
        });
    }


    #[test]
    fn test_precompute(){
        let sk = PrivateKey{
            d:U256{v:[1,0,0,0]},
            d1inv:None,
            public_key: RefCell::new(None),
        };
        let pk = sk.public();
        let e = precompute_with_id_public_key(None, &pk);
        for i in e{
            print!("{:x}", i);
        }

    }
}
