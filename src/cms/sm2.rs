use num::BigInt;
use crate::{sm2::*, sm3};
use crate::cryptobyte::{Builder, Parser};

use alloc::vec::*;

fn bigint_to_u256(n: &BigInt) ->U256{
    let (_, digits) = n.to_u64_digits();
    U256{
        v:[digits[0], digits[1], digits[2], digits[3]],
    }
}

fn u256_to_bigint(n: &U256) ->BigInt{
    BigInt::from_bytes_be(num::bigint::Sign::Plus, &n.to_be_bytes())
}


const PRE_MASTER_KEY_SIZE: usize = 48;

// TLCP use a cipher text of length 48.
// I.e, encrypte pre-master secret.
pub trait ASN1Decode {
    fn decode_sm2_public_key(&mut self) -> Option<PublicKey>;
    fn decode_sm2_cipher(&mut self)-> Option<Cipher::<PRE_MASTER_KEY_SIZE>>;
    fn decode_sm2_signature(&mut self)-> Option<Signature>;
}


impl<'a> ASN1Decode for Parser<'a>{
    fn decode_sm2_cipher(&mut self)-> Option<Cipher::<PRE_MASTER_KEY_SIZE>>{
        let mut parser = self.read_asn1_sequence()?;
        let x = parser.read_asn1_bigint()?;
        let y = parser.read_asn1_bigint()?;
        
        let mut cipher = Cipher::<PRE_MASTER_KEY_SIZE>{
            x: bigint_to_u256(&x),
            y: bigint_to_u256(&y),
            h: [0u8;sm3::DIGEST_SIZE],
            c: [0u8; PRE_MASTER_KEY_SIZE],
        };
        
        let hash = parser.read_asn1_octet_string()?;
        if hash.len() != sm3::DIGEST_SIZE{
            return None;
        }
        cipher.h.copy_from_slice(hash);
        
        let c = parser.read_asn1_octet_string()?;
        if c.len() != PRE_MASTER_KEY_SIZE{
            return None;
        }
        cipher.c.copy_from_slice(c);
        Some(cipher)
    }    

     // TODO: public key = BIT STRING = 04||x||y or ...
     fn decode_sm2_public_key(&mut self) -> Option<PublicKey> {
        let v = self.v;
        if v.len() < 65 {
            return None;
        }
        match v[0] {
            4 => {
                if v.len() != 65 {
                    None
                } else {
                    let x = U256::from_be_slice(&v[1..33]).unwrap();
                    let y = U256::from_be_slice(&v[33..65]).unwrap();
                    let public_key = PublicKey { x, y };
                    if !public_key.is_valid() {
                        return None;
                    }
                    Some(public_key)
                }
            }
            _ => {
                panic!("unsupported public key format: {}", v[0])
            }
        }
    }
    fn decode_sm2_signature(&mut self)-> Option<Signature>{
        let mut parser = self.read_asn1_sequence()?;
        let r = parser.read_asn1_bigint()?;
        let s = parser.read_asn1_bigint()?;

        
        Some(Signature{r: bigint_to_u256(&r), s: bigint_to_u256(&s)})
    }
}

pub fn encode_sm2_cipher<const N:usize>(cipher: &Cipher<N>)-> Option<Vec<u8>> {
    let mut b = Builder::new(Vec::new());
    b.add_asn1_sequence(|b| {
        b.add_asn1_bigint(&u256_to_bigint(&cipher.x));
        b.add_asn1_bigint(&u256_to_bigint(&cipher.y));
        b.add_asn1_octet_string(&cipher.h);
        b.add_asn1_octet_string(&cipher.c);
    });
    
    b.take().ok()
}