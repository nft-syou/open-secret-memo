# Open Secret Memo — Core Crate + CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the dependency-minimal Rust `core` crate (Argon2id + AES-256-GCM, self-describing binary payload, standard + Japanese-wordlist text encodings, auto-detection) plus a CLI that exercises it, the canonical `test-vector.json`, and the byte-level `SPEC.md`.

**Architecture:** A Cargo workspace. `crates/core` is the single source of truth for all crypto and encoding, with no dependency on WASM or web concerns. The crypto path takes an injectable RNG so test vectors are deterministic. `crates/cli` is a thin binary over `core`. The same `test-vector.json` is consumed by `core`'s tests (proving correctness) and shipped in the recovery kit (proving re-implementability).

**Tech Stack:** Rust (2021 edition), `argon2`, `aes-gcm`, `base64`, `unicode-normalization`, `serde`/`serde_json`, `thiserror`, `rand_core`, `clap` (CLI), `proptest` (dev), `hex` (dev), `cargo-fuzz` (dev tooling).

## Global Constraints

- Rust edition: **2021**. Minimum toolchain: stable (no nightly features in `core`/`cli`; `cargo-fuzz` uses nightly only for the fuzz harness).
- `core` MUST NOT depend on any WASM, web, or async crate. Crypto crates + encoding + serde only.
- Binary integers are **big-endian**. Payload magic is the 3 bytes `OSM` (`0x4F 0x53 0x4D`). Binary format version starts at **1**.
- Argon2id defaults (OWASP): `m_cost = 65536` KiB (64 MiB), `t_cost = 1`, `p_cost = 1`. Derived key length = **32 bytes**.
- AES-256-GCM: 12-byte nonce, 16-byte tag appended to ciphertext. The 41-byte header (magic..nonce) is the **AAD**.
- Passphrase normalization (and ONLY the passphrase, never the memo): NFKC → trim → UTF-8 bytes.
- Crate names: workspace root `open-secret-memo`; member crates `osm-core`, `osm-cli`. Public API names are exact and reused verbatim by Plan 2.

---

### Task 1: Workspace skeleton, error types, public API surface

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/core/src/error.rs`
- Test: inline in `crates/core/src/error.rs`

**Interfaces:**
- Produces: `osm_core::DecryptError`, `osm_core::FormatError` enums; `crates/core` compiles as a library named `osm_core`.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/error.rs`:

```rust
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
```

- [ ] **Step 2: Create the workspace and crate manifests**

Create root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/core", "crates/cli"]
```

Create `crates/core/Cargo.toml`:

```toml
[package]
name = "osm-core"
version = "0.1.0"
edition = "2021"

[lib]
name = "osm_core"

[dependencies]
argon2 = "0.5"
aes-gcm = "0.10"
base64 = "0.22"
unicode-normalization = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
rand_core = "0.6"

[dev-dependencies]
proptest = "1"
hex = "0.4"
```

Create `crates/core/src/lib.rs`:

```rust
mod error;

pub use error::{DecryptError, FormatError};
```

(`crates/cli` is added in Task 11; until then the workspace `members` entry will fail to build. Add a placeholder so the workspace resolves:)

Create `crates/cli/Cargo.toml`:

```toml
[package]
name = "osm-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "osm"
path = "src/main.rs"

[dependencies]
osm-core = { path = "../core" }
```

Create `crates/cli/src/main.rs`:

```rust
fn main() {}
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test -p osm-core error::`
Expected: PASS (1 test).

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml crates/
git commit -m "feat(core): workspace skeleton and error types"
```

---

### Task 2: Argon2 parameters with defaults and validation

**Files:**
- Create: `crates/core/src/params.rs`
- Modify: `crates/core/src/lib.rs`
- Test: inline in `crates/core/src/params.rs`

**Interfaces:**
- Produces: `osm_core::Argon2Params { m_cost: u32, t_cost: u32, p_cost: u8 }`, `Argon2Params::default()` (OWASP values), `Argon2Params::validate(&self) -> Result<(), ParamError>`, and bound constants `M_COST_MIN`, `M_COST_MAX`, `T_COST_MIN`, `P_COST_MIN`.

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/params.rs`:

```rust
use thiserror::Error;

/// Minimum Argon2 memory cost in KiB (8 MiB).
pub const M_COST_MIN: u32 = 8 * 1024;
/// Maximum Argon2 memory cost in KiB (1 GiB) — guards against unusable browser memory requests.
pub const M_COST_MAX: u32 = 1024 * 1024;
pub const T_COST_MIN: u32 = 1;
pub const P_COST_MIN: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2Params {
    /// Memory cost in KiB.
    pub m_cost: u32,
    /// Number of iterations.
    pub t_cost: u32,
    /// Degree of parallelism (lanes).
    pub p_cost: u8,
}

impl Default for Argon2Params {
    fn default() -> Self {
        Argon2Params { m_cost: 65536, t_cost: 1, p_cost: 1 }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParamError {
    #[error("m_cost {0} KiB out of range [{M_COST_MIN}, {M_COST_MAX}]")]
    MCost(u32),
    #[error("t_cost {0} below minimum {T_COST_MIN}")]
    TCost(u32),
    #[error("p_cost {0} below minimum {P_COST_MIN}")]
    PCost(u8),
}

impl Argon2Params {
    pub fn validate(&self) -> Result<(), ParamError> {
        if self.m_cost < M_COST_MIN || self.m_cost > M_COST_MAX {
            return Err(ParamError::MCost(self.m_cost));
        }
        if self.t_cost < T_COST_MIN {
            return Err(ParamError::TCost(self.t_cost));
        }
        if self.p_cost < P_COST_MIN {
            return Err(ParamError::PCost(self.p_cost));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_owasp() {
        let p = Argon2Params::default();
        assert_eq!(p, Argon2Params { m_cost: 65536, t_cost: 1, p_cost: 1 });
        assert!(p.validate().is_ok());
    }

    #[test]
    fn rejects_low_memory() {
        let p = Argon2Params { m_cost: 1024, ..Default::default() };
        assert_eq!(p.validate(), Err(ParamError::MCost(1024)));
    }

    #[test]
    fn rejects_excessive_memory() {
        let p = Argon2Params { m_cost: M_COST_MAX + 1, ..Default::default() };
        assert_eq!(p.validate(), Err(ParamError::MCost(M_COST_MAX + 1)));
    }
}
```

- [ ] **Step 2: Wire into lib.rs**

Edit `crates/core/src/lib.rs` to add:

```rust
mod params;

pub use params::{Argon2Params, ParamError, M_COST_MAX, M_COST_MIN, P_COST_MIN, T_COST_MIN};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p osm-core params::`
Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/params.rs crates/core/src/lib.rs
git commit -m "feat(core): Argon2Params with defaults and validation"
```

---

### Task 3: Payload struct and binary serialization

**Files:**
- Create: `crates/core/src/payload.rs`
- Modify: `crates/core/src/lib.rs`
- Test: inline in `crates/core/src/payload.rs`

**Interfaces:**
- Consumes: `Argon2Params` (Task 2), `FormatError` (Task 1).
- Produces:
  - `osm_core::Payload { version: u8, params: Argon2Params, salt: [u8;16], nonce: [u8;12], ciphertext: Vec<u8> }`
  - `Payload::CURRENT_VERSION: u8 = 1`
  - `Payload::header(&self) -> [u8; 41]` (the AAD bytes)
  - `Payload::to_bytes(&self) -> Vec<u8>` (header followed by ciphertext)
  - `Payload::from_bytes(&[u8]) -> Result<Payload, FormatError>`
  - const `MAGIC: [u8;3]`, `HEADER_LEN: usize = 41`

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/payload.rs`:

```rust
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
```

- [ ] **Step 2: Wire into lib.rs**

Edit `crates/core/src/lib.rs` to add:

```rust
mod payload;

pub use payload::{Payload, HEADER_LEN, MAGIC};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p osm-core payload::`
Expected: PASS (5 tests).

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/payload.rs crates/core/src/lib.rs
git commit -m "feat(core): self-describing binary payload (de)serialization"
```

---

### Task 4: RNG abstraction + encrypt/decrypt (Argon2id + AES-256-GCM)

**Files:**
- Create: `crates/core/src/rng.rs`
- Create: `crates/core/src/crypto.rs`
- Modify: `crates/core/src/lib.rs`
- Test: inline in `crates/core/src/crypto.rs`

**Interfaces:**
- Consumes: `Payload`, `Argon2Params`, `DecryptError`.
- Produces:
  - `osm_core::Rng` trait: `fn fill(&mut self, buf: &mut [u8])`
  - `osm_core::OsRng` (production RNG) and `osm_core::FixedRng` (deterministic, for tests/vectors)
  - `osm_core::encrypt(plaintext: &[u8], passphrase: &str, params: Argon2Params, rng: &mut impl Rng) -> Payload`
  - `osm_core::decrypt(payload: &Payload, passphrase: &str) -> Result<Vec<u8>, DecryptError>`
  - `osm_core::normalize_passphrase(&str) -> Vec<u8>`

- [ ] **Step 1: Write the RNG abstraction**

Create `crates/core/src/rng.rs`:

```rust
/// Abstraction over randomness so tests and test-vector generation are deterministic.
pub trait Rng {
    fn fill(&mut self, buf: &mut [u8]);
}

/// Production RNG backed by the OS CSPRNG.
pub struct OsRng;

impl Rng for OsRng {
    fn fill(&mut self, buf: &mut [u8]) {
        use rand_core::{OsRng as Backing, RngCore};
        Backing.fill_bytes(buf);
    }
}

/// Deterministic RNG that emits a fixed byte sequence, cycling if exhausted.
/// ONLY for tests and test-vector generation — never for real encryption.
pub struct FixedRng {
    bytes: Vec<u8>,
    pos: usize,
}

impl FixedRng {
    pub fn new(bytes: Vec<u8>) -> Self {
        assert!(!bytes.is_empty(), "FixedRng needs at least one byte");
        FixedRng { bytes, pos: 0 }
    }
}

impl Rng for FixedRng {
    fn fill(&mut self, buf: &mut [u8]) {
        for b in buf.iter_mut() {
            *b = self.bytes[self.pos % self.bytes.len()];
            self.pos += 1;
        }
    }
}
```

- [ ] **Step 2: Write the failing crypto test**

Create `crates/core/src/crypto.rs`:

```rust
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
```

- [ ] **Step 3: Wire into lib.rs**

Edit `crates/core/src/lib.rs` to add:

```rust
mod crypto;
mod rng;

pub use crypto::{decrypt, encrypt, normalize_passphrase};
pub use rng::{FixedRng, OsRng, Rng};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p osm-core crypto::`
Expected: PASS (6 tests). Note: these use 8 MiB Argon2 memory and run in well under a second each.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/rng.rs crates/core/src/crypto.rs crates/core/src/lib.rs
git commit -m "feat(core): Argon2id + AES-256-GCM encrypt/decrypt with injectable RNG"
```

---

### Task 5: Standard text encoding (base64url + `OSM<v>.` prefix)

**Files:**
- Create: `crates/core/src/encoding/mod.rs`
- Create: `crates/core/src/encoding/standard.rs`
- Modify: `crates/core/src/lib.rs`
- Test: inline in `crates/core/src/encoding/standard.rs`

**Interfaces:**
- Consumes: `Payload`, `FormatError`.
- Produces:
  - `osm_core::encode_standard(&Payload) -> String`
  - `osm_core::decode_standard(&str) -> Result<Payload, FormatError>`
  - `osm_core::standard_prefix(version: u8) -> String` (e.g. `"OSM1."`)

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/encoding/standard.rs`:

```rust
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
```

- [ ] **Step 2: Create the encoding module and wire into lib.rs**

Create `crates/core/src/encoding/mod.rs`:

```rust
mod standard;

pub use standard::{decode_standard, encode_standard, standard_prefix};
```

Edit `crates/core/src/lib.rs` to add:

```rust
mod encoding;

pub use encoding::{decode_standard, encode_standard, standard_prefix};
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p osm-core encoding::standard`
Expected: PASS (4 tests).

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/encoding/ crates/core/src/lib.rs
git commit -m "feat(core): standard base64url text encoding"
```

---

### Task 6: Vendor BIP-39 Japanese wordlist + base-2048 word encoding

**Files:**
- Create: `crates/core/data/bip39-japanese.txt` (vendored, 2048 lines)
- Create: `crates/core/src/encoding/words.rs`
- Modify: `crates/core/src/encoding/mod.rs`, `crates/core/src/lib.rs`
- Test: inline in `crates/core/src/encoding/words.rs`

**Interfaces:**
- Consumes: `Payload`, `FormatError`.
- Produces:
  - `osm_core::encode_words(&Payload) -> String`
  - `osm_core::decode_words(&str) -> Result<Payload, FormatError>`
  - `osm_core::WORDLIST: [&str; 2048]`

**Word encoding scheme (also documented in SPEC.md, Task 12):**
1. Build a bit source = `BE32(payload_len)` followed by the payload bytes, MSB-first.
2. Split the bit stream into 11-bit groups, MSB-first; zero-pad the final group's low bits to fill 11 bits.
3. Map each 11-bit group (0–2047) to `WORDLIST[index]`, join with `、`.
4. Decode reverses this: words → indices → 11-bit groups → bit stream → read `BE32` length `L`, then `L` payload bytes. Remaining bits are padding.

- [ ] **Step 1: Vendor the wordlist**

Download the canonical BIP-39 Japanese wordlist (2048 newline-separated hiragana words) and save it to `crates/core/data/bip39-japanese.txt`:

Run:
```bash
mkdir -p crates/core/data
curl -fsSL https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/japanese.txt \
  -o crates/core/data/bip39-japanese.txt
wc -l crates/core/data/bip39-japanese.txt
```
Expected: `2048 crates/core/data/bip39-japanese.txt` (file ends with a trailing newline → `wc -l` reports 2048).

- [ ] **Step 2: Write the failing test**

Create `crates/core/src/encoding/words.rs`:

```rust
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::FormatError;
use crate::payload::Payload;

/// BIP-39 Japanese wordlist (2048 hiragana words), vendored at build time.
pub static WORDLIST: [&str; 2048] = {
    // include_str! splits at compile time via a helper build constant below.
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/wordlist_array.in"))
};

fn word_index() -> &'static HashMap<&'static str, u16> {
    static MAP: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    MAP.get_or_init(|| {
        WORDLIST
            .iter()
            .enumerate()
            .map(|(i, w)| (*w, i as u16))
            .collect()
    })
}

pub fn encode_words(payload: &Payload) -> String {
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
            WORDLIST[idx as usize]
        })
        .collect();
    words.join("\u{3001}") // "、"
}

pub fn decode_words(s: &str) -> Result<Payload, FormatError> {
    let map = word_index();
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
}
```

- [ ] **Step 3: Generate the wordlist array include file**

The `WORDLIST` array needs a compile-time array literal. Generate `crates/core/data/wordlist_array.in` from the vendored text file:

Run:
```bash
awk 'BEGIN{printf "["} {printf "\"%s\",", $0} END{print "]"}' \
  crates/core/data/bip39-japanese.txt > crates/core/data/wordlist_array.in
```
Expected: a single-line file `["あいこくしん","あいさつ",...,"われる",]` containing exactly 2048 quoted words.

- [ ] **Step 4: Wire into module and lib.rs**

Edit `crates/core/src/encoding/mod.rs` to add:

```rust
mod words;

pub use words::{decode_words, encode_words, WORDLIST};
```

Edit `crates/core/src/lib.rs` `pub use encoding::{...}` line to also re-export:

```rust
pub use encoding::{decode_standard, decode_words, encode_standard, encode_words, standard_prefix, WORDLIST};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p osm-core encoding::words`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/core/data/ crates/core/src/encoding/
git commit -m "feat(core): BIP-39 Japanese base-2048 word encoding"
```

---

### Task 7: Auto-detection (`detect_and_decode`)

**Files:**
- Create: `crates/core/src/encoding/detect.rs`
- Modify: `crates/core/src/encoding/mod.rs`, `crates/core/src/lib.rs`
- Test: inline in `crates/core/src/encoding/detect.rs`

**Interfaces:**
- Consumes: `decode_standard`, `decode_words`, `Payload`, `FormatError`.
- Produces: `osm_core::detect_and_decode(&str) -> Result<Payload, FormatError>`

- [ ] **Step 1: Write the failing test**

Create `crates/core/src/encoding/detect.rs`:

```rust
use crate::error::FormatError;
use crate::payload::Payload;

use super::standard::decode_standard;
use super::words::decode_words;

/// Decode any supported text representation. Routing:
/// - starts with "OSM" + digit + "." => standard form
/// - otherwise => Japanese wordlist form
pub fn detect_and_decode(s: &str) -> Result<Payload, FormatError> {
    let t = s.trim();
    if is_standard(t) {
        decode_standard(t)
    } else {
        decode_words(t)
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
    use crate::encoding::{encode_standard, encode_words};
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
    fn garbage_is_an_error() {
        assert!(detect_and_decode("this is not a ciphertext at all").is_err());
    }
}
```

- [ ] **Step 2: Wire into module and lib.rs**

Edit `crates/core/src/encoding/mod.rs` to add:

```rust
mod detect;

pub use detect::detect_and_decode;
```

Add `detect_and_decode` to the `pub use encoding::{...}` re-export in `crates/core/src/lib.rs`.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test -p osm-core encoding::detect`
Expected: PASS (3 tests).

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/encoding/ crates/core/src/lib.rs
git commit -m "feat(core): auto-detect and decode any text representation"
```

---

### Task 8: Canonical test vectors (`spec/test-vector.json`)

**Files:**
- Create: `crates/core/src/vectors.rs`
- Create: `spec/test-vector.json` (generated)
- Create: `crates/core/examples/gen_vectors.rs`
- Modify: `crates/core/src/lib.rs`
- Test: inline in `crates/core/src/vectors.rs`

**Interfaces:**
- Consumes: `encrypt`, `decrypt`, `encode_standard`, `encode_words`, `FixedRng`, `Argon2Params`.
- Produces: `osm_core::vectors::TestVector` (serde struct) and `osm_core::vectors::load_vectors(&str) -> Vec<TestVector>`.

- [ ] **Step 1: Write the TestVector struct and the conformance test (failing)**

Create `crates/core/src/vectors.rs`:

```rust
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
```

- [ ] **Step 2: Write the generator example**

Create `crates/core/examples/gen_vectors.rs`:

```rust
use osm_core::vectors::{TestVector, VectorArgon2};
use osm_core::{encode_standard, encode_words, encrypt, Argon2Params, FixedRng};

struct Case {
    name: &'static str,
    passphrase: &'static str,
    plaintext: &'static str,
    salt: [u8; 16],
    nonce: [u8; 12],
    params: Argon2Params,
}

fn main() {
    let cases = [
        Case {
            name: "ascii-basic",
            passphrase: "紙袋、みかん、夜道、ラジオ",
            plaintext: "secret note",
            salt: [0x11; 16],
            nonce: [0x22; 12],
            params: Argon2Params { m_cost: 8 * 1024, t_cost: 1, p_cost: 1 },
        },
        Case {
            name: "japanese-memo",
            passphrase: "あいことば",
            plaintext: "東京駅 18:30 集合 予約番号 AB-1234",
            salt: [0xAB; 16],
            nonce: [0xCD; 12],
            params: Argon2Params { m_cost: 8 * 1024, t_cost: 2, p_cost: 1 },
        },
        Case {
            name: "empty-plaintext",
            passphrase: "x",
            plaintext: "",
            salt: [0x01; 16],
            nonce: [0x02; 12],
            params: Argon2Params { m_cost: 8 * 1024, t_cost: 1, p_cost: 1 },
        },
    ];

    let vectors: Vec<TestVector> = cases
        .iter()
        .map(|c| {
            let mut seed = c.salt.to_vec();
            seed.extend_from_slice(&c.nonce);
            let mut rng = FixedRng::new(seed);
            let payload = encrypt(c.plaintext.as_bytes(), c.passphrase, c.params, &mut rng);
            TestVector {
                name: c.name.to_string(),
                passphrase: c.passphrase.to_string(),
                plaintext_utf8: c.plaintext.to_string(),
                argon2: VectorArgon2 {
                    m_cost: c.params.m_cost,
                    t_cost: c.params.t_cost,
                    p_cost: c.params.p_cost,
                },
                salt_hex: hex::encode(c.salt),
                nonce_hex: hex::encode(c.nonce),
                payload_hex: hex::encode(payload.to_bytes()),
                standard: encode_standard(&payload),
                words: encode_words(&payload),
            }
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&vectors).unwrap());
}
```

Add `hex` to `[dependencies]` (not just dev) is unnecessary; instead add it for the example by moving `hex` to a normal dependency OR add an example-scoped dep. Simplest: keep `hex` in `[dev-dependencies]` and mark the example as test-only is not supported, so add to `crates/core/Cargo.toml`:

```toml
[[example]]
name = "gen_vectors"
```

and move `hex = "0.4"` from `[dev-dependencies]` to `[dependencies]`. Also expose vectors module: add to `crates/core/src/lib.rs`:

```rust
pub mod vectors;
```

- [ ] **Step 3: Generate the test vectors**

Run:
```bash
cargo run -p osm-core --example gen_vectors > spec/test-vector.json
```
Expected: `spec/test-vector.json` containing a JSON array of 3 objects, each with non-empty `payload_hex`, `standard` (starting `OSM1.`), and `words`.

- [ ] **Step 4: Run the conformance test to verify it passes**

Run: `cargo test -p osm-core vectors::`
Expected: PASS (1 test iterating all 3 vectors). This proves generation and verification agree.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/vectors.rs crates/core/examples/gen_vectors.rs crates/core/Cargo.toml crates/core/src/lib.rs spec/test-vector.json
git commit -m "feat(core): canonical test vectors with conformance test"
```

---

### Task 9: Property tests (proptest)

**Files:**
- Create: `crates/core/tests/properties.rs`
- Test: this file IS the test.

**Interfaces:**
- Consumes: public API (`encrypt`, `decrypt`, `encode_standard`, `decode_standard`, `encode_words`, `decode_words`, `Argon2Params`, `FixedRng`).

- [ ] **Step 1: Write the property tests**

Create `crates/core/tests/properties.rs`:

```rust
use osm_core::{
    decode_standard, decode_words, decrypt, encode_standard, encode_words, encrypt, Argon2Params,
    FixedRng,
};
use proptest::prelude::*;

fn fast() -> Argon2Params {
    Argon2Params { m_cost: 8 * 1024, t_cost: 1, p_cost: 1 }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn encrypt_decrypt_roundtrip(plaintext: Vec<u8>, passphrase in ".{1,64}") {
        let mut rng = FixedRng::new(vec![1, 2, 3, 4, 5, 6, 7]);
        let payload = encrypt(&plaintext, &passphrase, fast(), &mut rng);
        prop_assert_eq!(decrypt(&payload, &passphrase).unwrap(), plaintext);
    }

    #[test]
    fn standard_encode_decode_roundtrip(plaintext: Vec<u8>) {
        let mut rng = FixedRng::new(vec![9, 8, 7, 6, 5]);
        let payload = encrypt(&plaintext, "pw", fast(), &mut rng);
        let s = encode_standard(&payload);
        prop_assert_eq!(decode_standard(&s).unwrap(), payload);
    }

    #[test]
    fn words_encode_decode_roundtrip(plaintext: Vec<u8>) {
        let mut rng = FixedRng::new(vec![3, 1, 4, 1, 5, 9, 2]);
        let payload = encrypt(&plaintext, "pw", fast(), &mut rng);
        let w = encode_words(&payload);
        prop_assert_eq!(decode_words(&w).unwrap(), payload);
    }
}
```

- [ ] **Step 2: Run the property tests to verify they pass**

Run: `cargo test -p osm-core --test properties`
Expected: PASS (3 properties, 64 cases each).

- [ ] **Step 3: Commit**

```bash
git add crates/core/tests/properties.rs
git commit -m "test(core): property tests for crypto and encoding roundtrips"
```

---

### Task 10: Fuzz targets (cargo-fuzz)

**Files:**
- Create: `crates/core/fuzz/Cargo.toml`
- Create: `crates/core/fuzz/fuzz_targets/decode_standard.rs`
- Create: `crates/core/fuzz/fuzz_targets/decode_words.rs`
- Create: `crates/core/fuzz/fuzz_targets/detect_and_decode.rs`

**Interfaces:**
- Consumes: public API decoders. Goal: no input string causes a panic — decoders must always return `Ok`/`Err`.

- [ ] **Step 1: Create the fuzz crate manifest**

Create `crates/core/fuzz/Cargo.toml`:

```toml
[package]
name = "osm-core-fuzz"
version = "0.0.0"
edition = "2021"
publish = false

[package.metadata]
cargo-fuzz = true

[dependencies]
libfuzzer-sys = "0.4"
osm-core = { path = ".." }

[[bin]]
name = "decode_standard"
path = "fuzz_targets/decode_standard.rs"
test = false
doc = false

[[bin]]
name = "decode_words"
path = "fuzz_targets/decode_words.rs"
test = false
doc = false

[[bin]]
name = "detect_and_decode"
path = "fuzz_targets/detect_and_decode.rs"
test = false
doc = false
```

- [ ] **Step 2: Create the three fuzz targets**

Create `crates/core/fuzz/fuzz_targets/decode_standard.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = osm_core::decode_standard(s);
    }
});
```

Create `crates/core/fuzz/fuzz_targets/decode_words.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = osm_core::decode_words(s);
    }
});
```

Create `crates/core/fuzz/fuzz_targets/detect_and_decode.rs`:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = osm_core::detect_and_decode(s);
    }
});
```

- [ ] **Step 3: Smoke-run each target briefly**

Requires the nightly toolchain and `cargo install cargo-fuzz`. Run each target for a bounded time:

Run:
```bash
cargo +nightly fuzz run decode_standard -- -max_total_time=30
cargo +nightly fuzz run decode_words -- -max_total_time=30
cargo +nightly fuzz run detect_and_decode -- -max_total_time=30
```
Expected: each exits cleanly with no crash artifact written to `crates/core/fuzz/artifacts/`.

- [ ] **Step 4: Commit**

```bash
git add crates/core/fuzz/Cargo.toml crates/core/fuzz/fuzz_targets/
git commit -m "test(core): cargo-fuzz targets for decoder panic-resistance"
```

---

### Task 11: CLI (`encrypt` / `decrypt` / `verify`)

**Files:**
- Modify: `crates/cli/Cargo.toml`
- Modify: `crates/cli/src/main.rs`
- Test: `crates/cli/tests/cli.rs`

**Interfaces:**
- Consumes: full `osm_core` public API.
- Produces: binary `osm` with subcommands `encrypt`, `decrypt`, `verify`.

- [ ] **Step 1: Update CLI dependencies**

Replace `crates/cli/Cargo.toml` with:

```toml
[package]
name = "osm-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "osm"
path = "src/main.rs"

[dependencies]
osm-core = { path = "../core" }
clap = { version = "4", features = ["derive"] }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

- [ ] **Step 2: Write the failing CLI test**

Create `crates/cli/tests/cli.rs`:

```rust
use assert_cmd::Command;

#[test]
fn encrypt_then_decrypt_roundtrip() {
    // Encrypt reads the memo from stdin, passphrase from --passphrase.
    let assert = Command::cargo_bin("osm")
        .unwrap()
        .args(["encrypt", "--passphrase", "test-pass", "--m-cost", "8192"])
        .write_stdin("my secret")
        .assert()
        .success();
    let ciphertext = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let ciphertext = ciphertext.trim().to_string();
    assert!(ciphertext.starts_with("OSM1."));

    Command::cargo_bin("osm")
        .unwrap()
        .args(["decrypt", "--passphrase", "test-pass"])
        .write_stdin(ciphertext)
        .assert()
        .success()
        .stdout("my secret");
}

#[test]
fn decrypt_wrong_passphrase_fails() {
    let assert = Command::cargo_bin("osm")
        .unwrap()
        .args(["encrypt", "--passphrase", "right", "--m-cost", "8192"])
        .write_stdin("data")
        .assert()
        .success();
    let ciphertext = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    Command::cargo_bin("osm")
        .unwrap()
        .args(["decrypt", "--passphrase", "wrong"])
        .write_stdin(ciphertext.trim().to_string())
        .assert()
        .failure();
}

#[test]
fn verify_passes_on_bundled_vectors() {
    Command::cargo_bin("osm")
        .unwrap()
        .args(["verify", "--vectors", "../../spec/test-vector.json"])
        .assert()
        .success();
}
```

- [ ] **Step 3: Implement the CLI**

Replace `crates/cli/src/main.rs` with:

```rust
use std::io::{Read, Write};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use osm_core::vectors::load_vectors;
use osm_core::{
    decrypt, detect_and_decode, encode_standard, encode_words, encrypt, Argon2Params, FixedRng,
    OsRng,
};

#[derive(Parser)]
#[command(name = "osm", about = "Open Secret Memo CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Encrypt a memo read from stdin; prints the ciphertext to stdout.
    Encrypt {
        #[arg(long)]
        passphrase: String,
        #[arg(long, default_value_t = 65536)]
        m_cost: u32,
        #[arg(long, default_value_t = 1)]
        t_cost: u32,
        #[arg(long, default_value_t = 1)]
        p_cost: u8,
        /// Output the Japanese wordlist form instead of standard.
        #[arg(long)]
        words: bool,
    },
    /// Decrypt a ciphertext read from stdin; prints the plaintext to stdout.
    Decrypt {
        #[arg(long)]
        passphrase: String,
    },
    /// Verify that this build reproduces a test-vector.json file.
    Verify {
        #[arg(long)]
        vectors: String,
    },
}

fn read_stdin() -> Vec<u8> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf).expect("read stdin");
    buf
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Encrypt { passphrase, m_cost, t_cost, p_cost, words } => {
            let params = Argon2Params { m_cost, t_cost, p_cost };
            if let Err(e) = params.validate() {
                eprintln!("invalid parameters: {e}");
                return ExitCode::FAILURE;
            }
            let plaintext = read_stdin();
            let mut rng = OsRng;
            let payload = encrypt(&plaintext, &passphrase, params, &mut rng);
            let out = if words { encode_words(&payload) } else { encode_standard(&payload) };
            println!("{out}");
            ExitCode::SUCCESS
        }
        Command::Decrypt { passphrase } => {
            let input = String::from_utf8(read_stdin()).unwrap_or_default();
            let payload = match detect_and_decode(&input) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
            match decrypt(&payload, &passphrase) {
                Ok(plaintext) => {
                    std::io::stdout().write_all(&plaintext).unwrap();
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Verify { vectors } => {
            let json = std::fs::read_to_string(&vectors).expect("read vectors file");
            for v in load_vectors(&json) {
                let salt = hex_decode(&v.salt_hex);
                let nonce = hex_decode(&v.nonce_hex);
                let mut seed = salt;
                seed.extend_from_slice(&nonce);
                let mut rng = FixedRng::new(seed);
                let payload =
                    encrypt(v.plaintext_utf8.as_bytes(), &v.passphrase, v.argon2.to_params(), &mut rng);
                if encode_standard(&payload) != v.standard {
                    eprintln!("MISMATCH in vector {}", v.name);
                    return ExitCode::FAILURE;
                }
            }
            println!("all vectors verified");
            ExitCode::SUCCESS
        }
    }
}

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex"))
        .collect()
}
```

Add `pub use params::ParamError;` is already exported. Ensure `Argon2Params::validate` is reachable (it is, via `Argon2Params`).

- [ ] **Step 4: Run the CLI tests to verify they pass**

Run: `cargo test -p osm-cli`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/cli/
git commit -m "feat(cli): encrypt/decrypt/verify subcommands over osm-core"
```

---

### Task 12: SPEC.md (byte-level specification)

**Files:**
- Create: `spec/SPEC.md`

**Interfaces:** None (documentation). Must describe every field precisely enough for an independent re-implementation to pass `test-vector.json`.

- [ ] **Step 1: Write SPEC.md**

Create `spec/SPEC.md` with the following content (fill the constants from the implemented code — they are all fixed above):

```markdown
# Open Secret Memo — Format Specification (v1)

This document defines the Open Secret Memo ciphertext format precisely enough to
re-implement decryption in any language. Conformance is verified by
`test-vector.json`.

## 1. Cryptographic primitives

- Key derivation: **Argon2id** (RFC 9106), version 0x13, output 32 bytes.
- Encryption: **AES-256-GCM**, 12-byte nonce, 16-byte tag.

## 2. Passphrase normalization (passphrase ONLY)

1. Treat as a Unicode string.
2. Normalize with **NFKC**.
3. Trim leading/trailing whitespace.
4. Encode as UTF-8.
5. Use as the Argon2id password input.

The memo body is NEVER normalized; its bytes are encrypted verbatim.

## 3. Binary payload layout

All integers big-endian.

| Offset | Size | Field |
|--------|------|-------|
| 0  | 3  | Magic `OSM` (0x4F 0x53 0x4D) |
| 3  | 1  | Version (= 1) |
| 4  | 4  | Argon2 m_cost (KiB) |
| 8  | 4  | Argon2 t_cost (iterations) |
| 12 | 1  | Argon2 p_cost (parallelism) |
| 13 | 16 | salt |
| 29 | 12 | nonce |
| 41 | .. | AES-256-GCM ciphertext, with the 16-byte tag appended |

The **first 41 bytes (offsets 0–40)** are passed to AES-256-GCM as the
**Additional Authenticated Data (AAD)**.

## 4. Encryption procedure

1. Generate random 16-byte salt and 12-byte nonce.
2. Assemble the 41-byte header (AAD) from version, params, salt, nonce.
3. key = Argon2id(normalized_passphrase, salt, m_cost, t_cost, p_cost), 32 bytes.
4. ciphertext||tag = AES-256-GCM-Encrypt(key, nonce, plaintext, aad = header).
5. payload = header || ciphertext||tag.

Decryption reverses this; an authentication-tag mismatch means wrong passphrase
or corruption.

## 5. Text representations

### 5.1 Standard form
`"OSM" + <version digit> + "." + base64url_nopad(payload)`. Example prefix: `OSM1.`.

### 5.2 Japanese wordlist form
Uses the BIP-39 Japanese wordlist (2048 hiragana words) as a base-2048 alphabet.

1. bitstream = BE32(payload length in bytes) || payload bytes, MSB-first.
2. Split into 11-bit groups (MSB-first); zero-pad the final group's low bits.
3. Each group (0–2047) selects `wordlist[index]`; join words with `、` (U+3001).

Decoding: words → indices → 11-bit groups → bitstream → read BE32 length L →
read L payload bytes. Trailing bits are zero padding.

> Note: this uses the BIP-39 wordlist purely as an encoding alphabet. It is NOT a
> wallet seed phrase and carries no BIP-39 checksum in v1.

## 6. Detection

- Input matching `^OSM[0-9]\.` is the standard form.
- Otherwise it is parsed as the wordlist form.

## 7. Test vectors

See `test-vector.json`. Each entry fixes salt/nonce so that encryption is
deterministic and reproducible. A conforming implementation MUST reproduce
`payload_hex`, `standard`, and `words` for every entry.
```

- [ ] **Step 2: Cross-check the spec against vectors**

Run: `cargo test -p osm-core vectors::`
Expected: PASS — confirms the spec's described procedure matches the generated vectors.

- [ ] **Step 3: Commit**

```bash
git add spec/SPEC.md
git commit -m "docs(spec): byte-level format specification v1"
```

---

## Self-Review

**Spec coverage check (design doc → tasks):**
- Argon2id + AES-256-GCM, key derivation → Task 4 ✓
- 41-byte binary layout + AAD → Task 3, 4 ✓
- NFKC/trim passphrase normalization → Task 4 ✓
- Argon2 defaults + advanced param validation → Task 2, CLI flags Task 11 ✓
- Standard form `OSM1.` → Task 5 ✓
- Japanese wordlist base-2048 + length-prefix clarification → Task 6 ✓
- Auto-detection → Task 7 ✓
- DecryptError taxonomy (Malformed / UnsupportedVersion / InvalidWord / AuthenticationFailed) → Task 1, surfaced through Tasks 3/6/4 ✓
- test-vector.json (canonical) → Task 8 ✓
- Tests: unit (Tasks 1–8), property (Task 9), fuzz (Task 10), vector conformance (Task 8) → ✓
- CLI encrypt/decrypt/verify → Task 11 ✓
- SPEC.md → Task 12 ✓
- Deferred to Plan 2/3 (web/PWA, wasm, recovery kit, IPFS, deploy, UI strength warning) — out of this plan's scope by design ✓

**Note on validation surface:** the "weak passphrase = warning only" and "empty memo/mismatch disables button" rules are UI behaviors implemented in Plan 2 (web). `core` deliberately does not block weak passphrases — it encrypts whatever it is given. Argon2 *parameter* range validation lives in `core` (Task 2) and is enforced by the CLI (Task 11) and will be enforced by the web layer.

**Placeholder scan:** No TBD/TODO; every code step contains complete code. ✓

**Type consistency:** `Argon2Params`, `Payload`, `FixedRng`, `OsRng`, `encrypt/decrypt`, `encode_*/decode_*`, `detect_and_decode`, `TestVector`/`VectorArgon2` names are consistent across Tasks 2–12. ✓
