use core::arch::asm;

use super::LIMB;


// a + b mod m.
#[inline(always)]
pub fn add256_mod_aarch64(
    a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB, 
    b0: LIMB, b1: LIMB, b2: LIMB, b3: LIMB, 
    m0: LIMB, m1: LIMB, m2: LIMB, m3: LIMB) -> (LIMB, LIMB, LIMB, LIMB) {

    let mut acc0:LIMB;
    let mut acc1:LIMB;
    let mut acc2:LIMB;
    let mut acc3:LIMB;
    
    unsafe{
        asm!(
            "adds  {acc0}, {a0}, {b0}",
            "adcs  {acc1}, {a1}, {b1}",
            "adcs  {acc2}, {a2}, {b2}",
            "adcs  {acc3}, {a3}, {b3}",
            "adcs  {carry}, xzr, xzr",

            "subs {t0}, {acc0}, {m0}",
            "sbcs {t1}, {acc1}, {m1}",
            "sbcs {t2}, {acc2}, {m2}",
            "sbcs {t3}, {acc3}, {m3}",
            "sbcs {carry}, {carry}, xzr",

            "csel {acc0}, {acc0}, {t0}, cc",
            "csel {acc1}, {acc1}, {t1}, cc",
            "csel {acc2}, {acc2}, {t2}, cc",
            "csel {acc3}, {acc3}, {t3}, cc",
            a0 = in(reg) a0,
            a1 = in(reg) a1,
            a2 = in(reg) a2,
            a3 = in(reg) a3,
            b0 = in(reg) b0,
            b1 = in(reg) b1,
            b2 = in(reg) b2,
            b3 = in(reg) b3,
            carry = out(reg) _,
            m0 = in(reg) m0,
            m1 = in(reg) m1,
            m2 = in(reg) m2,
            m3 = in(reg) m3,
            t0 = out(reg) _,
            t1 = out(reg) _,
            t2 = out(reg) _,
            t3 = out(reg) _,
            acc0 = out(reg) acc0,
            acc1 = out(reg) acc1,
            acc2 = out(reg) acc2,
            acc3 = out(reg) acc3,
        )
    }
    (acc0, acc1, acc2,acc3)
}



// sub m if necessary.
// [a0, a1, a2, a3, carry] - m
#[inline(always)]
pub fn sub256_conditional_aarch64(a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB, carry: LIMB, m0: LIMB, m1: LIMB, m2: LIMB, m3: LIMB) -> (LIMB, LIMB, LIMB, LIMB) {
    let mut b0:LIMB;
    let mut b1:LIMB;
    let mut b2:LIMB;
    let mut b3:LIMB;
    unsafe{
        asm!(
            "subs {b0}, {a0}, {m0}",
            "sbcs {b1}, {a1}, {m1}",
            "sbcs {b2}, {a2}, {m2}",
            "sbcs {b3}, {a3}, {m3}",
            "sbcs {carry}, {carry}, xzr",
            // if acc[4,5,0,1,2] >= p, then return t
            "csel {b0}, {b0}, {a0}, cs",
            "csel {b1}, {b1}, {a1}, cs",
            "csel {b2}, {b2}, {a2}, cs",
            "csel {b3}, {b3}, {a3}, cs",
            a0 = in(reg) a0,
            a1 = in(reg) a1,
            a2 = in(reg) a2,
            a3 = in(reg) a3,
            carry = in(reg) carry,
            m0 = in(reg) m0,
            m1 = in(reg) m1,
            m2 = in(reg) m2,
            m3 = in(reg) m3,
            b0 = out(reg) b0,
            b1 = out(reg) b1,
            b2 = out(reg) b2,
            b3 = out(reg) b3,
        )
    }
    (b0, b1, b2,b3)
}



#[inline(always)]
pub fn mul256_aarch64(a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB, b0: LIMB, b1: LIMB, b2: LIMB, b3: LIMB) -> (LIMB, LIMB, LIMB, LIMB, LIMB, LIMB, LIMB, LIMB) {
    let mut acc0:LIMB;
    let mut acc1:LIMB;
    let mut acc2:LIMB;
    let mut acc3:LIMB;
    let mut acc4:LIMB;
    let mut acc5:LIMB;
    let mut acc6:LIMB;
    let mut acc7:LIMB;

    unsafe {
        // a[0:3] = a[0:3] * b[0:3]
        asm! (
            // {b0} * a
            "mul   {acc0}, {b0}, {a0}",
            "umulh {acc1}, {b0}, {a0}",

            "mul   {t0}, {b0}, {a1}",
            "adds  {acc1}, {t0}, {acc1}",
            "umulh {acc2}, {b0}, {a1}",

            "mul   {t0}, {b0}, {a2}",
            "adcs  {acc2}, {t0}, {acc2}",
            "umulh {acc3}, {b0}, {a2}",

            "mul   {t0}, {b0}, {a3}",
            "adcs  {acc3}, {t0}, {acc3}",
            "umulh {acc4}, {b0}, {a3}",
            "adcs  {acc4}, {acc4}, xzr",

            // {b1} * a
            "mul   {t0}, {b1}, {a0}",
            "adds  {acc1}, {acc1}, {t0}",
            "umulh {t1}, {b1}, {a0}",

            "mul   {t0}, {b1}, {a1}",
            "adcs  {acc2}, {acc2}, {t0}",
            "umulh {t2}, {b1}, {a1}",

            "mul   {t0}, {b1}, {a2}",
            "adcs  {acc3}, {acc3}, {t0}",
            "umulh {t3}, {b1}, {a2}",

            "mul   {t0}, {b1}, {a3}",
            "adcs  {acc4}, {acc4}, {t0}",
            "umulh {acc5}, {b1}, {a3}",
            "adcs  {acc5}, {acc5}, xzr",

            "adds  {acc2}, {acc2}, {t1}",
            "adcs  {acc3}, {acc3}, {t2}",
            "adcs  {acc4}, {acc4}, {t3}",
            "adcs  {acc5}, {acc5}, xzr",

            // {b2} * a
            "mul   {t0}, {b2}, {a0}",
            "adds  {acc2}, {acc2}, {t0}",
            "umulh {t1}, {b2}, {a0}",

            "mul   {t0}, {b2}, {a1}",
            "adcs  {acc3}, {acc3}, {t0}",
            "umulh {t2}, {b2}, {a1}",

            "mul   {t0}, {b2}, {a2}",
            "adcs  {acc4}, {acc4}, {t0}",
            "umulh {t3}, {b2}, {a2}",

            "mul   {t0}, {b2}, {a3}",
            "adcs  {acc5}, {acc5}, {t0}",
            "umulh {acc6}, {b2}, {a3}",
            "adcs  {acc6}, {acc6}, xzr",

            "adds  {acc3}, {acc3}, {t1}",
            "adcs  {acc4}, {acc4}, {t2}",
            "adcs  {acc5}, {acc5}, {t3}",
            "adcs  {acc6}, {acc6}, xzr",

            // {b3} * a
            "mul   {t0}, {b3}, {a0}",
            "adds  {acc3}, {acc3}, {t0}",
            "umulh {t1}, {b3}, {a0}",

            "mul   {t0}, {b3}, {a1}",
            "adcs  {acc4}, {acc4}, {t0}",
            "umulh {t2}, {b3}, {a1}",

            "mul   {t0}, {b3}, {a2}",
            "adcs  {acc5}, {acc5}, {t0}",
            "umulh {t3}, {b3}, {a2}",

            "mul   {t0}, {b3}, {a3}",
            "adcs  {acc6}, {acc6}, {t0}",
            "umulh {acc7}, {b3}, {a3}",
            "adcs  {acc7}, {acc7}, xzr",

            "adds  {acc4}, {acc4}, {t1}",
            "adcs  {acc5}, {acc5}, {t2}",
            "adcs  {acc6}, {acc6}, {t3}",
            "adcs  {acc7}, {acc7}, xzr",
            a0 = in(reg) a0,
            a1 = in(reg) a1,
            a2 = in(reg) a2,
            a3 = in(reg) a3,
            b0 = in(reg) b0,
            b1 = in(reg) b1,
            b2 = in(reg) b2,
            b3 = in(reg) b3,
            t0 = out(reg) _,
            t1 = out(reg) _,
            t2 = out(reg) _,
            t3 = out(reg) _,
            acc0 = out(reg) acc0, 
            acc1 = out(reg) acc1, 
            acc2 = out(reg) acc2, 
            acc3 = out(reg) acc3, 
            acc4 = out(reg) acc4, 
            acc5 = out(reg) acc5, 
            acc6 = out(reg) acc6, 
            acc7 = out(reg) acc7, 
        );
    }
    (acc0, acc1, acc2, acc3, acc4, acc5,acc6, acc7)
}



#[inline(always)]
pub fn square256_aarch64(a0: LIMB, a1: LIMB, a2: LIMB, a3: LIMB) -> (LIMB, LIMB, LIMB, LIMB, LIMB, LIMB, LIMB, LIMB) {
    let mut acc0:LIMB;
    let mut acc1:LIMB;
    let mut acc2:LIMB;
    let mut acc3:LIMB;
    let mut acc4:LIMB;
    let mut acc5:LIMB;
    let mut acc6:LIMB;
    let mut acc7:LIMB;

    unsafe {
        asm! (
            // {acc}[1,2,3,4] = a[1,2,3] * a[0]
            "mul     {acc1}, {a0}, {a1}",
            "umulh   {acc2}, {a0}, {a1}",
            "mul     {t0}, {a0}, {a2}",
            "adds    {acc2}, {acc2}, {t0}",
            "umulh   {acc3}, {a0}, {a2}",
            "mul     {t0}, {a0}, {a3}",
            "adcs    {acc3}, {acc3}, {t0}",
            "umulh   {acc4}, {a0}, {a3}",
            "adcs    {acc4}, {acc4}, xzr",

            // {acc}[1,2,3,4,5] += a[2,3] * a[1]
            "mul     {t0}, {a1}, {a2}",
            "adds    {acc3}, {acc3}, {t0}",
            "umulh   {t1}, {a1}, {a2}",
            "adcs    {acc4}, {acc4}, {t1}",
            "adcs    {acc5}, xzr, xzr",

            "mul     {t0}, {a1}, {a3}",
            "adds    {acc4}, {acc4}, {t0}",
            "umulh   {t1}, {a1}, {a3}",
            "adcs    {acc5}, {acc5}, {t1} ",
            // no carry for {acc6}. because
            // a[1,2,3] * a[0] + a[2,3] * a[1] <= ?

            // {acc}[1,2,3,4,5,6,7] += a[3] * a[2]
            "mul     {t0}, {a3}, {a2}",
            "adds    {acc5}, {acc5}, {t0}",
            "umulh   {acc6}, {a3}, {a2}",
            "adcs    {acc6}, {acc6}, xzr", // no carry for acc7

            // {acc}[1,2,3,4,5,6,7] * 2
            "adcs    {acc1}, {acc1}, {acc1}",
            "adcs    {acc2}, {acc2}, {acc2}",
            "adcs    {acc3}, {acc3}, {acc3}",
            "adcs    {acc4}, {acc4}, {acc4}",
            "adcs    {acc5}, {acc5}, {acc5}",
            "adcs    {acc6}, {acc6}, {acc6}",
            "adcs    {acc7}, xzr, xzr",

            // add a[i]^2
            "mul     {acc0}, {a0}, {a0}",
            "umulh   {t0}, {a0}, {a0}",
            "adds    {acc1}, {acc1}, {t0}",

            "mul     {t0}, {a1}, {a1}",
            "adcs    {acc2}, {acc2}, {t0}",
            "umulh   {t1}, {a1}, {a1}",
            "adcs    {acc3}, {acc3}, {t1}",

            "mul     {t0}, {a2}, {a2}",
            "adcs    {acc4}, {acc4}, {t0}",
            "umulh   {t1}, {a2}, {a2}",
            "adcs    {acc5}, {acc5}, {t1}",

            "mul     {t0}, {a3}, {a3}",
            "adcs    {acc6}, {acc6}, {t0}",
            "umulh   {t1}, {a3}, {a3}",
            "adcs    {acc7}, {acc7}, {t1}",
            a0 = in(reg) a0,
            a1 = in(reg) a1,
            a2 = in(reg) a2,
            a3 = in(reg) a3,
            t0 = out(reg) _,
            t1 = out(reg) _,
            acc0 = out(reg) acc0, 
            acc1 = out(reg) acc1, 
            acc2 = out(reg) acc2, 
            acc3 = out(reg) acc3, 
            acc4 = out(reg) acc4, 
            acc5 = out(reg) acc5, 
            acc6 = out(reg) acc6, 
            acc7 = out(reg) acc7, 
        );
    }
    (acc0, acc1, acc2, acc3, acc4, acc5,acc6, acc7)
}


#[cfg(test)]
mod tests {
    use core::arch::asm;

    #[test]
    fn test_madd(){
        let mut a = 2u64;
        let b = 4u64;
        let c = 6u64;
        unsafe {
            asm!{
                // 2*4+6
                "madd {a}, {a}, {b}, {c}", 
                a = inout(reg) a,
                b = in(reg) b,
                c = in(reg) c,
            }
        }
        println!("{}",a);
    }

}