use super::*;
use core::ops::{Add, AddAssign};

#[derive(Default)]
pub struct GHasherGeneric {
    // productTable contains the first sixteen powers of the key, H.
    // However, they are in bit reversed order. See NewGCMWithNonceSize.
    product_table: [FieldElement; 16],

    y: FieldElement,
}

// reverse order of bits of f(x) * (x^7 + x^2 + x + 1) for deg(x) <= 3.
// The result is represented by two bytes.
// EX: f(x) = 1 and x^7 + x^2 + x + 1 = 0b0000_0000_1000_0111 => 0b1110_0001_0000_0000 = 0xe100
// reverse bits of f(x) = 1 is 1000 = 8.
// So GCM_REDUCTION_TABLE[8] = 0xe100.
const GCM_REDUCTION_TABLE: [u64; 16] = [
    0x0000, 0x1c20, 0x3840, 0x2460, 0x7080, 0x6ca0, 0x48c0, 0x54e0, 0xe100,
    0xfd20, 0xd940, 0xc560, 0x9180, 0x8da0, 0xa9c0, 0xb5e0,
];

impl GHash for GHasherGeneric {
    fn init(&mut self, key: &[u8; 16]) {
        // We precompute 16 multiples of |key|. However, when we do lookups
        // into this table we'll be using bits from a field element and
        // therefore the bits will be in the reverse order. So normally one
        // would expect, say, 4*key to be in index 4 of the table but due to
        // this bit ordering it will actually be in index 0010 (base 2) = 2.
        let x = FieldElement {
            low: u64::from_be_bytes(key[..8].try_into().unwrap()),
            high: u64::from_be_bytes(key[8..].try_into().unwrap()),
        };
        // let mut g = GHasherGeneric{
        //     product_table: [FieldElement::default();16],
        // };
        self.product_table[reverse_bits(1) as usize] = x;

        let mut i = 2;
        while i < 16 {
            self.product_table[reverse_bits(i) as usize] =
                self.product_table[reverse_bits(i / 2) as usize].double();
            self.product_table[reverse_bits(i + 1) as usize] =
                self.product_table[reverse_bits(i) as usize] + x;
            i += 2;
        }
        self.y = FieldElement::default();
    }

    fn reset(&mut self) {
        self.y = FieldElement::default();
    }

    fn update(&mut self, data: &[u8]) {
        let mut y = self.y;
        let full_blocks = (data.len() >> 4) << 4; //data.len() % 16

        self.update_blocks(&mut y, &data[..full_blocks]);

        if data.len() != full_blocks {
            let mut partial_block = [0u8; BLOCK_SIZE];
            partial_block[..data.len() - full_blocks]
                .copy_from_slice(&data[full_blocks..]);
            self.update_blocks(&mut y, &partial_block);
        }
        self.y = y;
    }

    fn update_u64x2(&mut self, a: u64, b: u64) {
        let mut y = self.y;

        y.low ^= a;
        y.high ^= b;

        self.mul(&mut y);
        self.y = y;
    }

    fn sum(&self, h: &mut [u8; 16]) {
        h.copy_from_slice(&self.y.bytes());
    }

    // // deriveCounter computes the initial GCM counter state from the given nonce.
    // // See NIST SP 800-38D, section 7.1. This assumes that counter is filled with
    // // zeros on entry.
    // fn derive_counter(&self, nonce: &[u8]) -> [u8; BLOCK_SIZE] {
    //     // GCM has two modes of operation with respect to the initial counter
    //     // state: a "fast path" for 96-bit (12-byte) nonces, and a "slow path"
    //     // for nonces of other lengths. For a 96-bit nonce, the nonce, along
    //     // with a four-byte big-endian counter starting at one, is used
    //     // directly as the starting counter. For other nonce sizes, the counter
    //     // is computed by passing it through the GHASH function.
    //     let mut counter = [0; BLOCK_SIZE];
    //     if nonce.len() == STD_NONCE_SIZE {
    //         counter[..STD_NONCE_SIZE].copy_from_slice(nonce);
    //         counter[BLOCK_SIZE - 1] = 1;
    //     } else {
    //         let mut y = FieldElement::default();

    //         self.update(&mut y, nonce);
    //         y.high ^= (nonce.len() << 3) as u64;

    //         self.mul(&mut y);
    //         counter[..8].copy_from_slice(&y.low.to_be_bytes());
    //         counter[8..16].copy_from_slice(&y.high.to_be_bytes());
    //     }
    //     counter
    // }

    // // auth calculates GHASH(ciphertext, additionalData), masks the result with
    // // tagMask and writes the result to out.
    // fn ghash(&self, ciphertext: &[u8], add: Option<&[u8]>) -> [u8;16]{
    //     let mut y = FieldElement::default();
    //     let add_length = match add {
    //         Some(add) => {
    //             self.update(&mut y, add);
    //             add.len() as u64
    //         },
    //         None => 0,
    //     };

    //     self.update(&mut y, ciphertext);
    //     y.low ^= add_length * 8;
    //     y.high ^= ciphertext.len() as u64 * 8;

    //     self.mul(&mut y);
    //     y.bytes()
    // }
}
impl GHasherGeneric {
    // updateBlocks extends y with more polynomial terms from blocks, based on
    // Horner's rule. There must be a multiple of gcmBlockSize bytes in blocks.
    fn update_blocks(&self, y: &mut FieldElement, blocks: &[u8]) {
        for block in blocks.chunks_exact(16) {
            y.low ^= u64::from_be_bytes(block[..8].try_into().unwrap());
            y.high ^= u64::from_be_bytes(block[8..16].try_into().unwrap());
            self.mul(y);
        }
    }

    // set y = y*H
    fn mul(&self, y: &mut FieldElement) {
        let mut z = FieldElement::default();
        for i in 0..2 {
            let mut word = match i {
                0 => y.high,
                _ => y.low,
            };

            // Multiplication works by multiplying z by 16 and adding in
            // one of the precomputed multiples of H.
            let mut j = 0;
            while j < 64 {
                let msw = z.high & 0xf;
                z.high >>= 4;
                z.high |= z.low << 60;
                z.low >>= 4;
                z.low ^= GCM_REDUCTION_TABLE[msw as usize] << 48;

                // the values in |table| are ordered for
                // little-endian bit positions. See the comment
                // in NewGCMWithNonceSize.
                let t = self.product_table[(word & 0xf) as usize];

                z.low ^= t.low;
                z.high ^= t.high;
                word >>= 4;
                j += 4;
            }
        }

        *y = z;
    }
}

// gcmFieldElement represents a value in GF(2¹²⁸). In order to reflect the GCM
// standard and make binary.BigEndian suitable for marshaling these values, the
// bits are stored in big endian order. For example:
//
//	the coefficient of x⁰ can be obtained by v.low >> 63.
//	the coefficient of x⁶³ can be obtained by v.low & 1.
//	the coefficient of x⁶⁴ can be obtained by v.high >> 63.
//	the coefficient of x¹²⁷ can be obtained by v.high & 1.
// GF(2^128) = GF(2)[x]/(x^128 + x^7 + x^2 + x + 1)
#[derive(Default, Copy, Clone, Eq, PartialEq)]
struct FieldElement {
    low: u64,
    high: u64,
}

impl Add for FieldElement {
    type Output = FieldElement;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        FieldElement {
            low: self.low ^ rhs.low,
            high: self.high ^ rhs.high,
        }
    }
}
impl AddAssign for FieldElement {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.low ^= rhs.low;
        self.high ^= rhs.high;
    }
}

impl FieldElement {
    #[inline]
    pub fn low(&mut self) -> &mut u64 {
        &mut self.low
    }

    #[inline]
    pub fn high(&mut self) -> &mut u64 {
        &mut self.high
    }

    #[inline]
    pub fn double(self) -> Self {
        let msb = self.high & 1;

        let high = (self.high >> 1) | (self.low << 63);
        let mut low = self.low >> 1;

        // If the most-significant bit was set before shifting then it,
        // conceptually, becomes a term of x^128. This is greater than the
        // irreducible polynomial so the result has to be reduced. The
        // irreducible polynomial is 1+x+x^2+x^7+x^128. We can subtract that to
        // eliminate the term at x^128 which also means subtracting the other
        // four terms. In characteristic 2 fields, subtraction == addition ==
        // XOR.
        if msb == 1 {
            low ^= 0xe100000000000000;
        }
        FieldElement { low, high }
    }

    #[inline]
    pub fn bytes(self) -> [u8; 16] {
        let mut out = [0; 16];
        out[..8].copy_from_slice(&self.low.to_be_bytes());
        out[8..16].copy_from_slice(&self.high.to_be_bytes());
        out
    }
}

// reverseBits reverses the order of the bits of 4-bit number in i.
#[inline]
fn reverse_bits(i: u32) -> u32 {
    let i = ((i << 2) & 0xc) | ((i >> 2) & 0x3);
    ((i << 1) & 0xa) | ((i >> 1) & 0x5)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    extern crate test;

    use test::Bencher;
    #[bench]
    fn bench_ghash_mul(b: &mut Bencher) {
        // let mut x: u128 = unsafe { transmute([0x111111111u64, 0x12345678]) };
        let mut y = FieldElement{
            low: 123422334,
            high: 1123456788,
        };
        let mut g = GHasherGeneric::default();
        g.init(&[0xab; 16]);
        // 4.03 ns
        b.iter(|| {
            test::black_box(g.mul(&mut y));
        });
    }
}
