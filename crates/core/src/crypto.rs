use aes_gcm::aead::{Aead, KeyInit, Payload as AeadPayload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params as A2Params, Version};
use unicode_normalization::UnicodeNormalization;

use crate::error::DecryptError;
use crate::params::Argon2Params;
use crate::payload::Payload;
use crate::rng::Rng;

/// NFKC-normalize, trim, and return UTF-8 bytes of a passphrase. Applies to the
/// passphrase ONLY — never to the memo body.
pub fn normalize_passphrase(passphrase: &str) -> Vec<u8> {
    let normalized: String = passphrase.nfkc().collect();
    normalized.trim().as_bytes().to_vec()
}

fn derive_key(passphrase: &str, salt: &[u8], params: &Argon2Params) -> [u8; 32] {
    let a2 = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        A2Params::new(params.m_cost, params.t_cost, params.p_cost as u32, Some(32))
            .expect("validated params"),
    );
    let pwd = normalize_passphrase(passphrase);
    let mut key = [0u8; 32];
    a2.hash_password_into(&pwd, salt, &mut key)
        .expect("argon2 key derivation");
    key
}

pub fn encrypt(
    plaintext: &[u8],
    passphrase: &str,
    params: Argon2Params,
    rng: &mut impl Rng,
) -> Payload {
    let mut salt = [0u8; 16];
    rng.fill(&mut salt);
    let mut nonce = [0u8; 12];
    rng.fill(&mut nonce);

    // Build the payload header first so it can be authenticated as AAD.
    let mut payload = Payload {
        version: Payload::CURRENT_VERSION,
        params,
        salt,
        nonce,
        ciphertext: Vec::new(),
    };
    let aad = payload.header();

    let key = derive_key(passphrase, &salt, &params);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), AeadPayload { msg: plaintext, aad: &aad })
        .expect("aes-gcm encryption");
    payload.ciphertext = ct;
    payload
}

pub fn decrypt(payload: &Payload, passphrase: &str) -> Result<Vec<u8>, DecryptError> {
    let aad = payload.header();
    let key = derive_key(passphrase, &payload.salt, &payload.params);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    cipher
        .decrypt(
            Nonce::from_slice(&payload.nonce),
            AeadPayload { msg: &payload.ciphertext, aad: &aad },
        )
        .map_err(|_| DecryptError::AuthenticationFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::FixedRng;

    fn fast_params() -> Argon2Params {
        // Small memory keeps tests fast while exercising the real KDF.
        Argon2Params { m_cost: 8 * 1024, t_cost: 1, p_cost: 1 }
    }

    #[test]
    fn roundtrip() {
        let mut rng = FixedRng::new(vec![7u8]);
        let p = encrypt(b"secret note", "\u{7d19}\u{888b}", fast_params(), &mut rng);
        assert_eq!(decrypt(&p, "\u{7d19}\u{888b}").unwrap(), b"secret note");
    }

    #[test]
    fn wrong_passphrase_fails() {
        let mut rng = FixedRng::new(vec![7u8]);
        let p = encrypt(b"secret note", "correct", fast_params(), &mut rng);
        assert_eq!(decrypt(&p, "wrong"), Err(DecryptError::AuthenticationFailed));
    }

    #[test]
    fn tampered_header_fails() {
        let mut rng = FixedRng::new(vec![7u8]);
        let mut p = encrypt(b"secret note", "pw", fast_params(), &mut rng);
        // Flip an Argon2 param byte — AAD mismatch must surface as auth failure.
        p.params.t_cost = 2;
        assert_eq!(decrypt(&p, "pw"), Err(DecryptError::AuthenticationFailed));
    }

    #[test]
    fn nfkc_halfwidth_fullwidth_equivalent() {
        // Half-width katakana normalizes to full-width under NFKC.
        assert_eq!(normalize_passphrase("\u{ff76}\u{ff85}"), normalize_passphrase("\u{30ab}\u{30ca}"));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize_passphrase("  hello  "), b"hello".to_vec());
    }

    #[test]
    fn empty_plaintext_roundtrips() {
        let mut rng = FixedRng::new(vec![3u8]);
        let p = encrypt(b"", "pw", fast_params(), &mut rng);
        assert_eq!(decrypt(&p, "pw").unwrap(), b"");
    }
}
