mod ghash;
mod ghash_generic;

#[cfg(target_arch = "aarch64")]
mod ghash_aarch64;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod ghash_amd64;

use ghash::*;

#[cfg(target_arch = "aarch64")]
use ghash_aarch64::*;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use ghash_amd64::*;
use ghash_generic::GHasherGeneric;

use super::{Error, Result};
use crate::sm4;
use crate::traits::Block;
use crate::traits::AEAD;
use core::cmp::min;

const BLOCK_SIZE: usize = 16;
const TAG_SIZE: usize = 16;
const MIN_TAG_SIZE: usize = 12;
const STD_NONCE_SIZE: usize = 12;

// Returns a GCM instance with standard nonce size 12 and tag size 16.
// The input must have length 16.
pub fn new_sm4_gcm_std(key: &[u8]) -> Sm4Gcm<STD_NONCE_SIZE, TAG_SIZE> {
    Sm4Gcm::<STD_NONCE_SIZE, TAG_SIZE>::new(sm4::Cipher::new(&key))
}

pub type Sm4Gcm<const N: usize, const T: usize> = GCM<sm4::Cipher, N, T>;

pub struct GCM<B: Block, const N: usize, const T: usize> {
    pub cipher: B,
    key: [u8; 16],
}

#[inline(always)]
fn ghash(
    tag: &mut [u8; 16],
    key: &[u8; 16],
    data1: Option<&[u8]>,
    data2: &[u8],
) {
    #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
    ghash_template::<GHasherGeneric>(tag, key, data1, data2);

    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    if support_pmull_amd64() {
        ghash_template::<GHasherAmd64>(tag, key, data1, data2);
    }else{
        ghash_template::<GHasherGeneric>(tag, key, data1, data2);
    }

    #[cfg(target_arch = "aarch64")]
    if support_pmull_aarch64() {
        // ghash_template::<GHasherGeneric>(tag, key, data1, data2);
        ghash_template::<GHasherAarch64>(tag, key, data1, data2);
    } else {
        ghash_template::<GHasherGeneric>(tag, key, data1, data2);
    };
}

#[inline(always)]
fn ghash_template<H: GHash + Default>(
    tag: &mut [u8; 16],
    key: &[u8; 16],
    data1: Option<&[u8]>,
    data2: &[u8],
) {
    let mut g = H::default();
    g.init(key);
    let mut a = 0;
    if let Some(data1) = data1 {
        g.update(data1);
        a = data1.len() as u64 * 8;
    }

    g.update(data2);
    g.update_u64x2(a, data2.len() as u64 * 8);
    g.sum(tag);
}

// gcm represents a Galois Counter Mode with a specific key. See
// https://csrc.nist.gov/groups/ST/toolkit/BCM/documents/proposedmodes/gcm/gcm-revised-spec.pdf
impl<B: Block, const N: usize, const T: usize> GCM<B, N, T> {
    pub fn new(block: B) -> Self {
        debug_assert!(T >= MIN_TAG_SIZE);
        debug_assert!(T <= BLOCK_SIZE);
        debug_assert_eq!(block.block_size(), BLOCK_SIZE);

        // h = CIPH_K(0^128)
        let mut key: [u8; 16] = [0; BLOCK_SIZE];
        block.encrypt_inplace(&mut key);

        GCM { cipher: block, key }
    }
}

impl<B: Block, const N: usize, const T: usize> AEAD for GCM<B, N, T> {
    type Error = super::Error;

    fn overhead(&self) -> usize {
        T
    }

    fn nonce_size(&self) -> usize {
        N
    }

    fn seal_inplace(
        &self,
        in_out: &mut [u8],
        tag: &mut [u8],
        nonce: &[u8],
        add: Option<&[u8]>,
    ) -> Result<()> {
        if nonce.len() != N {
            return Err(Error::InvalidNonceSize(N, nonce.len()));
        }

        let mut counter = self.derive_counter(nonce);
        let mut tag_mask = [0; BLOCK_SIZE];
        self.cipher.encrypt(&mut tag_mask, &counter);
        inc32(&mut counter);

        self.counter_crypt_inplacd(in_out, &mut counter);

        self.auth(tag, in_out, add, &tag_mask);
        Ok(())
    }

    fn open_inplace(
        &self,
        in_out: &mut [u8],
        tag: &[u8],
        nonce: &[u8],
        add: Option<&[u8]>,
    ) -> Result<()> {
        if nonce.len() != N {
            return Err(Error::GCMAuthenticationError);
        }

        let mut tag_mask = [0; BLOCK_SIZE];
        let mut counter = self.derive_counter(nonce);
        self.cipher.encrypt(&mut tag_mask, &counter);

        inc32(&mut counter);

        let mut expected_tag = [0; BLOCK_SIZE];
        self.auth(&mut expected_tag, in_out, add, &tag_mask);

        self.counter_crypt_inplacd(in_out, &mut counter);
        for i in 0..min(T, tag.len()) {
            if expected_tag[i] != tag[i] {
                return Err(Error::GCMAuthenticationError);
            }
        }
        Ok(())
    }

    fn seal(
        &self,
        out: &mut [u8],
        nonce: &[u8],
        plaintext: &[u8],
        add: Option<&[u8]>,
    ) -> Result<()> {
        if nonce.len() != N {
            return Err(Error::InvalidNonceSize(N, nonce.len()));
        }

        let plaintext_length = plaintext.len();
        if out.len() < plaintext_length + T {
            return Err(Error::OutputTooSmall(plaintext_length + T, out.len()));
        }

        let (ciphertext, tag) = out.split_at_mut(plaintext_length);
        let mut counter = self.derive_counter(nonce);
        let mut tag_mask = [0; BLOCK_SIZE];
        self.cipher.encrypt(&mut tag_mask, &counter);
        inc32(&mut counter);

        self.counter_crypt(ciphertext, plaintext, &mut counter);

        self.auth(tag, ciphertext, add, &tag_mask);
        Ok(())
    }

    fn open(
        &self,
        out: &mut [u8],
        nonce: &[u8],
        ciphertext: &[u8],
        add: Option<&[u8]>,
    ) -> Result<usize> {
        if nonce.len() != N {
            return Err(Error::GCMAuthenticationError);
        }

        if ciphertext.len() < T {
            return Err(Error::GCMCiphertextTooSmall(ciphertext.len(), T));
        }

        let (ciphertext, tag) = ciphertext.split_at(ciphertext.len() - T);
        if out.len() < ciphertext.len() {
            return Err(Error::OutputBufferTooShort(
                ciphertext.len(),
                out.len(),
            ));
        }

        let mut tag_mask = [0; BLOCK_SIZE];
        let mut counter = self.derive_counter(nonce);
        self.cipher.encrypt(&mut tag_mask, &counter);

        inc32(&mut counter);

        let mut expected_tag = [0; BLOCK_SIZE];
        self.auth(&mut expected_tag, ciphertext, add, &tag_mask);

        for i in 0..T {
            if expected_tag[i] != tag[i] {
                return Err(Error::GCMAuthenticationError);
            }
        }

        self.counter_crypt(out, ciphertext, &mut counter);
        Ok(ciphertext.len())
    }
}

impl<B: Block, const N: usize, const T: usize> GCM<B, N, T> {
    fn derive_counter(&self, nonce: &[u8]) -> [u8; 16] {
        let mut counter = [0; BLOCK_SIZE];
        if nonce.len() == STD_NONCE_SIZE {
            counter[..STD_NONCE_SIZE].copy_from_slice(nonce);
            counter[BLOCK_SIZE - 1] = 1;
        } else {
            ghash(&mut counter, &self.key, None, nonce);
        }
        counter
    }

    // counterCrypt crypts in to out using g.cipher in counter mode.
    fn counter_crypt(
        &self,
        out: &mut [u8],
        input: &[u8],
        counter: &mut [u8; BLOCK_SIZE],
    ) {
        let counter_buf_len = (input.len() + BLOCK_SIZE) & (!15);
        let mut counter_buf = vec![0; counter_buf_len];

        for chunk in counter_buf.chunks_exact_mut(BLOCK_SIZE) {
            chunk.copy_from_slice(counter);
            inc32(counter);
        }
        self.cipher.encrypt_inplace(&mut counter_buf);

        input
            .iter()
            .zip(&counter_buf[..input.len()])
            .zip(out.iter_mut())
            .for_each(|((x, y), z)| *z = *x ^ *y);
    }

    // counterCrypt crypts in to out using g.cipher in counter mode.
    fn counter_crypt_inplacd(
        &self,
        in_out: &mut [u8],
        counter: &mut [u8; BLOCK_SIZE],
    ) {
        let in_out_length = in_out.len();
        let counter_buf_len = (in_out_length + BLOCK_SIZE) & (!15);
        let mut counter_buf = vec![0; counter_buf_len];

        for chunk in counter_buf.chunks_exact_mut(BLOCK_SIZE) {
            chunk.copy_from_slice(counter);
            inc32(counter);
        }
        self.cipher.encrypt_inplace(&mut counter_buf);

        in_out
            .iter_mut()
            .zip(&counter_buf[..in_out_length])
            .for_each(|(z, y)| *z ^= *y);
    }

    // auth calculates GHASH(ciphertext, additionalData), masks the result with
    // tagMask and writes the result to out.
    fn auth(
        &self,
        tag: &mut [u8],
        ciphertext: &[u8],
        add: Option<&[u8]>,
        tag_mask: &[u8; BLOCK_SIZE],
    ) {
        let mut out = [0; 16];
        ghash(&mut out, &self.key, add, ciphertext);

        let tag_length = tag.len();
        out.iter_mut()
            .zip(&tag_mask[..tag_length])
            .for_each(|(z, x)| *z ^= *x);
        tag.copy_from_slice(&out[..T]);
    }
}
#[inline]
fn inc32(counter: &mut [u8]) {
    let clen = counter.len();
    let ctr = &mut counter[clen - 4..];
    let x = u32::from_be_bytes(ctr.try_into().unwrap()) + 1;
    ctr.copy_from_slice(&u32::to_be_bytes(x));
}

#[cfg(test)]
mod tests {
    use super::new_sm4_gcm_std;
    use crate::{
        blockmode::gcm::GCM,
        sm4::{self},
        traits::AEAD,
    };
    use hex_literal::hex;
    use std::vec::Vec;

    struct TestVec<'a> {
        key: Vec<u8>,
        nonce: Vec<u8>,
        plain: Vec<u8>,
        add: Option<&'a [u8]>,
        expected: Vec<u8>,
    }

    #[test]
    fn test_gcm_std() {
        let test_vecs = vec![
            TestVec {
                key: hex!("11754cd72aec309bf52f7687212e8957").to_vec(),
                nonce: hex!("3c819d9a9bed087615030b65").to_vec(),
                plain: "plaintext".as_bytes().to_vec(),
                add: Some(
                    "additional message not need encrypt, empty is ok"
                        .as_bytes(),
                ),
                expected: hex!(
                    "6111f78f2f82b913c20e333160bfec034c3720ac133a6203b1"
                )
                .to_vec(),
            },
            // add = Some(&[]) equals to add = None
            TestVec {
                key: hex!("11754cd72aec309bf52f7687212e8957").to_vec(),
                nonce: hex!("3c819d9a9bed087615030b65").to_vec(),
                plain: "plaintext".as_bytes().to_vec(),
                add: Some("".as_bytes()),
                expected: hex!(
                    "6111f78f2f82b913c29c2e12d652d7dd0d1930120b7788281d"
                )
                .to_vec(),
            },
            TestVec {
                key: hex!("11754cd72aec309bf52f7687212e8957").to_vec(),
                nonce: hex!("3c819d9a9bed087615030b65").to_vec(),
                plain: "plaintext".as_bytes().to_vec(),
                add: None,
                expected: hex!(
                    "6111f78f2f82b913c29c2e12d652d7dd0d1930120b7788281d"
                )
                .to_vec(),
            },
        ];

        for v in test_vecs {
            let g = new_sm4_gcm_std(&v.key);
            let mut out = Vec::new();
            out.resize(v.plain.len() + g.overhead(), 0);

            g.seal(&mut out, &v.nonce, &v.plain, v.add).unwrap();

            assert_eq!(&out, &v.expected);

            let mut decrypted_plain = [0; 128];
            match g.open(&mut decrypted_plain, &v.nonce, &out, v.add) {
                Ok(n) => assert_eq!(&decrypted_plain[..n], v.plain),
                Err(e) => println!("{:?}", e),
            }

            out.clear();
            out.extend_from_slice(&v.plain);
            out.resize(v.plain.len() + g.overhead(), 0);
            let (ciphertext, tag) = out.split_at_mut(v.plain.len());
            g.seal_inplace(ciphertext, tag, &v.nonce, v.add).unwrap();
            assert_eq!(out.as_slice(), v.expected.as_slice());

            let (ciphertext, tag) = out.split_at_mut(v.plain.len());
            g.open_inplace(ciphertext, tag, &v.nonce, v.add).unwrap();
            assert_eq!(ciphertext, &v.plain);
        }
        // cbc.decrypt_inplace(&iv, &mut plain).unwrap();
        // assert_eq!(plain, hex!("7B5BD9FDAE2521A3F0FBDD2F4427142F785C52080B0DB22523C3BC5D8716D141CE315586EBB3EDF4480193B1B3C33524"));
    }

    #[test]
    fn test_gcm_nonce10() {
        let test_vecs = vec![TestVec {
            key: hex!("11754cd72aec309bf52f7687212e8957").to_vec(),
            nonce: hex!("3c819d9a9bed08761503").to_vec(),
            plain: "plaintext".as_bytes().to_vec(),
            add: None,
            expected: hex!(
                "7705c6569e9ada5811d8b7523617ca62ce1aa4a924de38a31d"
            )
            .to_vec(),
        }];

        for v in test_vecs {
            let g = GCM::<_, 10, 16>::new(sm4::Cipher::new(&v.key));
            let mut out = Vec::new();
            out.resize(v.plain.len() + g.overhead(), 0);

            g.seal(&mut out, &v.nonce, &v.plain, v.add).unwrap();

            assert_eq!(&out, &v.expected);

            let mut decrypted_plain = [0; 128];
            match g.open(&mut decrypted_plain, &v.nonce, &out, v.add) {
                Ok(n) => assert_eq!(&decrypted_plain[..n], v.plain),
                Err(e) => println!("{:?}", e),
            }

            out.clear();
            out.extend_from_slice(&v.plain);
            out.resize(v.plain.len() + g.overhead(), 0);
            let (ciphertext, tag) = out.split_at_mut(v.plain.len());
            g.seal_inplace(ciphertext, tag, &v.nonce, v.add).unwrap();
            assert_eq!(out.as_slice(), v.expected.as_slice());

            let (ciphertext, tag) = out.split_at_mut(v.plain.len());
            g.open_inplace(ciphertext, tag, &v.nonce, v.add).unwrap();
            assert_eq!(ciphertext, &v.plain);
        }
        // cbc.decrypt_inplace(&iv, &mut plain).unwrap();
        // assert_eq!(plain, hex!("7B5BD9FDAE2521A3F0FBDD2F4427142F785C52080B0DB22523C3BC5D8716D141CE315586EBB3EDF4480193B1B3C33524"));
    }

    use std::time::*;

    // cargo test --release --package opengm_crypto --lib -- blockmode::gcm::tests::test_bench --exact --show-output
    // NUC: 583.31 MB/s (913.73 MB/s without auth - CTR)
    // M1:  164.99 MB/s (196.38 MB/s without auth)
    #[test]
    fn test_bench() {
        const TOTAL_BYTES: usize = 1024 * 1024;
        const COUNT: usize = 1000;
        let mut msg = vec![0xabu8; TOTAL_BYTES];
        let key = hex!("11754cd72aec309bf52f7687212e8957");
        let nonce = hex!("3c819d9a9bed087615030b65");
        let g = new_sm4_gcm_std(&key);
        let mut tag = [0; 16];

        let start = Instant::now();
        for _ in 0..COUNT {
            test::black_box(g.seal_inplace(&mut msg, &mut tag, &nonce, None).unwrap());
        }
        let d = (Instant::now() - start).as_micros() as f64 / 1000000.0;
        println!("{:.2} MB/s", (TOTAL_BYTES * COUNT / 1024 / 1024) as f64 / d);
    }

    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_gcm(b: &mut Bencher) {
        let key = hex!("11754cd72aec309bf52f7687212e8957");
        let nonce = hex!("3c819d9a9bed087615030b65");
        let mut plain = [0; 1024 * 4];
        let add = "additional message not need encrypt, empty is ok".as_bytes();

        let g = new_sm4_gcm_std(&key);
        let mut tag = [0; 16];

        // amd64:   7,487.37 ns/iter(NUC)
        // aarch64: 22,864.24 ns/iter (+/- 863.90) - 22,633.01 ns/iter
        // generic: 37,280.99 ns/iter (+/- 1,188.12)
        b.iter(|| {
            test::black_box(
                g.seal_inplace(&mut plain, &mut tag, &nonce, Some(add))
                    .unwrap(),
            );
        });
    }
}
