use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::FormatError;
use crate::payload::Payload;

/// BIP-39 Japanese wordlist (2048 hiragana words), vendored at build time.
pub static WORDLIST: [&str; 2048] = {
    // Array literal generated from data/bip39-japanese.txt into data/wordlist_array.in, pasted here at compile time via include!.
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/wordlist_array.in"))
};

/// Index-aligned kanji "skin" of [`WORDLIST`] (frozen, 2048 entries). Each index
/// matches [`WORDLIST`]; entries that cannot be safely written in 常用漢字 stay in
/// the original BIP-39 hiragana (so the form is "漢字混じり"). Experimental.
pub static KANJI_WORDLIST: [&str; 2048] = {
    // Generated from data/bip39-japanese-kanji.txt into data/wordlist_kanji_array.in via include!.
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/wordlist_kanji_array.in"))
};

fn word_index() -> &'static HashMap<&'static str, u16> {
    static MAP: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    MAP.get_or_init(|| index_of(&WORDLIST))
}

fn kanji_word_index() -> &'static HashMap<&'static str, u16> {
    static MAP: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    MAP.get_or_init(|| index_of(&KANJI_WORDLIST))
}

fn index_of(wordlist: &'static [&'static str; 2048]) -> HashMap<&'static str, u16> {
    wordlist.iter().enumerate().map(|(i, w)| (*w, i as u16)).collect()
}

/// Encode a payload as a Japanese BIP-39 word sequence (hiragana, base-2048),
/// words joined by `、` (U+3001).
pub fn encode_words(payload: &Payload) -> String {
    encode_with(payload, &WORDLIST)
}

/// Same encoding using the kanji "skin" wordlist (experimental 漢字混じり form).
pub fn encode_words_kanji(payload: &Payload) -> String {
    encode_with(payload, &KANJI_WORDLIST)
}

fn encode_with(payload: &Payload, wordlist: &[&str; 2048]) -> String {
    let payload_bytes = payload.to_bytes();
    let mut bits: Vec<u8> = Vec::new();
    let len = payload_bytes.len() as u32;
    push_bytes(&mut bits, &len.to_be_bytes());
    push_bytes(&mut bits, &payload_bytes);
    // Pad to a multiple of 11 bits with zeros.
    while bits.len() % 11 != 0 {
        bits.push(0);
    }
    let words: Vec<&str> = bits
        .chunks(11)
        .map(|chunk| {
            let idx = chunk.iter().fold(0u16, |acc, &b| (acc << 1) | b as u16);
            wordlist[idx as usize]
        })
        .collect();
    words.join("\u{3001}") // "、"
}

/// Decode a Japanese BIP-39 word sequence (hiragana wordlist) back into a payload.
pub fn decode_words(s: &str) -> Result<Payload, FormatError> {
    decode_with(s, word_index())
}

/// Decode the kanji "skin" form.
pub fn decode_words_kanji(s: &str) -> Result<Payload, FormatError> {
    decode_with(s, kanji_word_index())
}

fn decode_with(s: &str, map: &HashMap<&'static str, u16>) -> Result<Payload, FormatError> {
    let mut bits: Vec<u8> = Vec::new();
    for token in s.split('\u{3001}') {
        let w = token.trim();
        if w.is_empty() {
            continue;
        }
        let idx = *map.get(w).ok_or_else(|| FormatError::InvalidWord(w.to_string()))?;
        for shift in (0..11).rev() {
            bits.push(((idx >> shift) & 1) as u8);
        }
    }
    let bytes = bits_to_bytes(&bits);
    if bytes.len() < 4 {
        return Err(FormatError::Malformed);
    }
    let len = u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;
    let payload_bytes = bytes.get(4..4 + len).ok_or(FormatError::Malformed)?;
    Payload::from_bytes(payload_bytes)
}

fn push_bytes(bits: &mut Vec<u8>, bytes: &[u8]) {
    for &byte in bytes {
        for shift in (0..8).rev() {
            bits.push((byte >> shift) & 1);
        }
    }
}

fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    bits.chunks(8)
        .filter(|c| c.len() == 8)
        .map(|c| c.iter().fold(0u8, |acc, &b| (acc << 1) | b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::Argon2Params;

    fn payload(ct_len: usize) -> Payload {
        Payload {
            version: Payload::CURRENT_VERSION,
            params: Argon2Params::default(),
            salt: [5u8; 16],
            nonce: [6u8; 12],
            ciphertext: (0..ct_len).map(|i| i as u8).collect(),
        }
    }

    #[test]
    fn wordlist_has_2048_unique_words() {
        let set: std::collections::HashSet<_> = WORDLIST.iter().collect();
        assert_eq!(set.len(), 2048);
    }

    #[test]
    fn roundtrip_various_lengths() {
        // 16..19 covers the byte counts where naive padding inference would fail.
        for ct_len in [16usize, 17, 18, 19, 32, 100] {
            let p = payload(ct_len);
            let words = encode_words(&p);
            assert_eq!(decode_words(&words).unwrap(), p, "ct_len={ct_len}");
        }
    }

    #[test]
    fn output_is_separated_by_ideographic_comma() {
        let words = encode_words(&payload(16));
        assert!(words.contains('\u{3001}'));
    }

    #[test]
    fn unknown_word_is_reported() {
        let err = decode_words("notarealword").unwrap_err();
        assert_eq!(err, FormatError::InvalidWord("notarealword".to_string()));
    }

    #[test]
    fn empty_input_is_malformed() {
        assert_eq!(decode_words(""), Err(FormatError::Malformed));
        assert_eq!(decode_words("   "), Err(FormatError::Malformed));
    }

    #[test]
    fn kanji_wordlist_is_2048_unique_and_index_aligned() {
        assert_eq!(KANJI_WORDLIST.len(), WORDLIST.len());
        let set: std::collections::HashSet<_> = KANJI_WORDLIST.iter().collect();
        assert_eq!(set.len(), 2048);
        // At least some entries were actually kanji-ified (differ from the hiragana list).
        assert!(KANJI_WORDLIST.iter().zip(WORDLIST.iter()).any(|(k, h)| k != h));
    }

    #[test]
    fn roundtrip_kanji_various_lengths() {
        for ct_len in [16usize, 17, 18, 19, 32, 100] {
            let p = payload(ct_len);
            let words = encode_words_kanji(&p);
            assert_eq!(decode_words_kanji(&words).unwrap(), p, "ct_len={ct_len}");
        }
    }
}
