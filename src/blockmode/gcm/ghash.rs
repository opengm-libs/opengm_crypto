pub trait GHash{
    fn init(&mut self, key: &[u8; 16]);
    fn reset(&mut self);

    // hash data, padding 0 if data is not of length of multiple of 128
    fn update(&mut self, data: &[u8]);
    fn update_u64x2(&mut self, a: u64, b: u64);
    fn sum(&self, h: &mut [u8; 16]);
}

// Note: For GCM standard, a 16 Bytes sequence represents a polynomial of degree 127 in GF(2)[x]
// The Bytes sequence is in little endian. 
// But in each byte is "big-endian". For example, 0x80, the first bit is 1, and 0x01, the 7-th bit is 1.
// Example:
// [0x80, 00, ..., 0x01] <=> x^127 + 1
// [b0, b1, b2, ..., b15]:
// coefficient of:
// x^0 = (b0 >> 7) & 1
// x^1 = (b0 >> 6) & 1
// ....
// x^126 = (b15 >> 1) & 1
// x^127 = b15 & 1
