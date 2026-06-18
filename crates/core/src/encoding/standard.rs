use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

use crate::error::FormatError;
use crate::payload::Payload;

/// The text prefix for a given binary version, e.g. version 1 => "OSM1.".
pub fn standard_prefix(version: u8) -> String {
    format!("OSM{version}.")
}

pub fn encode_standard(payload: &Payload) -> String {
    let body = URL_SAFE_NO_PAD.encode(payload.to_bytes());
    format!("{}{}", standard_prefix(payload.version), body)
}

pub fn decode_standard(s: &str) -> Result<Payload, FormatError> {
    let s = s.trim();
    // Expect "OSM" + one ASCII digit version + "." + base64url body.
    let rest = s.strip_prefix("OSM").ok_or(FormatError::Malformed)?;
    let (ver_str, body) = rest.split_once('.').ok_or(FormatError::Malformed)?;
    if ver_str.len() != 1 || !ver_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(FormatError::Malformed);
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(body.as_bytes())
        .map_err(|_| FormatError::Malformed)?;
    Payload::from_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Argon2Params;

    fn sample() -> Payload {
        Payload {
            version: Payload::CURRENT_VERSION,
            params: Argon2Params::default(),
            salt: [5u8; 16],
            nonce: [6u8; 12],
            ciphertext: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
        }
    }

    #[test]
    fn roundtrip() {
        let p = sample();
        let s = encode_standard(&p);
        assert!(s.starts_with("OSM1."));
        assert_eq!(decode_standard(&s).unwrap(), p);
    }

    #[test]
    fn tolerates_surrounding_whitespace() {
        let s = format!("  {}\n", encode_standard(&sample()));
        assert_eq!(decode_standard(&s).unwrap(), sample());
    }

    #[test]
    fn rejects_missing_prefix() {
        assert_eq!(decode_standard("hello world"), Err(FormatError::Malformed));
    }

    #[test]
    fn rejects_bad_base64() {
        assert_eq!(decode_standard("OSM1.!!!notbase64!!!"), Err(FormatError::Malformed));
    }
}
