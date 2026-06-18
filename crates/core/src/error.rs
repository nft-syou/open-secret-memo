use thiserror::Error;

/// Errors returned when decoding a text representation into a [`crate::Payload`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("input is not a recognized Open Secret Memo ciphertext")]
    Malformed,
    #[error("unsupported binary format version: {0}")]
    UnsupportedVersion(u8),
    #[error("word not found in wordlist: {0}")]
    InvalidWord(String),
}

/// Errors returned by [`crate::decrypt`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecryptError {
    #[error(transparent)]
    Format(#[from] FormatError),
    #[error("authentication failed: wrong passphrase or corrupted ciphertext")]
    AuthenticationFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_error_converts_into_decrypt_error() {
        let d: DecryptError = FormatError::Malformed.into();
        assert_eq!(d, DecryptError::Format(FormatError::Malformed));
    }
}
