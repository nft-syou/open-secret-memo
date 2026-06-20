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

### 5.3 Kanji-mixed form (experimental)
An index-aligned kanji "skin" of the §5.2 wordlist. The encoding is identical to
§5.2; only the alphabet differs — `kanji_wordlist[index]` replaces `wordlist[index]`.
The kanji wordlist shares indices with the hiragana wordlist: each entry is either
the standard 常用漢字 spelling of the same word, or (when no safe, unambiguous kanji
spelling exists) the original BIP-39 hiragana. The output is therefore 漢字混じり.

Each index uses the most natural standard written form of the same BIP-39 word,
chosen in this priority order:

1. **Kanji** — a JMdict kanji writing whose kanji characters are all 常用漢字 and which
   is NFC-stable (kana such as okurigana is allowed, e.g. `赤ちゃん`). A single
   candidate is taken as-is (`愛国心`); multiple candidates are disambiguated by JMdict
   frequency markers, preferring the most common (`感謝` over `官舎`).
2. **Katakana** — inherently foreign / loanwords (a JMdict entry whose readings are all
   katakana): `あめりか`→`アメリカ`, `たいみんぐ`→`タイミング`.
3. **Kanji (proper noun)** — place/person names via JMnedict with a single 常用漢字
   writing: `かなざわし`→`金沢市`.
4. **Hiragana** — otherwise the original BIP-39 hiragana is kept (native words written
   in kana, obscure-ateji-only readings, and collision avoidance).

The frozen list lives at `crates/core/data/bip39-japanese-kanji.txt`.

Normalization: the BIP-39 hiragana base is NFKD. List generation matches via NFC;
kana-fallback entries keep BIP-39's NFKD bytes (so indices stay parallel to §5.2);
kanji entries are NFC-stable atomic ideographs (stable under every normalization form).

> Status: EXPERIMENTAL. The standard form (§5.1) is recommended for long-term
> storage. The kanji list is not yet frozen; a reproducibility test vector will be
> added once it has been audited and frozen.

## 6. Detection

- Input matching `^OSM[0-9]\.` is the standard form.
- Otherwise it is parsed as the hiragana wordlist form (§5.2), falling back to the
  experimental kanji form (§5.3) when a token is absent from the hiragana wordlist.
  Because the two wordlists share indices, an all-kana input decodes identically
  under either table.

## 7. Test vectors

See `test-vector.json`. Each entry fixes salt/nonce so that encryption is
deterministic and reproducible. A conforming implementation MUST reproduce
`payload_hex`, `standard`, and `words` for every entry.
