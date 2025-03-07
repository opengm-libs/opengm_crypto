use core::{arch::aarch64::*, mem::transmute};

// a Digest context with 4 lanes to computing 4 msgs one time.
pub struct Digest{
    states: [uint32x4_t; 8],
    lengths: [u64;4],
}


impl Digest{
    fn dump(&self)-> [[u8;32];4]{
        unsafe {
            let mut d = [[0; 32]; 4];
            // tmp:
            // iv00 iv01 iv02 iv03 <- iv0
            // iv10 iv11 iv12 iv13
            // iv20 iv21 iv22 iv23
            // iv30 iv31 iv32 iv33
            // iv04 iv05 iv06 iv07 <- iv0
            // ...
            let tmp = [0u8; 4 * 32];
            vst4q_u32(tmp.as_ptr() as *mut u32, transmute([self.states[0],self.states[1],self.states[2],self.states[3]]));
            vst4q_u32((&tmp[64..]).as_ptr() as *mut u32, transmute([self.states[4],self.states[5],self.states[6],self.states[7]]));
          
            d[0][0..16].copy_from_slice(&tmp[0..16]);
            d[0][16..32].copy_from_slice(&tmp[64..64 + 16]);
            d[1][0..16].copy_from_slice(&tmp[16..32]);
            d[1][16..32].copy_from_slice(&tmp[64 + 16..64 + 32]);
            d[2][0..16].copy_from_slice(&tmp[32..48]);
            d[2][16..32].copy_from_slice(&tmp[64 + 32..64 + 48]);
            d[3][0..16].copy_from_slice(&tmp[48..64]);
            d[3][16..32].copy_from_slice(&tmp[64 + 48..64 + 64]);

            d
        }
    }

    // write data to the lane idx
    fn write(_idx: usize, _data: &[u8]){

    }
}



