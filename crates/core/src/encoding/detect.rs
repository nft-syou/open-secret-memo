use crate::error::FormatError;
use crate::payload::Payload;

use super::standard::decode_standard;
use super::words::{decode_words, decode_words_kanji};

/// Decode any supported text representation. Routing:
/// - starts with "OSM" + digit + "." => standard form
/// - otherwise => Japanese wordlist form (hiragana), falling back to the kanji
///   "skin" form. The two word tables share indices, so a pure-kana-fallback
///   string decodes identically under either; a string containing any kanji
///   token fails the hiragana table and succeeds under the kanji table.
pub fn detect_and_decode(s: &str) -> Result<Payload, FormatError> {
    let t = s.trim();
    if is_standard(t) {
        decode_standard(t)
    } else {
        decode_words(t).or_else(|_| decode_words_kanji(t))
    }
}

fn is_standard(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 5
        && &bytes[0..3] == b"OSM"
        && bytes[3].is_ascii_digit()
        && bytes[4] == b'.'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_standard, encode_words, encode_words_kanji};
    use crate::params::Argon2Params;

    fn sample() -> Payload {
        Payload {
            version: Payload::CURRENT_VERSION,
            params: Argon2Params::default(),
            salt: [5u8; 16],
            nonce: [6u8; 12],
            ciphertext: vec![1u8; 20],
        }
    }

    #[test]
    fn routes_standard() {
        let s = encode_standard(&sample());
        assert_eq!(detect_and_decode(&s).unwrap(), sample());
    }

    #[test]
    fn routes_words() {
        let s = encode_words(&sample());
        assert_eq!(detect_and_decode(&s).unwrap(), sample());
    }

    #[test]
    fn routes_kanji() {
        let s = encode_words_kanji(&sample());
        assert_eq!(detect_and_decode(&s).unwrap(), sample());
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(detect_and_decode("this is not a ciphertext at all").is_err());
    }
}
