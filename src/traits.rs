use core::any::Any;
use alloc::vec::Vec;
use num::BigInt;

use crate::{cryptobyte::{self, Builder, Parser}, sm2::{self, U256}, sm3::{self}, sm4};

pub trait MarshalAsn1{
    type Error;

    // Return a fixed length der encode instance, say, sm2 signature, sm2 cipher, etc.
    fn marshal_asn1(&self)-> Result<Vec::<u8>, Self::Error>;
}
pub trait UnmarshalAsn1: Sized{
    type Error;
    fn unmarshal_asn1(data: &[u8])-> Result<Self, Self::Error>;
}

// assume n.len() > 0.
#[inline]
fn trim_be_bytes(n: &[u8])-> &[u8]{
    debug_assert!(n.len() > 0);

    for (i, x) in n.iter().enumerate(){
        if *x != 0{
            return &n[i..];
        }
    }
    // n is all zero, return the last one: &[0]
    &n[n.len()-1..]
}
fn bigint_to_u256(n: &BigInt) ->U256{
    let (_, digits) = n.to_u64_digits();
    U256{
        v:[digits[0], digits[1], digits[2], digits[3]],
    }
}

fn u256_to_bigint(n: &U256) ->BigInt{
    BigInt::from_bytes_be(num::bigint::Sign::Plus, &n.to_be_bytes())
}

impl MarshalAsn1 for sm2::Signature{
    type Error = crate::cryptobyte::errors::Error;

    // DER encoded sm2 signature
    // sm2Signature ::= SEQUENCE {
    //  r INTEGER,
    //  s INTEGER,
    // }
    fn marshal_asn1(&self)-> Result<Vec::<u8>, Self::Error> {
        // at most 1+1+ 2*(1 + 1 + 33) = 72
        let mut builder = Builder::new(Vec::with_capacity(72));
        let r = self.r.to_be_bytes();
        let s = self.s.to_be_bytes();
        builder.add_asn1_sequence(|b|{
            let r = trim_be_bytes(&r);
            if r[0] >= 128{
                b.add_u8(0);
            }
            b.add_bytes(r);

            let s = trim_be_bytes(&s);
            if s[0] >= 128{
                b.add_u8(0);
            }
            b.add_bytes(s);
        });
        builder.take()
    }
}

impl UnmarshalAsn1 for sm2::Signature{
    type Error= crate::cryptobyte::errors::Error;

    fn unmarshal_asn1(data: &[u8])-> Result<Self, Self::Error> {
        let mut parser = Parser::new(data);
        // TODO: redesign error
        let mut parser = parser.read_asn1_sequence().ok_or(cryptobyte::errors::Error::Unknown)?;
        let r = parser.read_asn1_bigint().ok_or(cryptobyte::errors::Error::Unknown)?;
        let s = parser.read_asn1_bigint().ok_or(cryptobyte::errors::Error::Unknown)?;
        
        Ok(sm2::Signature{r: bigint_to_u256(&r), s: bigint_to_u256(&s)})
    }
}

pub trait Sign{
    fn sign<T: Any>(&self, data: &[u8]) -> T;
}

pub trait Hash<const DIGEST_SIZE:usize> {
    fn reset(&mut self);
    
    fn write(&mut self, data: &[u8]);

    fn sum_into(&self, digest: &mut [u8]);
    
    fn sum(&self)->[u8; DIGEST_SIZE]{
        let mut digest = [0; DIGEST_SIZE];
        self.sum_into(&mut digest);
        digest
    }

    // The block size of the input for the Hash, used in HMAC.
    // For SM3, it's 64.
    // The self is not necessarily, but we need it for a dyn Hash.
    fn block_size(&self)-> usize;
}

impl Hash<32> for sm3::Digest{

    fn reset(&mut self) {
        sm3::Digest::reset(self);
    }

    fn write(&mut self, data: &[u8]) {
        sm3::Digest::write(self, data);
    }

    fn sum_into(&self, digest: &mut [u8]) {
        sm3::Digest::sum_into(self,  digest);
    }
    fn sum(&self)->[u8; 32] {
        sm3::Digest::sum(self)
    }

    fn block_size(&self)-> usize{
        sm3::BLOCK_SIZE
    }
}

pub trait Block {
    fn block_size(&self)->usize;

    // Encrypt as many blocks as possible from src to dst.
    // More precisely, encrypt min(dst.len()/BLOCK_SIZE, src.len()/BLOCK_SIZE) blocks.
    // Returns the number of bytes encrypted.
    fn encrypt(&self, dst: &mut [u8], src: &[u8]) -> usize;
    fn decrypt(&self, dst: &mut [u8], src: &[u8]) -> usize;

    fn encrypt_inplace(&self, in_out: &mut [u8]) -> usize;
    fn decrypt_inplace(&self, in_out: &mut [u8]) -> usize;
}

pub trait Cipher {
    
}

pub trait ASN1Decode{
    fn asn1_decode(der: &[u8]);
}


impl Block for sm4::Cipher {
    // encrypt blocks into dst. returns the bytes encrypted.
    fn encrypt(&self, dst: &mut [u8], src: &[u8]) -> usize {
        sm4::Cipher::encrypt(self, dst, src)
    }

    // encrypt blocks into dst. returns the bytes encrypted.
    fn decrypt(&self, dst: &mut [u8], src: &[u8]) -> usize {
        sm4::Cipher::decrypt(self, dst, src)
    }
    
    fn encrypt_inplace(&self, in_out: &mut [u8]) -> usize {
        sm4::Cipher::encrypt_inplace(self, in_out)
    }
    
    fn decrypt_inplace(&self, in_out: &mut [u8]) -> usize {
        sm4::Cipher::decrypt_inplace(self, in_out)
    }
    
    
    
    fn block_size(&self)->usize {
        sm4::BLOCK_SIZE
    }
}



pub trait AEAD {
    type Error;

    // TLCP中,由master_secret生成4字节的client_iv, server_iv.
    // nonce = client_iv/server_iv(4) + explicit_nonce(8).

    // NonceSize returns the size of the nonce that must be passed to Seal
    // and Open.
    fn nonce_size(&self) -> usize;

    // Overhead returns the maximum difference between the lengths of a
    // plaintext and its ciphertext.
    fn overhead(&self) -> usize;

    fn seal(&self, out: &mut [u8], nonce: &[u8], plaintext: &[u8], add: Option<&[u8]>)-> Result<(), Self::Error>;
    
    fn open(&self, out: &mut [u8], nonce: &[u8], ciphertext: &[u8], add: Option<&[u8]>)-> Result<usize, Self::Error>;
    
    fn seal_inplace(&self, in_out: &mut [u8], tag: &mut [u8], nonce: &[u8], add: Option<&[u8]>)-> Result<(), Self::Error>;

    fn open_inplace(&self, in_out: &mut [u8], tag: &[u8], nonce: &[u8], add: Option<&[u8]>)-> Result<(), Self::Error> ;
}