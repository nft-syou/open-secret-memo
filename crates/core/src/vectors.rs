use serde::{Deserialize, Serialize};

use crate::params::Argon2Params;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorArgon2 {
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestVector {
    pub name: String,
    pub passphrase: String,
    pub plaintext_utf8: String,
    pub argon2: VectorArgon2,
    /// Hex of the fixed 16-byte salt used during generation.
    pub salt_hex: String,
    /// Hex of the fixed 12-byte nonce used during generation.
    pub nonce_hex: String,
    pub payload_hex: String,
    pub standard: String,
    pub words: String,
}

impl VectorArgon2 {
    pub fn to_params(&self) -> Argon2Params {
        Argon2Params { m_cost: self.m_cost, t_cost: self.t_cost, p_cost: self.p_cost }
    }
}

pub fn load_vectors(json: &str) -> Vec<TestVector> {
    serde_json::from_str(json).expect("valid test-vector.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decrypt, detect_and_decode, encode_standard, encode_words, encrypt, FixedRng};

    const VECTORS_JSON: &str = include_str!("../../../spec/test-vector.json");

    #[test]
    fn every_vector_reproduces_exactly() {
        for v in load_vectors(VECTORS_JSON) {
            // Re-derive salt+nonce deterministically: FixedRng emits salt bytes then nonce bytes.
            let salt = hex::decode(&v.salt_hex).unwrap();
            let nonce = hex::decode(&v.nonce_hex).unwrap();
            let mut seed = salt.clone();
            seed.extend_from_slice(&nonce);
            let mut rng = FixedRng::new(seed);

            let payload = encrypt(
                v.plaintext_utf8.as_bytes(),
                &v.passphrase,
                v.argon2.to_params(),
                &mut rng,
            );

            assert_eq!(hex::encode(payload.to_bytes()), v.payload_hex, "payload {}", v.name);
            assert_eq!(encode_standard(&payload), v.standard, "standard {}", v.name);
            assert_eq!(encode_words(&payload), v.words, "words {}", v.name);

            // And the decode paths recover the plaintext.
            assert_eq!(
                decrypt(&detect_and_decode(&v.standard).unwrap(), &v.passphrase).unwrap(),
                v.plaintext_utf8.as_bytes(),
                "decrypt standard {}",
                v.name
            );
            assert_eq!(
                decrypt(&detect_and_decode(&v.words).unwrap(), &v.passphrase).unwrap(),
                v.plaintext_utf8.as_bytes(),
                "decrypt words {}",
                v.name
            );
        }
    }
}
