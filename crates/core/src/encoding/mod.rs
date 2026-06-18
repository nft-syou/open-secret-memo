mod standard;
mod words;

pub use standard::{decode_standard, encode_standard, standard_prefix};
pub use words::{decode_words, encode_words, WORDLIST};
