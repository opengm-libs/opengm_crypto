// A demonstration.
struct ConstRngCore{
    c: u64,
}
impl ConstRngCore{
    fn new(c:u64)-> Self{
        ConstRngCore{c}
    }
}


impl rand::RngCore for ConstRngCore{
    fn next_u32(&mut self) -> u32 {
        return self.c as u32;
    }

    fn next_u64(&mut self) -> u64 {
        return self.c;
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for x in dest{
            *x = self.c as u8;
        }
    }

    // fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
    //     self.fill_bytes(dest);
    //     Ok(())
    // }
}



#[cfg(test)]
mod tests{
    use super::ConstRngCore;
    // impl rand::RngCore, then we automatically get rand::Rng.
    use rand::Rng;
    // use rand::rngs::OsRng;

    #[test]
    fn test_const_rng() {
        let mut rng = ConstRngCore::new(1);
        // let mut rng = OsRng::default();
        let s:[u8;10] = rng.random();
        println!("{:?}", s);
    }
}