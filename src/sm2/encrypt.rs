use super::{
    ec::curve::AffinePoint,
    PrivateKey, PublicKey, U256,
};
use crate::sm3;
use subtle::ConstantTimeEq;
use crate::sm2::error::{SM2Error, Result};

/// The Cipher type has a equal length Cipher.c with the plaintext.
/// SM2 should not to encrypt a large plaintext, for the cipher is allocated on the stack,
/// too big plaintext cause stack overflows.
/// It is always used for encrypting a SM4 key, that is, 16 bytes long, or a pre-master key
/// in TLS/TLCP, that is, 48 bytes long.
pub struct Cipher<const N:usize> {
    pub x: U256,
    pub y: U256,
    pub h: [u8; sm3::DIGEST_SIZE],
    pub c: [u8; N],
}

/// encrypt computes the cipher
/// The N can not too big, or stack overflows.
pub fn encrypt<const N:usize>(pk: &PublicKey, data: &[u8;N], k: &[u64; 4]) -> Cipher<N> {
    let c1 = AffinePoint::new_from_scalar_base_mul(k);
    let mut s = AffinePoint::from((pk.x, pk.y));
    s.scalar_mul(k);
    let x = U256::from(s.x);
    let y = U256::from(s.y);

    let mut kdf = KDF::new();
    kdf.read(x.v[3].to_be_bytes().as_slice());
    kdf.read(x.v[2].to_be_bytes().as_slice());
    kdf.read(x.v[1].to_be_bytes().as_slice());
    kdf.read(x.v[0].to_be_bytes().as_slice());
    kdf.read(y.v[3].to_be_bytes().as_slice());
    kdf.read(y.v[2].to_be_bytes().as_slice());
    kdf.read(y.v[1].to_be_bytes().as_slice());
    kdf.read(y.v[0].to_be_bytes().as_slice());
    let mut c = [0_u8; N];
    kdf.write(c.as_mut());
    for i in 0..N {
        c[i] ^= data[i];
    }
    let mut c3_hash = sm3::Digest::new();
    c3_hash.write(x.v[3].to_be_bytes().as_slice());
    c3_hash.write(x.v[2].to_be_bytes().as_slice());
    c3_hash.write(x.v[1].to_be_bytes().as_slice());
    c3_hash.write(x.v[0].to_be_bytes().as_slice());
    c3_hash.write(data);
    c3_hash.write(y.v[3].to_be_bytes().as_slice());
    c3_hash.write(y.v[2].to_be_bytes().as_slice());
    c3_hash.write(y.v[1].to_be_bytes().as_slice());
    c3_hash.write(y.v[0].to_be_bytes().as_slice());

    Cipher {
        x: U256::from(c1.x),
        y: U256::from(c1.y),
        h: c3_hash.sum(),
        c: c,
    }
}

pub fn decrypt<const N:usize>(sk: &PrivateKey, cipher: &Cipher<N>) -> Result<[u8;N]> {
    let mut s = AffinePoint::new(cipher.x, cipher.y);
    s.scalar_mul(&sk.d.v);
    let x = U256::from(s.x);
    let y = U256::from(s.y);

    let mut kdf = KDF::new();
    kdf.read(x.v[3].to_be_bytes().as_slice());
    kdf.read(x.v[2].to_be_bytes().as_slice());
    kdf.read(x.v[1].to_be_bytes().as_slice());
    kdf.read(x.v[0].to_be_bytes().as_slice());
    kdf.read(y.v[3].to_be_bytes().as_slice());
    kdf.read(y.v[2].to_be_bytes().as_slice());
    kdf.read(y.v[1].to_be_bytes().as_slice());
    kdf.read(y.v[0].to_be_bytes().as_slice());
    let mut m = [0_u8; N];
    kdf.write(m.as_mut());
    for i in 0..m.len() {
        m[i] ^= cipher.c[i];
    }
    let mut c3_hash = sm3::Digest::new();
    c3_hash.write(x.v[3].to_be_bytes().as_slice());
    c3_hash.write(x.v[2].to_be_bytes().as_slice());
    c3_hash.write(x.v[1].to_be_bytes().as_slice());
    c3_hash.write(x.v[0].to_be_bytes().as_slice());
    c3_hash.write(m.as_slice());
    c3_hash.write(y.v[3].to_be_bytes().as_slice());
    c3_hash.write(y.v[2].to_be_bytes().as_slice());
    c3_hash.write(y.v[1].to_be_bytes().as_slice());
    c3_hash.write(y.v[0].to_be_bytes().as_slice());
    let hash = c3_hash.sum();
    let eq: bool = hash.ct_eq(cipher.h.as_slice()).into();
    if !eq {
        Err(SM2Error::InvalidCipherHash)
    } else {
        Ok(m)
    }
}
// We need copy the internal states for different counter.
#[derive(Debug, Clone, Copy)]
struct KDF {
    hash: sm3::Digest,
}

impl KDF {
    fn new() -> KDF {
        KDF { hash: sm3::Digest::new() }
    }

    // KDF read multiple times from input.
    fn read(&mut self, input: &[u8]) -> &mut Self {
        self.hash.write(input);
        self
    }

    // KDF can only write once!
    fn write(mut self, output: &mut [u8]) {
        let mut ct = 1_u32;
        const CHUNK_SIZE: usize = sm3::DIGEST_SIZE;
        let (chunks, tail) = output.as_chunks_mut::<CHUNK_SIZE>();
        for chunk in chunks {
            let mut hash = self.hash;
            hash.write(ct.to_be_bytes().as_slice());
            hash.sum_into(chunk);
            ct += 1;
        }
        let tail_hash = self.hash.write(ct.to_be_bytes().as_slice()).sum();
        for i in 0..tail.len() {
            tail[i] = tail_hash[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use super::*;

    #[test]
    fn test_enc() {
        let mut rng = rand::rng();
        let sk = PrivateKey::new(&mut rng);
        let pk = sk.public();

        const N:usize = 32 * 1024;
        let m = [1u8; N];
        let k = rng.random();
        let cipher = encrypt(&pk, &m, &k);

        let mm = decrypt(&sk, &cipher).unwrap();

        assert_eq!(m, mm.as_slice());
    }
}
