use alloc::string::String;
#[allow(unused_imports)]
use alloc::fmt;
use crate::cryptobyte::{Result,Error};


// OBJECT IDENTIFIER
#[macro_export]
macro_rules! oid{
    ($($d:expr),*) => {
        $crate::cryptobyte::asn1::ObjectIdentifier::from_slice(&[$($d,)*]).unwrap()
    }
}

const OID_MAX_SIZE:usize = 63;

// An ObjectIdentifier represents an ASN.1 OBJECT IDENTIFIER.
// There is only one way to initialize a ObjectIdentifier
// let oid1 = 
#[derive(Debug)]
pub struct ObjectIdentifier{
    der: [u8; OID_MAX_SIZE],
    der_len: u8,
}

impl PartialEq for ObjectIdentifier{
    fn eq(&self, other: &Self) -> bool {
        self.der_len == other.der_len && 
        self.der[..self.der_len as usize] == other.der[..self.der_len as usize]
    }
}
impl Eq for ObjectIdentifier{}

impl Default for ObjectIdentifier{
    fn default() -> Self {
        Self { der: [0;OID_MAX_SIZE], der_len: Default::default() }
    }
}

impl fmt::Display for ObjectIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

fn parse_base10(chars: &[u8]) -> (u32, &[u8]){
    let mut n = 0u32;
    for (i, c) in chars.iter().enumerate(){
        if '0' as u8 <= *c && *c <= '9' as u8 {
            n = 10*n + (*c as u32);
        }else{
            return (n, &chars[i..])
        }
    }
    return (n, &chars[chars.len()..])
}

impl TryFrom<&str> for ObjectIdentifier {
    type Error = Error;
    fn try_from(value: &str) -> Result<Self> {
        let mut v = [0u32; 32];
        let mut i = 0;
        for s in value.split('.'){
            v[i] = s.parse::<u32>().map_err(|_| Error::ASN1InvalidOid)?;
            i += 1;
        }
        ObjectIdentifier::from_slice(&v[..i]).ok_or(Error::ASN1InvalidOid)
    }
}

impl From<&ObjectIdentifier> for String {
    fn from(oid: &ObjectIdentifier) -> Self {
        let mut s = String::with_capacity(32);

        if oid.der_len == 0 {
            return s;
        }
        let v = oid.der[0];
        s.push_str(&format!("{}.{}", v/40, v%40));

        let mut n = 0;
        // 0x81, 0x82, 0x03 => 1*128^2 + 2*128 + 3
        for x in oid.der[1..oid.der_len as usize].iter(){
            if *x & 0x80 == 0{
                n = (n<<7) + (*x) as u32;
                s.push_str(&format!(".{}", n));
                n = 0;
            }else{
                n = (n << 7) + ((*x) & 0x7f) as u32; 
            }
        }
        s
    }
}


impl ObjectIdentifier {
    pub fn to_string(&self) -> String {
        String::from(self)
    }

    /// Parses an OID from a slice u32, e.g. [1, 2, 840, 113549].
    pub const fn from_slice(parts: &[u32]) -> Option<ObjectIdentifier> {
        if parts.len() < 2{
            return None;
        }
        
        if parts[0] > 2 || (parts[0] < 2 && parts[1] >= 40) {
            return None;
        }

        let mut der_data = [(40 * parts[0] + parts[1]) as u8; OID_MAX_SIZE];
        let mut der_data_len = 1;
        
        let mut i = 2; 
        while i < parts.len() {
            let part = parts[i];
            
            if part == 0{
                der_data[der_data_len] = 0;
                der_data_len += 1;
                continue;
            }

            // How many bytes to encode part.
            let mut length = 32 - part.leading_zeros();
            length = (length + 6)/7;

            let mut j = length - 1; 
            while j > 0 { 
                der_data[der_data_len] = (0x80 | (part >> (7*j))) as u8; 
                der_data_len += 1;
                j -= 1;
            }
            der_data[der_data_len] = (part & 0x7f) as u8; 
            der_data_len += 1;
            i += 1;
        }

        Some(ObjectIdentifier {
            der: der_data,
            der_len: der_data_len as u8,
        })
    }

    // /// Parses an OID from a dotted string, e.g. `"1.2.840.113549"`.
    // pub fn from_string(s: &str) -> Option<ObjectIdentifier> {
    //     let mut parts = s.split('.');
    //     ObjectIdentifier::from_slice(parts.collect::<Vec<_>>().as_slice())
    // }
}


impl ObjectIdentifier {
    // ObjectIdentifier from a DER encoding bytes.
    pub fn try_from_asn1(v: &[u8]) -> Result<Self> {
        // TODO: check v
        let mut oid = ObjectIdentifier::default();
        oid.der[..v.len()].copy_from_slice(v);
        oid.der_len = v.len() as u8;
        Ok(oid)
    }

    pub fn as_der(&self) -> &[u8] {
        &self.der[..self.der_len as usize]
    }

    pub fn is_valid(&self) -> bool {
        if self.der_len < 2 {
            return false;
        }

        let (a0, a1) = (self.der[0] / 40, self.der[0] % 40);
        if a0 > 2 || (a0 <= 1 && a1 >= 40) {
            return false;
        }
        

        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_oi() {
        let a = ObjectIdentifier::from_slice(&[1, 2, 3]);
        println!("{:?}", a);
    }


    #[test]
    fn test_from() {
        let a = vec![0x2au8, 0x86, 0x48,0x86,0xf7,0x0d];
        let oid = ObjectIdentifier::try_from_asn1(a.as_slice()).unwrap();
        assert_eq!("1.2.840.113549", oid.to_string());
    }
}
