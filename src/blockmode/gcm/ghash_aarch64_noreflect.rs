use super::*;
use core::arch::aarch64::*;
use core::mem::transmute;
use std::arch::*;

#[derive(Default)]
pub struct GHasherAarch64 {
    h: u128,// the key
    y: u128,
}


pub fn support_pmull_aarch64() -> bool {
    is_aarch64_feature_detected!("neon") && is_aarch64_feature_detected!("aes")
}

impl GHash for GHasherAarch64 {
    #[inline]
    fn init(&mut self, key: &[u8; 16]) {
        self.h = to_u128(key);
        self.y = 0;
    }

    #[inline]
    fn reset(&mut self){
        self.y = 0;
    }

    // update extends y with more polynomial terms from data. If data is not a
    // multiple of gcmBlockSize bytes long then the remainder is zero padded.
    #[inline]
    fn update(&mut self, data: &[u8]) {
        let full_blocks = (data.len() >> 4) << 4; //data.len() % 16

        self.update_blocks(&data[..full_blocks]);

        if data.len() != full_blocks {
            let mut partial_block = [0u8; BLOCK_SIZE];
            partial_block[..data.len() - full_blocks]
                .copy_from_slice(&data[full_blocks..]);
            self.update_blocks(&partial_block);
        }
    }

    
    fn sum(&self, h: &mut [u8; 16]) {
        h.copy_from_slice(&to_bytes(self.y));
    }


    // This only works
    // for two u64 a || b, in memory as bit-endian 16 bytes:
    // a0 ... a7 || b0 ... b7
    // The ghash need to A7 .. A0 || B7 .. B0, where Ai = reflect bits of ai.
    #[inline(always)]
    fn update_u64x2(&mut self, a: u64, b: u64) {
        unsafe {
            let z = (a as u128) | ((b as u128) << 64);
            let z: u128 =
                transmute(vrev64q_u8(transmute(vrbitq_p8(transmute(z)))));
            self.y ^= z;
        };
        self.y = ghash_mul_feflect(self.y, self.h);
    }

}

impl GHasherAarch64 {
    #[target_feature(enable = "neon", enable = "aes")]
    unsafe fn ghash_unsafe(
        &mut self,
        ciphertext: &[u8],
        add: Option<&[u8]>,
    ) -> [u8; 16] {
        let add_length = match add {
            Some(add) => {
                self.update(add);
                add.len() as u64
            }
            None => 0,
        };

        self.update(ciphertext);
        self.update_u64x2(
            add_length * 8,
            (ciphertext.len() * 8) as u64,
        );
        to_bytes(self.y)
    }



    // updateBlocks extends y with more polynomial terms from blocks, based on
    // Horner's rule. There must be a multiple of gcmBlockSize bytes in blocks.
    #[inline]
    fn update_blocks(&mut self, blocks: &[u8]) {
        for block in blocks.chunks_exact(16) {
            self.y ^= to_u128(block.try_into().unwrap());
            self.y = ghash_mul_feflect(self.y, self.h);
            // self.mul(y);
        }
    }

    // set y = y*H
    #[inline]
    fn mul(&self, y: &mut u128) {
        *y = ghash_mul_feflect(self.h, *y);
    }

    // ghash update two u64: (a)_64 || (b)_64.
    
}

fn reverse(b: u8) -> u8 {
    let mut x = 0;
    for i in 0..8 {
        x |= ((b >> i) & 1) << (7 - i);
    }
    x
}

fn reverse_u64(y: u64) -> u64 {
    let mut x = 0;
    for i in 0..64 {
        x |= ((y >> i) & 1) << (63 - i);
    }
    x
}

fn reverse_u128(y: u128) -> u128 {
    unsafe { transmute(vrbitq_p8(transmute(y))) }
}
#[inline(always)]
fn to_u128(v: &[u8; 16]) -> u128 {
    unsafe { transmute(vrbitq_p8(transmute(u128::from_le_bytes(*v)))) }
}
#[inline(always)]
fn to_bytes(v: u128) -> [u8; 16] {
    let v: u128 = unsafe { transmute(vrbitq_p8(transmute(v))) };
    v.to_le_bytes()
}

fn print_u128(v: u128) {
    let v0 = reverse_u64(v as u64);
    let v1 = reverse_u64((v >> 64) as u64);
    println!("{:016x} {:016x}", v0, v1);
}

#[inline(always)]
fn poly_mul_full(x: u128, y: u128) -> (u128, u128) {
    unsafe {
        let x0 = x as u64;
        let x1 = (x >> 64) as u64;
        let y0 = y as u64;
        let y1 = (y >> 64) as u64;
        let a = vmull_p64(x0, y0);
        let b = vmull_p64(x1, y1);
        let c = vmull_p64(x0 ^ x1, y0 ^ y1);
        let d = c ^ a ^ b;
        let z0 = a ^ (d << 64);
        let z1 = b ^ (d >> 64);
        (z0, z1)
    }
}

// return x0 * x^128 = x^0 * (x^7 + x^2 + x + 1)
#[inline(always)]
fn poly_mul_x128(x0: u64) -> u128 {
    unsafe { vmull_p64(x0, 0x87) }
}

// return x0 * x^192 = x^0 * (x^7 + x^2 + x + 1) * 64
#[inline(always)]
fn poly_mul_x192(x0: u64) -> u128 {
    let z = unsafe { vmull_p64(x0, 0x87) }; // x0 * (x^7 + x^2 + x + 1)

    // z = (z0, z1)
    // returns (z1 * (x^7 + x^2 + x + 1)) ^ (z0 << 64)
    poly_mul_x128((z >> 64) as u64) ^ (z << 64)
}
#[inline(always)]
fn poly_mod(z0: u128, z1: u128) -> u128 {
    z0 ^ poly_mul_x128(z1 as u64) ^ poly_mul_x192((z1 >> 64) as u64)
}

#[inline(always)]
fn ghash_mul_feflect(x: u128, y: u128) -> u128 {
    let (z0, z1) = poly_mul_full(x , y);
    poly_mod(z0, z1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn test_pmull_full() {
        let x = 1<<64;
        let y = 1<<64;
        let (z0, z1) = poly_mul_full(x, y);
        println!("{:016x} {:016x}", z1, z0);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn test_ghash_mull() {
        let x = (1 << 64) + 1;
        let y = 1 << 64;
        let z = ghash_mul_feflect(x, y);
        println!("{:016x} {:016x}", (z >> 64) as u64, z as u64);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn test_reverse_bit() {
        let x: u128 = 0x01010101;
        println!("{:016x} {:016x}", (x >> 64) as u64, x as u64);
        let z = reverse_u128(x);
        println!("{:016x} {:016x}", (z >> 64) as u64, z as u64);
    }


    extern crate test;
    use test::Bencher;
    #[bench]
    fn bench_ghash_mul(b: &mut Bencher) {
        let mut x: u128 = unsafe { transmute([0x111111111u64, 0x12345678]) };
        let y: u128 = unsafe { transmute([123422334u64, 1123456788]) };

        // 4.03 ns
        b.iter(|| {
            test::black_box(x = ghash_mul_feflect(x, y));
        });
    }
}
