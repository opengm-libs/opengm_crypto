use core::cell::RefCell;

use ec::{curve::*, LIMB, NLIMBS};
use rand::Rng;
mod ec;
mod encrypt;
mod key_exchange;

pub mod sign;
pub mod error;

pub use encrypt::*;
pub use sign::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct U256 {
    // little-endian representation of 256 bits integer.
    pub v: [LIMB; NLIMBS],
}

impl U256 {
    //
    pub fn from_be_slice(v: &[u8]) -> Option<Self> {
        if v.len() != 32 {
            None
        } else {
            Some(U256 {
                v: [
                    u64::from_be_bytes(v[24..32].try_into().unwrap()),
                    u64::from_be_bytes(v[16..24].try_into().unwrap()),
                    u64::from_be_bytes(v[8..16].try_into().unwrap()),
                    u64::from_be_bytes(v[0..8].try_into().unwrap()),
                ],
            })
        }
    }

    pub fn from_le_slice(v: &[u8]) -> Option<Self> {
        if v.len() != 32 {
            None
        } else {
            Some(U256 {
                v: [
                    u64::from_le_bytes(v[0..8].try_into().unwrap()),
                    u64::from_le_bytes(v[8..16].try_into().unwrap()),
                    u64::from_le_bytes(v[16..24].try_into().unwrap()),
                    u64::from_le_bytes(v[24..32].try_into().unwrap()),
                ],
            })
        }
    }

    // U256 to big-endian 32 bytes.
    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut res = [0; 32];
        res[0..8].copy_from_slice(&self.v[3].to_be_bytes());
        res[8..16].copy_from_slice(&self.v[2].to_be_bytes());
        res[16..24].copy_from_slice(&self.v[1].to_be_bytes());
        res[24..32].copy_from_slice(&self.v[0].to_be_bytes());
        res
    }
}

impl From<[u8; 32]> for U256 {
    fn from(value: [u8; 32]) -> Self {
        U256 {
            v: [
                u64::from_le_bytes(value[0..8].try_into().unwrap()),
                u64::from_le_bytes(value[8..16].try_into().unwrap()),
                u64::from_le_bytes(value[16..24].try_into().unwrap()),
                u64::from_le_bytes(value[24..32].try_into().unwrap()),
            ],
        }
    }
}
impl From<[u64; 4]> for U256 {
    fn from(value: [u64; 4]) -> Self {
        U256 { v: value }
    }
}

#[derive(Debug, Clone)]
pub struct PublicKey {
    pub x: U256,
    pub y: U256,
}

impl PublicKey {
    // TODO
    pub fn is_valid(&self) -> bool {
        return true;
    }
}

pub struct PrivateKey {
    d: U256,
    d1inv: Option<U256>, // 1/(1+d)
    public_key: RefCell<Option<PublicKey>>,
}

impl Drop for PrivateKey {
    fn drop(&mut self) {
        self.d.v = [0, 0, 0, 0];
    }
}

impl PrivateKey {
    pub fn new(d: &mut impl Rng) -> Self {
        PrivateKey {
            d: U256 { v: d.random() },
            d1inv: None,
            public_key: RefCell::new(None),
        }
    }

    /// returns the public key.
    pub fn public(&self) -> PublicKey {
        if self.public_key.borrow().is_none() {
            let mut p = AffinePoint::from(
                &JacobianPoint::new_from_scalar_base_mul(&self.d.v),
            );
            p.x.transform_from_mont();
            p.y.transform_from_mont();

            *self.public_key.borrow_mut() = Some(PublicKey {
                x: U256 { v: p.x.limbs },
                y: U256 { v: p.y.limbs },
            });
        }
        (*self.public_key.borrow()).clone().unwrap()
    }
}

// fincrypto benchmark:
//
// Run on (10 X 24 MHz CPU s) M1
// CPU Caches:
//   L1 Data 64 KiB
//   L1 Instruction 128 KiB
//   L2 Unified 4096 KiB (x10)
// Load Average: 5.96, 2.96, 3.01
// -------------------------------------------------------------------------------------------
// Benchmark                                 Time             CPU   Iterations UserCounters...
// -------------------------------------------------------------------------------------------
// BM_Sign/min_time:1.000                 9010 ns         8964 ns       149571 items_per_second=111.562k/s
// BM_Verify/min_time:1.000              50229 ns        50153 ns        27952 items_per_second=19.9391k/s
// BM_c256Add/min_time:1.000              1.87 ns         1.87 ns    749536893 items_per_second=535.581M/s
// BM_c256Sub/min_time:1.000              1.88 ns         1.88 ns    748374957 items_per_second=532.694M/s
// BM_c256Mul/min_time:1.000              11.0 ns         11.0 ns    126445087 items_per_second=90.6872M/s
// BM_c256Sqr/min_time:1.000              10.7 ns         10.7 ns    129808718 items_per_second=93.2619M/s
// BM_c256Inv/min_time:1.000              3786 ns         3780 ns       371063 items_per_second=264.532k/s
// BM_ordMul/min_time:1.000               15.2 ns         15.2 ns     91869545 items_per_second=65.7277M/s
// BM_ordAdd/min_time:1.000               2.18 ns         2.16 ns    656617295 items_per_second=463.369M/s
// BM_ordInv/min_time:1.000               6056 ns         5946 ns       237501 items_per_second=168.18k/s
// BM_pointAdd                             217 ns          214 ns      3282101 items_per_second=4.67481M/s
// BM_pointDouble                          118 ns          118 ns      5876082 items_per_second=8.48161M/s
// BM_scalarBaseMult/min_time:1.000       6438 ns         6427 ns       217653 items_per_second=155.596k/s
// BM_scalarMult/min_time:1.000          47385 ns        47326 ns        29407 items_per_second=21.1299k/s

// Run on (2 X 2803.2 MHz CPU s) Intel NUC
// CPU Caches:
//   L1 Data 48 KiB (x2)
//   L1 Instruction 32 KiB (x2)
//   L2 Unified 1280 KiB (x2)
//   L3 Unified 12288 KiB (x1)
// Load Average: 0.21, 0.13, 0.18
// -------------------------------------------------------------------------------------------
// Benchmark                                 Time             CPU   Iterations UserCounters...
// -------------------------------------------------------------------------------------------
// BM_Sign/min_time:1.000                10873 ns        10861 ns       129970 items_per_second=92.0717k/s
// BM_Verify/min_time:1.000              63519 ns        63490 ns        20993 items_per_second=15.7505k/s
// BM_c256Add/min_time:1.000              2.94 ns         2.94 ns    441312631 items_per_second=340.562M/s
// BM_c256Sub/min_time:1.000              2.10 ns         2.10 ns    683291397 items_per_second=476.669M/s
// BM_c256Mul/min_time:1.000              16.3 ns         16.3 ns     88013826 items_per_second=61.2835M/s
// BM_c256Sqr/min_time:1.000              13.1 ns         12.9 ns    106685084 items_per_second=77.2475M/s
// BM_c256Inv/min_time:1.000              4289 ns         4288 ns       326004 items_per_second=233.194k/s
// BM_ordMul/min_time:1.000               25.6 ns         25.6 ns     55066089 items_per_second=39.015M/s
// BM_ordAdd/min_time:1.000               2.08 ns         2.07 ns    673114763 items_per_second=482.45M/s
// BM_ordInv/min_time:1.000               8056 ns         8053 ns       170454 items_per_second=124.182k/s
// BM_pointAdd                             300 ns          300 ns      2130292 items_per_second=3.33546M/s
// BM_pointDouble                          158 ns          157 ns      4656050 items_per_second=6.37395M/s
// BM_scalarBaseMult/min_time:1.000       7590 ns         7570 ns       179677 items_per_second=132.099k/s
// BM_scalarMult/min_time:1.000          61629 ns        61592 ns        23012 items_per_second=16.2358k/s
#[cfg(test)]
mod tests {}
