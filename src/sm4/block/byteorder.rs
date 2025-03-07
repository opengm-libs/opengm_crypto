#[inline(always)]
pub(crate) fn get_u32_le(src: &[u8]) -> u32 {
    u32::from_le_bytes(src[..4].try_into().unwrap())
}

#[inline(always)]
pub(crate) fn put_u32_le(dst: &mut [u8], a: u32) {
    [dst[0], dst[1], dst[2], dst[3]] = a.to_le_bytes();
}

#[inline(always)]
pub(crate) fn get_u32_be(src: &[u8]) -> u32 {
    u32::from_be_bytes(src[..4].try_into().unwrap())
}

#[inline(always)]
pub(crate) fn put_u32_be(dst: &mut [u8], a: u32) {
    [dst[0], dst[1], dst[2], dst[3]] = a.to_be_bytes();
}
