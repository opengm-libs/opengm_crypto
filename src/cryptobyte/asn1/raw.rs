
const NULL:[u8;2] = [5,0];

#[derive(Debug)]
pub struct RawObject<'a> {
    // bit 1-5
    pub tag: u8,

    // bit 8 and bit 7
    pub class: u8,
    // bit 6
    pub is_compound: bool,

    // the Value of the ASN.1 object
    pub bytes: &'a [u8],

    // the TLV of the ASN.1 object
    pub full_bytes: &'a [u8],
}

// default RawObject represent a NULL object
impl<'a> Default for RawObject<'a> {
    fn default() -> Self {
        Self { 
            class: 0, 
            tag: 5, 
            is_compound: false, 
            bytes: &NULL[NULL.len()..NULL.len()], 
            full_bytes: &NULL, 
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cryptobyte::asn1::raw::RawObject;

    #[test]
    fn test_raw(){
        println!("{:?}", RawObject::default())
    }

}