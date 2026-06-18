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
