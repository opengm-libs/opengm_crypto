use core::arch::asm;

use super::LIMB;

// Input: a = [a0, a1, a2, a3] < B^4
// Output: (a + a0*p)/B
// Note that (a + a0*p)/B < ((B^4-1) + (B-1)*(B^4-1))/B <= B^4 - 1.
#[inline(always)]
pub fn montgomery_reduce_limb_aarch64(a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB) -> (LIMB, LIMB, LIMB, LIMB) {
    let mut a0:LIMB = a0;
    let mut a1:LIMB = a1;
    let mut a2:LIMB = a2;
    let mut a3:LIMB = a3;

    unsafe {
        asm!(
            "lsl {t1}, {a0}, #32", 
            "lsr {t2}, {a0}, #32",
            "adds {a1}, {a0}, {a1}", 
            "adcs {a2}, {a2}, xzr", 
            "adcs {a3}, {a3}, xzr",
            "adcs {a0}, {a0}, xzr", 
            
            "subs {a1}, {a1}, {t1}",
            "sbcs {a2}, {a2}, {t2}", 
            "sbcs {a3}, {a3}, {t1}", 
            "sbcs {a0}, {a0}, {t2}", 
            a0 = inout(reg) a0,
            a1 = inout(reg) a1,
            a2 = inout(reg) a2,
            a3 = inout(reg) a3,
            t1 = out(reg) _,
            t2 = out(reg) _,
        );
    }
    (a1, a2, a3, a0)
}