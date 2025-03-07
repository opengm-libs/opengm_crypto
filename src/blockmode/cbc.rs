use crate::traits::Block;
use super::{Error, Result};

pub struct CBCMode<B:Block>{
    pub block_size: usize,
    pub block: B,
}

impl<B:Block> CBCMode<B>{
    pub fn new(block: B) -> Self{
        CBCMode{
            block_size: block.block_size(),
            block: block,
        }
    }

    pub fn encrypt_inplace(&self, iv: &[u8], in_out: &mut [u8])-> Result<()>{
        let block = &self.block;
        let block_size = block.block_size();

        if in_out.len() % block_size != 0 ||iv.len() % block_size != 0 {
            return Err(Error::InvalidInputSize);
        }
        if in_out.len() == 0{
            return Ok(());
        }

        assert!(block_size <= 32);
        let mut buf = [0;32];
        let buf = &mut buf[0..block_size];

        let mut last_chunk = iv;
        for chunk  in in_out.chunks_mut(block_size){
            for i in 0..block_size{
                buf[i] = last_chunk[i] ^ chunk[i];
            }
            self.block.encrypt( chunk, buf);
            last_chunk = chunk;
        }
        Ok(())
    }

    pub fn decrypt_inplace(&self,iv: &[u8], in_out: &mut [u8])-> Result<()>{
        let block = &self.block;
        let block_size = block.block_size();

        if in_out.len() % block_size != 0 ||iv.len() % block_size != 0 {
            return Err(Error::InvalidInputSize)
        }
        if in_out.len() == 0{
            return Ok(());
        }

        assert!(block_size <= 32);
        let mut buf_iv = [0;32];
        let buf_iv = &mut buf_iv[0..block_size];
        
        let mut plain = [0;16];
        
        buf_iv.copy_from_slice(iv);
        for chunk  in in_out.chunks_mut(block_size){
            self.block.decrypt( &mut plain,chunk);
            for i in 0..block_size{
                plain[i] = plain[i] ^ buf_iv[i];
            }
            buf_iv.copy_from_slice(chunk);
            chunk.copy_from_slice(&plain);
        }
        Ok(())
    }

}


#[cfg(test)]
mod tests{
    use hex_literal::hex;

    use crate::sm4::Cipher;

    use super::CBCMode;

    #[test]
    fn test_cbc(){
        let key = hex!("D54B4C962526A7A6F873695DF032BF21");
        let iv = hex!("C5FBC0E3B1F7324E256A827F91CC0D3E");
        let mut plain = hex!("7B5BD9FDAE2521A3F0FBDD2F4427142F785C52080B0DB22523C3BC5D8716D141CE315586EBB3EDF4480193B1B3C33524");
        let wanted = hex!("B4C72E13618BB3BBD69C7BEB8B545B49C50B279D28D89958898DF22CA79CE3478B75E6AB0151C83BAEBBFCF0B3ED91EA");

        let cbc = CBCMode::new(Cipher::new(&key));
        cbc.encrypt_inplace(&iv, &mut plain).unwrap();
        assert_eq!(plain, wanted);
        
        cbc.decrypt_inplace(&iv, &mut plain).unwrap();
        assert_eq!(plain, hex!("7B5BD9FDAE2521A3F0FBDD2F4427142F785C52080B0DB22523C3BC5D8716D141CE315586EBB3EDF4480193B1B3C33524"));
    }
}