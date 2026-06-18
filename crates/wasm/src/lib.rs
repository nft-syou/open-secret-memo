use wasm_bindgen::prelude::*;

use osm_core::{
    decrypt as core_decrypt, detect_and_decode, encode_standard, encode_words, encrypt as core_encrypt,
    Argon2Params, DecryptError, FormatError, OsRng,
};

/// Result of a decrypt attempt. `ok` distinguishes success from a recoverable
/// error; on failure `error_kind` is one of the stable kind strings.
#[wasm_bindgen(getter_with_clone)]
pub struct DecryptOutcome {
    pub ok: bool,
    pub text: String,
    pub error_kind: String,
    pub error_word: String,
}

#[wasm_bindgen]
pub fn encrypt(
    plaintext: &str,
    passphrase: &str,
    m_cost: u32,
    t_cost: u32,
    p_cost: u8,
    format: &str,
) -> Result<String, JsError> {
    let params = Argon2Params { m_cost, t_cost, p_cost };
    params.validate().map_err(|e| JsError::new(&e.to_string()))?;
    let mut rng = OsRng;
    let payload = core_encrypt(plaintext.as_bytes(), passphrase, params, &mut rng);
    match format {
        "standard" => Ok(encode_standard(&payload)),
        "words" => Ok(encode_words(&payload)),
        other => Err(JsError::new(&format!("unknown format: {other}"))),
    }
}

#[wasm_bindgen]
pub fn decrypt(ciphertext: &str, passphrase: &str) -> DecryptOutcome {
    let payload = match detect_and_decode(ciphertext) {
        Ok(p) => p,
        Err(e) => return from_format_error(e),
    };
    match core_decrypt(&payload, passphrase) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => DecryptOutcome { ok: true, text, error_kind: String::new(), error_word: String::new() },
            Err(_) => err("not_utf8", ""),
        },
        Err(DecryptError::AuthenticationFailed) => err("auth_failed", ""),
        Err(DecryptError::Format(e)) => from_format_error(e),
    }
}

fn from_format_error(e: FormatError) -> DecryptOutcome {
    match e {
        FormatError::Malformed => err("malformed", ""),
        FormatError::UnsupportedVersion(v) => err("unsupported_version", &v.to_string()),
        FormatError::InvalidWord(w) => err("invalid_word", &w),
    }
}

fn err(kind: &str, word: &str) -> DecryptOutcome {
    DecryptOutcome { ok: false, text: String::new(), error_kind: kind.to_string(), error_word: word.to_string() }
}
