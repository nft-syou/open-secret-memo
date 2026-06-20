# bip-39-japanese-for-kanji（漢字混じりメモ風形式）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** BIP-39 日本語(ひらがな2048語)を index 一致で漢字化した「漢字スキン」ワードリストを生成・凍結し、それを使う実験的な保存形式 `"kanji"`（漢字混じりメモ風）を core/wasm/CLI/web に追加する。

**Architecture:** 既存の 11bit base-2048 ワード符号化（`、`区切り・4バイトBE長さプレフィックス）を**完全流用**し、`WORDLIST` と並んで `KANJI_WORDLIST` を追加。符号化/復号関数を「表」でパラメータ化し、ひらがな版/漢字版の薄いラッパを提供。`detect_and_decode` に漢字版フォールバックを追加。漢字リストは**ビルド時の外部辞書(JMdict/KANJIDIC2, EDRDG)を使うワンショット生成スクリプト**で候補を作り、**人手監査の上で txt に凍結**（cargo/web ビルドはネットワーク不使用、凍結 txt のみ使用）。

**Tech Stack:** Rust (osm-core/osm-wasm/osm-cli), wasm-bindgen/wasm-pack, Vite+React+TS, Python 3（生成スクリプトのみ・標準ライブラリだけ／pip 依存なし）。

## Global Constraints

- **暗号・フォーマット・成功パス不変**：バイナリ payload・Argon2id・AES-GCM・標準形式(`OSM1.`+base64url)・既存ひらがなワード形式は一切変更しない。本作業は新しい外側テキスト符号化の追加のみ。
- **既存の安定 `error_kind` 不変**：`malformed`/`unsupported_version`/`invalid_word`/`auth_failed`/`not_utf8`。漢字語の不正は既存 `invalid_word` を流用（新種別を増やさない）。
- **プライバシー不変条件維持**：平文/合言葉/復号結果はログ・保存・送信しない。生成スクリプトは秘密を一切扱わない（公開辞書のみ）。
- **index 完全一致**：`bip39-japanese-kanji.txt` の行 i は `bip39-japanese.txt` の語 i の漢字スキン。2048 行・改行区切り・末尾改行あり（`wc -l` で 2048）。
- **漢字採用条件（全て満たす語だけ漢字化、他はかな維持）**：使用文字が全て常用漢字（KANJIDIC2 grade 1–8）／送り仮名なし(=keb が純漢字)／同音異義で候補が割れない(有効候補がちょうど1つ)／NFKC 安定。
- **凍結**：リリース後の `bip39-japanese-kanji.txt` は永久不変（1文字でも変えると過去テキストが復号不能）。修正は別ファイル/別種別で。
- **実験扱い**：UI/SPEC に実験ラベル＋「長期保存は標準形式推奨」を明記（#2 方針／spec Q-b）。
- **ツール**：Rust は `. "$HOME/.cargo/env"`。wasm テストはローカルにブラウザ無し→ `wasm-pack test --node`。web は pnpm 11 / Node 22。テストの Argon2 は `m_cost=8192`。
- 参照 spec：`docs/superpowers/specs/2026-06-20-bip39-japanese-kanji-wordlist-design.md`。

---

## File Structure

- `scripts/gen-kanji-wordlist.py` — 生成スクリプト（JMdict/KANJIDIC2 取得→候補生成→フィルタ→候補 txt＋カバレッジレポート）。ビルド時 AID、出力が正。
- `crates/core/data/bip39-japanese-kanji.txt` — 凍結データ（2048行）。**人手監査後に確定**。
- `crates/core/data/wordlist_kanji_array.in` — 上記から生成する配列リテラル include ファイル。
- `crates/core/src/encoding/words.rs` — `KANJI_WORDLIST` 追加＋符号化/復号を表でパラメータ化＋漢字版ラッパ＋テスト。
- `crates/core/src/encoding/mod.rs`, `crates/core/src/lib.rs` — 再エクスポート追加。
- `crates/core/src/encoding/detect.rs` — 漢字版フォールバック＋テスト。
- `crates/wasm/src/lib.rs`, `crates/wasm/tests/web.rs` — encrypt の `"kanji"` 受理＋テスト。
- `crates/cli/src/main.rs`, `crates/cli/tests/cli.rs` — `--kanji` フラグ＋テスト。
- `web/src/crypto/client.ts`, `web/src/crypto/worker.ts`, `web/src/components/EncryptTab.tsx`, `web/src/components/EncryptTab.test.tsx` — `"kanji"` 形式 UI（実験ラベル）＋テスト。
- `spec/SPEC.md`, `spec/test-vector.json`（または併設）, `docs/要件定義.md`, `docs/superpowers/specs/2026-06-17-open-secret-memo-design.md` — 仕様・ベクタ・ステータス同期。

---

### Task 1: 漢字ワードリスト生成スクリプト＋初回生成（カバレッジ確認ゲート）

**Files:**
- Create: `scripts/gen-kanji-wordlist.py`
- Produce (committed after audit): `crates/core/data/bip39-japanese-kanji.txt`
- Produce (report, git-ignored or committed): `scripts/kanji-wordlist-report.tsv`

**Interfaces:**
- Consumes: `crates/core/data/bip39-japanese.txt`（2048 ひらがな語）, JMdict_e.gz, kanjidic2.xml.gz（EDRDG）。
- Produces: `bip39-japanese-kanji.txt`（2048 行・index 一致）＋ `kanji-wordlist-report.tsv`（各語: index, 元かな, 判定種別, 採用値）。

- [ ] **Step 1: 生成スクリプトを書く**

`scripts/gen-kanji-wordlist.py`:

```python
#!/usr/bin/env python3
"""Generate the index-aligned kanji "skin" of the BIP-39 Japanese wordlist.

Build-time AID ONLY. Its OUTPUT (crates/core/data/bip39-japanese-kanji.txt) is the
frozen source of truth, committed to the repo. JMdict / KANJIDIC2 (EDRDG, CC-BY-SA)
are used here purely as a lookup aid to determine standard orthography; they are
NOT redistributed. The output is factual orthographic data (the standard kanji
spelling of common words), index-aligned with bitcoin/bips bip-0039/japanese.txt.
"""
import gzip, io, sys, unicodedata, urllib.request
import xml.etree.ElementTree as ET
from collections import Counter
from pathlib import Path

BIP39 = Path("crates/core/data/bip39-japanese.txt")
OUT = Path("crates/core/data/bip39-japanese-kanji.txt")
REPORT = Path("scripts/kanji-wordlist-report.tsv")
JMDICT_URL = "http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz"
KANJIDIC_URL = "http://ftp.edrdg.org/pub/Nihongo/kanjidic2.xml.gz"

def kata_to_hira(s: str) -> str:
    return "".join(chr(ord(c) - 0x60) if 0x30A1 <= ord(c) <= 0x30F6 else c for c in s)

def fetch_gz(url: str) -> bytes:
    print(f"fetching {url} ...", file=sys.stderr)
    with urllib.request.urlopen(url, timeout=120) as r:
        return gzip.decompress(r.read())

def load_jouyou() -> set:
    """KANJIDIC2: characters with misc/grade 1..8 are 常用漢字 (9,10 = 人名用)."""
    xml = fetch_gz(KANJIDIC_URL)
    jouyou = set()
    for _, el in ET.iterparse(io.BytesIO(xml), events=("end",)):
        if el.tag == "character":
            lit = el.findtext("literal")
            grade = el.findtext("misc/grade")
            if lit and grade and grade.isdigit() and 1 <= int(grade) <= 8:
                jouyou.add(lit)
            el.clear()
    return jouyou

def load_reading_index() -> dict:
    """JMdict: reading(kana, hira-normalized) -> set of kanji writings (keb)."""
    xml = fetch_gz(JMDICT_URL)
    idx = {}
    for _, el in ET.iterparse(io.BytesIO(xml), events=("end",)):
        if el.tag == "entry":
            kebs = [k.text for k in el.findall("k_ele/keb") if k.text]
            rebs = [kata_to_hira(r.text) for r in el.findall("r_ele/reb") if r.text]
            for r in rebs:
                idx.setdefault(r, set()).update(kebs)
            el.clear()
    return idx

def is_pure_jouyou(s: str, jouyou: set) -> bool:
    return bool(s) and all(ch in jouyou for ch in s)

def nfkc_stable(s: str) -> bool:
    return unicodedata.normalize("NFKC", s) == s

def main() -> int:
    words = [w for w in BIP39.read_text(encoding="utf-8").split("\n") if w]
    assert len(words) == 2048, f"expected 2048 BIP-39 words, got {len(words)}"
    jouyou = load_jouyou()
    idx = load_reading_index()
    print(f"jouyou kanji: {len(jouyou)}, reading keys: {len(idx)}", file=sys.stderr)

    result, reasons = [], []
    for w in words:
        valid = sorted({k for k in idx.get(w, set())
                        if is_pure_jouyou(k, jouyou) and nfkc_stable(k)})
        if len(valid) == 1:
            result.append(valid[0]); reasons.append("kanji")
        elif not valid:
            result.append(w); reasons.append("kana:no-candidate")
        else:
            result.append(w); reasons.append("kana:ambiguous(" + "/".join(valid) + ")")

    # Collision resolution: a kanji form chosen for >1 position -> revert all to kana.
    counts = Counter(result)
    for i, v in enumerate(result):
        if counts[v] > 1 and v != words[i]:
            result[i] = words[i]; reasons[i] = "kana:collision"

    assert len(result) == 2048
    assert len(set(result)) == 2048, "post-conversion list is not unique"
    assert all(nfkc_stable(r) for r in result)

    OUT.write_text("\n".join(result) + "\n", encoding="utf-8")
    REPORT.write_text(
        "\n".join(f"{i}\t{words[i]}\t{reasons[i]}\t{result[i]}" for i in range(2048)) + "\n",
        encoding="utf-8")
    n = sum(1 for r in reasons if r == "kanji")
    print(f"coverage: {n}/2048 kanji-ified ({100*n/2048:.1f}%)", file=sys.stderr)
    print(f"wrote {OUT} and {REPORT}", file=sys.stderr)
    return 0

if __name__ == "__main__":
    sys.exit(main())
```

- [ ] **Step 2: 実行してカバレッジを確認する（確認ゲート）**

Run（リポジトリ root から）:
```bash
python3 scripts/gen-kanji-wordlist.py
wc -l crates/core/data/bip39-japanese-kanji.txt
```
Expected: `coverage: N/2048 ...` がログに出て、`bip39-japanese-kanji.txt` が **2048 行**。

- [ ] **Step 3: 人手監査ゲート（必須・凍結前）**

`scripts/kanji-wordlist-report.tsv` を確認し、(1) 漢字化率、(2) `kana:ambiguous` で落ちた高頻度語、(3) 誤選択の有無、を点検。ここで **常用漢字のみで十分か／常用＋人名用へ緩めるか**を決定（緩める場合は `load_jouyou` の grade 上限を 10 に変更して再生成）。**監査で確定した値が以降の凍結対象**。

- [ ] **Step 4: コミット**

```bash
git add scripts/gen-kanji-wordlist.py crates/core/data/bip39-japanese-kanji.txt scripts/kanji-wordlist-report.tsv
git commit -m "feat(core): generate index-aligned kanji skin of BIP-39 JA wordlist (#2)"
```

---

### Task 2: core 符号化/復号に `KANJI_WORDLIST` を追加（表パラメータ化）

**Files:**
- Modify: `crates/core/src/encoding/words.rs`
- Create: `crates/core/data/wordlist_kanji_array.in`
- Modify: `crates/core/src/encoding/mod.rs`, `crates/core/src/lib.rs`
- Test: `crates/core/src/encoding/words.rs`(#[cfg(test)])

**Interfaces:**
- Consumes: `crates/core/data/bip39-japanese-kanji.txt`（Task 1）, 既存 `Payload`。
- Produces: `KANJI_WORDLIST: [&str; 2048]`, `pub fn encode_words_kanji(&Payload) -> String`, `pub fn decode_words_kanji(&str) -> Result<Payload, FormatError>`。

- [ ] **Step 1: 配列 include ファイルを生成**

Run:
```bash
awk 'BEGIN{printf "["} {printf "\"%s\",", $0} END{print "]"}' \
  crates/core/data/bip39-japanese-kanji.txt > crates/core/data/wordlist_kanji_array.in
```
Expected: 1 行 `["愛国心","挨拶",...,"...",]`（2048 個）。

- [ ] **Step 2: 失敗するテストを書く**

`crates/core/src/encoding/words.rs` の `mod tests` に追記:

```rust
    #[test]
    fn kanji_wordlist_is_2048_unique_and_index_aligned() {
        assert_eq!(KANJI_WORDLIST.len(), WORDLIST.len());
        let set: std::collections::HashSet<_> = KANJI_WORDLIST.iter().collect();
        assert_eq!(set.len(), 2048);
        // 少なくとも一部は実際に漢字化されている（ひらがな版と異なる位置がある）
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
```

- [ ] **Step 3: 失敗を確認**

Run: `. "$HOME/.cargo/env"; cargo test -p osm-core kanji 2>&1 | tail -15`
Expected: コンパイルエラー（`KANJI_WORDLIST`/`encode_words_kanji`/`decode_words_kanji` 未定義）。

- [ ] **Step 4: 表でパラメータ化し漢字版を実装**

`crates/core/src/encoding/words.rs` を編集。`KANJI_WORDLIST` 追加（`WORDLIST` の直後）:

```rust
/// Index-aligned kanji "skin" of the BIP-39 Japanese wordlist (frozen, 2048 entries).
/// Same indices as WORDLIST; entries that cannot be safely written in 常用漢字 stay hiragana.
pub static KANJI_WORDLIST: [&str; 2048] = {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/wordlist_kanji_array.in"))
};
```

`word_index()` の隣に漢字版インデックスを追加:

```rust
fn kanji_word_index() -> &'static HashMap<&'static str, u16> {
    static MAP: OnceLock<HashMap<&'static str, u16>> = OnceLock::new();
    MAP.get_or_init(|| {
        KANJI_WORDLIST.iter().enumerate().map(|(i, w)| (*w, i as u16)).collect()
    })
}
```

`encode_words` を表パラメータ化（既存ロジックは `encode_with` に移す）:

```rust
/// Encode a payload as a Japanese BIP-39 word sequence (base-2048), words joined by `、` (U+3001).
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
    words.join("\u{3001}")
}
```

`decode_words` を表パラメータ化（既存ロジックは `decode_with` に移す）:

```rust
/// Decode a Japanese BIP-39 word sequence back into a payload (hiragana wordlist).
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
```

- [ ] **Step 5: 再エクスポート**

`crates/core/src/encoding/mod.rs` の words 行を変更:
```rust
pub use words::{decode_words, decode_words_kanji, encode_words, encode_words_kanji, KANJI_WORDLIST, WORDLIST};
```
`crates/core/src/lib.rs:10` の encoding 再エクスポートに `decode_words_kanji, encode_words_kanji, KANJI_WORDLIST` を追加。

- [ ] **Step 6: テストを通す**

Run: `. "$HOME/.cargo/env"; cargo test -p osm-core 2>&1 | tail -15`
Expected: PASS（既存 + 新規 kanji テスト）。

- [ ] **Step 7: コミット**

```bash
git add crates/core/data/wordlist_kanji_array.in crates/core/src/encoding/words.rs crates/core/src/encoding/mod.rs crates/core/src/lib.rs
git commit -m "feat(core): KANJI_WORDLIST + encode/decode_words_kanji (#2)"
```

---

### Task 3: `detect_and_decode` に漢字版フォールバックを追加

**Files:**
- Modify: `crates/core/src/encoding/detect.rs`
- Test: `crates/core/src/encoding/detect.rs`(#[cfg(test)])

**Interfaces:**
- Consumes: `decode_words`, `decode_words_kanji`（Task 2）。
- Produces: 変更なし（`detect_and_decode` の挙動拡張）。

- [ ] **Step 1: 失敗するテストを書く**

`detect.rs` の `mod tests` に追記（`use crate::encoding::encode_words_kanji;` を `use` 行に追加）:

```rust
    #[test]
    fn routes_kanji() {
        let s = encode_words_kanji(&sample());
        assert_eq!(detect_and_decode(&s).unwrap(), sample());
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `. "$HOME/.cargo/env"; cargo test -p osm-core routes_kanji 2>&1 | tail -15`
Expected: FAIL（漢字トークンが `decode_words` で `InvalidWord` になり復号できない）。

- [ ] **Step 3: フォールバックを実装**

`detect.rs` を編集:
```rust
use super::words::{decode_words, decode_words_kanji};
```
```rust
pub fn detect_and_decode(s: &str) -> Result<Payload, FormatError> {
    let t = s.trim();
    if is_standard(t) {
        decode_standard(t)
    } else {
        // ひらがなワード→失敗時のみ漢字スキンを試す（index 一致なので純かな文は両者同一に復号）。
        decode_words(t).or_else(|_| decode_words_kanji(t))
    }
}
```

- [ ] **Step 4: テストを通す**

Run: `. "$HOME/.cargo/env"; cargo test -p osm-core 2>&1 | tail -15`
Expected: PASS（`routes_standard`/`routes_words`/`routes_kanji`/`garbage_is_an_error` 含む）。

- [ ] **Step 5: コミット**

```bash
git add crates/core/src/encoding/detect.rs
git commit -m "feat(core): detect_and_decode falls back to kanji wordlist (#2)"
```

---

### Task 4: wasm `encrypt` が `"kanji"` 形式を受理

**Files:**
- Modify: `crates/wasm/src/lib.rs`
- Test: `crates/wasm/tests/web.rs`

**Interfaces:**
- Consumes: core `encode_words_kanji`（Task 2）, `detect_and_decode`（Task 3, decrypt が使用）。
- Produces: wasm `encrypt(.., format="kanji")` が漢字スキン文字列を返す。decrypt は detect 経由で自動対応（変更不要）。

- [ ] **Step 1: 失敗するテストを書く**

`crates/wasm/tests/web.rs` に追記（先頭の `use osm_wasm::{decrypt, encrypt};` はそのまま）:

```rust
#[wasm_bindgen_test]
fn roundtrip_kanji() {
    let ct = encrypt("秘密のメモ", "pw", 8192, 1, 1, "kanji").unwrap();
    let outcome = decrypt(&ct, "pw");
    assert!(outcome.ok);
    assert_eq!(outcome.text, "秘密のメモ");
}
```

- [ ] **Step 2: 失敗を確認**

Run: `. "$HOME/.cargo/env"; wasm-pack test --node crates/wasm 2>&1 | tail -15`
Expected: FAIL（`"kanji"` が現状 `Err("unknown format: kanji")`）。

- [ ] **Step 3: 実装**

`crates/wasm/src/lib.rs` の use に `encode_words_kanji` を追加し、encrypt の match に `"kanji"` 分岐を追加:
```rust
    match format {
        "standard" => Ok(encode_standard(&payload)),
        "words" => Ok(encode_words(&payload)),
        "kanji" => Ok(encode_words_kanji(&payload)),
        other => Err(JsError::new(&format!("unknown format: {other}"))),
    }
```

- [ ] **Step 4: テストを通す**

Run: `. "$HOME/.cargo/env"; wasm-pack test --node crates/wasm 2>&1 | tail -15`
Expected: PASS（既存 6 + `roundtrip_kanji` = 7）。

- [ ] **Step 5: コミット**

```bash
git add crates/wasm/src/lib.rs crates/wasm/tests/web.rs
git commit -m "feat(wasm): accept kanji save format (#2)"
```

---

### Task 5: CLI `--kanji` フラグ

**Files:**
- Modify: `crates/cli/src/main.rs`
- Test: `crates/cli/tests/cli.rs`

**Interfaces:**
- Consumes: core `encode_words_kanji`。
- Produces: `osm encrypt --kanji` が漢字スキン形式で出力。`decrypt` は既存（detect 経由）で対応。

- [ ] **Step 1: 失敗するテストを書く**

`crates/cli/tests/cli.rs` に追記:

```rust
#[test]
fn encrypt_kanji_then_decrypt_roundtrip() {
    let assert = Command::cargo_bin("osm").unwrap()
        .args(["encrypt", "--passphrase", "k", "--m-cost", "8192", "--kanji"])
        .write_stdin("漢字メモ")
        .assert().success();
    let ct = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    Command::cargo_bin("osm").unwrap()
        .args(["decrypt", "--passphrase", "k"])
        .write_stdin(ct.trim().to_string())
        .assert().success().stdout("漢字メモ");
}
```

- [ ] **Step 2: 失敗を確認**

Run: `. "$HOME/.cargo/env"; cargo test -p osm-cli encrypt_kanji 2>&1 | tail -15`
Expected: FAIL（`--kanji` 未定義の clap エラー）。

- [ ] **Step 3: 実装**

`crates/cli/src/main.rs` の `Encrypt` バリアントに `--kanji` を追加（`words` の隣）:
```rust
        /// Output the kanji-mixed (experimental) form instead of standard.
        #[arg(long)]
        kanji: bool,
```
`Encrypt` アーム header を `Command::Encrypt { passphrase, m_cost, t_cost, p_cost, words, kanji } => {` に更新し、出力選択を変更（`encode_words_kanji` を core から import）:
```rust
            let out = if kanji {
                encode_words_kanji(&payload)
            } else if words {
                encode_words(&payload)
            } else {
                encode_standard(&payload)
            };
```
use 行に `encode_words_kanji` を追加。

- [ ] **Step 4: テストを通す**

Run: `. "$HOME/.cargo/env"; cargo test -p osm-cli 2>&1 | tail -15`
Expected: PASS（既存 4 + 新規 = 5）。

- [ ] **Step 5: コミット**

```bash
git add crates/cli/src/main.rs crates/cli/tests/cli.rs
git commit -m "feat(cli): --kanji save format (#2)"
```

---

### Task 6: web UI に漢字形式（実験）を追加

**Files:**
- Modify: `web/src/crypto/client.ts`（`SaveFormat` 型）, `web/src/components/EncryptTab.tsx`
- Test: `web/src/components/EncryptTab.test.tsx`

**Interfaces:**
- Consumes: worker→wasm `encrypt(format)` が `"kanji"` を受理（Task 4）。`SaveFormat = "standard" | "words" | "kanji"`。
- Produces: 保存形式 select に「漢字混じり（実験）」追加＋実験注記。

- [ ] **Step 1: `SaveFormat` 型を拡張**

`web/src/crypto/client.ts` の `SaveFormat` を確認し `"kanji"` を追加（例：`export type SaveFormat = "standard" | "words" | "kanji";`）。worker は format 文字列をそのまま wasm へ渡すため変更不要（要確認：`web/src/crypto/worker.ts` が format を透過していること）。

- [ ] **Step 2: 失敗するテストを書く**

`web/src/components/EncryptTab.test.tsx` に追記（既存の render パターンに合わせる）:

```tsx
it("offers the experimental kanji save format", () => {
  render(<EncryptTab />);
  const select = screen.getByLabelText("保存形式");
  expect(within(select).getByRole("option", { name: /漢字混じり/ })).toBeInTheDocument();
});
```
（`within` を `@testing-library/react` から import。未使用なら import 追加。）

- [ ] **Step 3: 失敗を確認**

Run: `cd web && pnpm test --run EncryptTab 2>&1 | tail -15`
Expected: FAIL（漢字オプション無し）。

- [ ] **Step 4: 実装**

`web/src/components/EncryptTab.tsx` の保存形式 `<select>` に option 追加:
```tsx
            <option value="standard">標準形式</option>
            <option value="words">日本語単語列形式</option>
            <option value="kanji">漢字混じり（実験）</option>
```
select 直下に実験注記を追加（`format === "kanji"` のとき表示）:
```tsx
          {format === "kanji" && (
            <p className="field-hint mt-1 text-amber-700">
              実験的な形式です。長期保存には標準形式を推奨します。
            </p>
          )}
```

- [ ] **Step 5: テストを通す**

Run: `cd web && pnpm test --run 2>&1 | grep -E "Test Files|Tests " && pnpm exec tsc --noEmit && echo tsc-ok`
（必要なら先に `pnpm install --frozen-lockfile` と `pnpm build:wasm`。）
Expected: PASS、tsc-ok。

- [ ] **Step 6: コミット**

```bash
git add web/src/crypto/client.ts web/src/components/EncryptTab.tsx web/src/components/EncryptTab.test.tsx
git commit -m "feat(web): experimental kanji save format option (#2)"
```

---

### Task 7: SPEC・テストベクタ・ドキュメント同期

**Files:**
- Modify: `spec/SPEC.md`, `spec/test-vector.json`
- Modify: `docs/要件定義.md`, `docs/superpowers/specs/2026-06-17-open-secret-memo-design.md`

**Interfaces:** なし（仕様・ドキュメント）。

- [ ] **Step 1: SPEC に漢字形式を追記**

`spec/SPEC.md` に節を追加：符号化規則は単語列形式と同一・表のみ `bip39-japanese-kanji.txt`（index 一致の漢字スキン）・採用条件（常用漢字/送り仮名なし/同音異義非分裂/NFKC安定）・凍結ポリシー・自動判定順序（標準→ひらがな→漢字）・**実験扱い**注記。出自（bitcoin/bips japanese.txt 派生、JMdict/KANJIDIC2 を生成 AID に使用、いずれも EDRDG）を明記。

- [ ] **Step 2: テストベクタに漢字例を追加**

`spec/test-vector.json` に既存と同じ payload から `encode_words_kanji` した文字列を 1 件追加（フィールド名は既存ベクタ構造に合わせる。例：`"kanji": "..."`）。値は実装後に `osm encrypt --kanji` 等で生成し貼る。`crates/core/src/vectors.rs` のベクタ検証が漢字フィールドも照合するよう必要なら 1 行追加。

- [ ] **Step 3: 既存ドキュメントのステータス更新**

`docs/superpowers/specs/2026-06-17-open-secret-memo-design.md` の「MVP後の形式」記述を「漢字混じり形式は #2 で実装（index 一致の漢字スキン・実験扱い）」へ更新。`docs/要件定義.md` の保存形式案にも 1 行追記。

- [ ] **Step 4: 検証＋コミット**

Run: `. "$HOME/.cargo/env"; cargo test -p osm-core 2>&1 | tail -8`（ベクタ検証含め PASS）
```bash
git add spec/SPEC.md spec/test-vector.json crates/core/src/vectors.rs docs/要件定義.md docs/superpowers/specs/2026-06-17-open-secret-memo-design.md
git commit -m "docs(spec): document experimental kanji format + test vector (#2)"
```

---

## 実行メモ
- **Task 1 の人手監査ゲート**が全体の前提。カバレッジ結果によっては「常用＋人名用へ緩和」「特定語の手修正」を行ってから Task 2 以降へ。
- Task 2–7 は表非依存（どの語が漢字でもコードは不変）なので、監査確定後は機械的に進む。
- 凍結後の `bip39-japanese-kanji.txt` 変更は禁止（互換性破壊）。
