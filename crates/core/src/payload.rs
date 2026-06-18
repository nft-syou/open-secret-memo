use crate::params::Argon2Params;
use crate::error::FormatError;

pub const MAGIC: [u8; 3] = *b"OSM";
pub const HEADER_LEN: usize = 41;
const GCM_TAG_LEN: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    pub version: u8,
    pub params: Argon2Params,
    pub salt: [u8; 16],
    pub nonce: [u8; 12],
    /// AES-256-GCM ciphertext with the 16-byte tag appended.
    pub ciphertext: Vec<u8>,
}

impl Payload {
    pub const CURRENT_VERSION: u8 = 1;

    /// The 41-byte header used as AES-GCM additional authenticated data.
    /// Layout: magic(3) version(1) m_cost(4 BE) t_cost(4 BE) p_cost(1) salt(16) nonce(12).
    pub fn header(&self) -> [u8; HEADER_LEN] {
        let mut h = [0u8; HEADER_LEN];
        h[0..3].copy_from_slice(&MAGIC);
        h[3] = self.version;
        h[4..8].copy_from_slice(&self.params.m_cost.to_be_bytes());
        h[8..12].copy_from_slice(&self.params.t_cost.to_be_bytes());
        h[12] = self.params.p_cost;
        h[13..29].copy_from_slice(&self.salt);
        h[29..41].copy_from_slice(&self.nonce);
        h
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.header().to_vec();
        out.extend_from_slice(&self.ciphertext);
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Payload, FormatError> {
        // Need full header plus at least a GCM tag.
        if bytes.len() < HEADER_LEN + GCM_TAG_LEN {
            return Err(FormatError::Malformed);
        }
        if bytes[0..3] != MAGIC {
            return Err(FormatError::Malformed);
        }
        let version = bytes[3];
        if version != Self::CURRENT_VERSION {
            return Err(FormatError::UnsupportedVersion(version));
        }
        let m_cost = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        let t_cost = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
        let p_cost = bytes[12];
        let mut salt = [0u8; 16];
        salt.copy_from_slice(&bytes[13..29]);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&bytes[29..41]);
        let ciphertext = bytes[HEADER_LEN..].to_vec();
        Ok(Payload {
            version,
            params: Argon2Params { m_cost, t_cost, p_cost },
            salt,
            nonce,
            ciphertext,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Payload {
        Payload {
            version: Payload::CURRENT_VERSION,
            params: Argon2Params::default(),
            salt: [1u8; 16],
            nonce: [2u8; 12],
            ciphertext: vec![9u8; 32], // arbitrary, >= tag length
        }
    }

    #[test]
    fn roundtrip_bytes() {
        let p = sample();
        let bytes = p.to_bytes();
        assert_eq!(bytes.len(), HEADER_LEN + 32);
        assert_eq!(Payload::from_bytes(&bytes).unwrap(), p);
    }

    #[test]
    fn header_is_41_bytes_and_starts_with_magic() {
        let h = sample().header();
        assert_eq!(h.len(), 41);
        assert_eq!(&h[0..3], b"OSM");
        assert_eq!(h[3], 1);
    }

    #[test]
    fn rejects_short_input() {
        assert_eq!(Payload::from_bytes(&[0u8; 10]), Err(FormatError::Malformed));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = sample().to_bytes();
        bytes[0] = b'X';
        assert_eq!(Payload::from_bytes(&bytes), Err(FormatError::Malformed));
    }

    #[test]
    fn rejects_unknown_version() {
        let mut bytes = sample().to_bytes();
        bytes[3] = 9;
        assert_eq!(Payload::from_bytes(&bytes), Err(FormatError::UnsupportedVersion(9)));
    }
}
