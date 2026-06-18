mod standard;
mod words;
mod detect;

pub use standard::{decode_standard, encode_standard, standard_prefix};
pub use words::{decode_words, encode_words, WORDLIST};
pub use detect::detect_and_decode;
