# Open Secret Memo 設計書

作成日: 2026-06-17
対象コンテスト: お悩み解決サイト選手権（締切 2026-06-30）
関連: [要件定義.md](../../要件定義.md)

## 1. 概要

**Open Secret Memo** は、見られたくないメモをブラウザ内だけで暗号化し、読めない文字列として
任意の場所（クラウドメモ・チャット・自分宛メール等）に保存できるローカル完結型の秘密メモツール。

最大のこだわりは **「サイトが消えた未来でも復号できる」** こと。そのため暗号ロジックを依存最小の
Rustコアに集約し、バイト単位の仕様書（SPEC.md）・テストベクター・復号キット・OSS公開・IPFSミラーを
用意する。

- プロダクト名: Open Secret Memo（確定）
- ドメイン: `osm.syou.io`（Cloudflare Pages）
- GitHubリポジトリ: `open-secret-memo`
- 復号キット配布: GitHub Releases
- 暗号方式: Argon2id + AES-256-GCM

### やらないこと（不変の制約）

- 平文メモ・合言葉・復号結果をサーバーに送信しない／保存しない
- 復号済みメモを localStorage に保存しない
- 秘密メモ・暗号文・合言葉を IPFS/Filecoin に保存しない
- 独自暗号を作らない
- 「絶対安全」とは言わない
- AIに合言葉・暗号文・復号結果を送る前提にしない（AIに渡すのは仕様書のみ）

## 2. アーキテクチャ

### リポジトリ構成（Cargo workspace + Web モノレポ）

```
open-secret-memo/
├── crates/
│   ├── core/     # 暗号ロジック（純粋Rust lib、依存最小、唯一の真実）
│   ├── wasm/     # wasm-bindgen ラッパ → wasm-pack で npm パッケージ出力
│   └── cli/      # core を使う復号/暗号化バイナリ
├── web/          # Vite + React + TS + Tailwind + PWA、wasmパッケージを import
├── spec/
│   ├── SPEC.md           # バイト単位のフォーマット仕様
│   └── test-vector.json  # core が生成・検証する正準テストベクター
├── scripts/
│   └── build-recovery-kit.sh   # 復号キットzipを組み立てる
└── README.md
```

### 依存の向き

`core` ← `wasm` ← `web`、`core` ← `cli`。
core は他から依存されるだけで、暗号crate以外に依存しない。これにより将来の再実装・監査・
テストベクター生成が容易になる。

### 配布チャネル

| 対象 | チャネル |
|------|---------|
| 本体サイト | Cloudflare Pages（`osm.syou.io`） |
| ソースコード | GitHub（`open-secret-memo`） |
| 復号キット | GitHub Releases（`build-recovery-kit.sh` がzip化して添付） |
| IPFSミラー | UIにCID欄を用意（実アップはMVP後） |

復号キットの中身: wasm-pack出力 + 最小HTML/JS + SPEC.md + README_RESTORE.md + test-vector.json。
リポジトリにビルド成果物はコミットしない。`file://` でWASMが動かない場合に備え、
`python3 -m http.server 8080` 等のローカルサーバー起動手順を README_RESTORE.md に記載する。

### 技術スタック

- フロント: Vite + React + TypeScript + Tailwind CSS + PWA
- WASM: Rust / wasm-bindgen / argon2 / aes-gcm / getrandom / base64 / serde・serde_json
- CLI: Rust（core を共用）

## 3. 暗号コアとペイロード形式

### 鍵導出

```
input    = UTF-8( NFKC( trim(合言葉) ) )
key(32B) = Argon2id(input, salt, m_cost, t_cost, p_cost)
```

正規化（NFKC + 前後trim）は **合言葉のみ** に適用する。メモ本文はバイト列をそのまま保持する。

合言葉の正規化手順（SPEC.md にも明記）:
1. Unicode文字列として扱う
2. Unicode NFKC で正規化
3. 前後の空白を trim
4. UTF-8 バイト列に変換
5. Argon2id の入力にする

### ペイロードのバイト構造（唯一の正準形式・SPEC.mdの中核）

| オフセット | サイズ | 内容 |
|-----------|--------|------|
| 0  | 3B  | マジック `"OSM"` (0x4F 0x53 0x4D) |
| 3  | 1B  | バージョン (= 0x01) |
| 4  | 4B  | Argon2 m_cost（KiB単位、ビッグエンディアン。64MiB = 65536） |
| 8  | 4B  | Argon2 t_cost（反復回数、BE） |
| 12 | 1B  | Argon2 p_cost（並列数） |
| 13 | 16B | salt |
| 29 | 12B | nonce |
| 41 | 可変 | AES-256-GCM 暗号文 + 認証タグ（末尾16B） |

- ヘッダ（先頭41B = magic〜nonce）を **AAD** として GCM に認証させる。これによりパラメータ・
  salt・nonce の改竄を検出できる。
- 整数はすべてビッグエンディアン。
- バージョンは 1 から開始。バイナリ構造が変わったらインクリメントする。

### テキスト表現（ペイロードの外側ラッパ）

**標準形式（確定・デフォルト）**
```
"OSM" + <version> + "." + base64url(payload)
例: OSM1.T1NNAQ...
```
先頭の `OSM1.` はバイナリ内 magic+version と冗長だが意図的。人間の目視・形式ルーティング用であり、
生バイトを受け取る再実装はバイナリ側 magic で検証できる。

**日本語単語列形式（確定）**
- payload のビット列を 11 ビット単位で基数2048エンコードし、BIP-39 日本語(ひらがな) 2048語を
  「、」で連結する。
- 端数ビットは仕様で固定したパディング規則で扱う（SPEC.md に明記）。
- POC ではチェックサムワードなし（純粋な基数2048エンコード）。
- 注意書き: ウォレットのシードフレーズと誤解されないよう UI で明示する。

**MVP後の形式（今回スコープ外）**
- 目立ちにくい形式（payloadのみ・プレフィックスなし）
- 漢字混じりメモ風形式（実験扱い、標準化された漢字2048語リストがなく長期保存には非推奨）
- 単語列形式のチェックサムワード版（誤記検出オプション）

### core が公開するAPI（薄いインターフェース）

```
encrypt(plaintext: &[u8], passphrase: &str, params: Argon2Params) -> Payload
decrypt(payload: &Payload, passphrase: &str) -> Result<Vec<u8>, DecryptError>
encode_standard(&Payload) -> String   /   decode_standard(&str) -> Result<Payload, _>
encode_words(&Payload) -> String      /   decode_words(&str)    -> Result<Payload, _>
detect_and_decode(&str) -> Result<Payload, DecryptError>   // 自動判別
```

テストベクター生成時のみ salt/nonce を固定値に注入できるよう、core の乱数源を差し替え可能にする。

### Argon2id パラメータ

- デフォルト（OWASP推奨）: m_cost 64MiB（65536 KiB）・t_cost 1・p_cost 1（処理時間目安 1〜2秒）
- 上級者オプション: 暗号化画面のプルダウンで m_cost / t_cost / p_cost を変更可能。
- パラメータは salt・nonce とともにペイロードへ埋め込み、復号時に自動読み取りする。

## 4. エラー処理とバリデーション

### 復号エラーの分類（`DecryptError`）

| エラー | 原因 | ユーザー向けメッセージ |
|--------|------|----------------------|
| `MalformedInput` | base64/単語列のデコード失敗、長さ不正、magic不一致 | 「暗号文の形式が正しくありません。全体をコピーできているか確認してください」 |
| `UnsupportedVersion` | version バイトが未知 | 「この暗号文は新しい版で作られています。最新版アプリで復号してください」 |
| `InvalidWord`（単語列形式のみ） | リストにない単語を検出 | 「『○○』は単語リストにありません。写し間違いの可能性があります」（語を特定して指摘） |
| `AuthenticationFailed` | GCMタグ不一致 | 「復号できませんでした。合言葉が違うか、暗号文が壊れている可能性があります」 |

形式A（チェックサムワードなし）では GCMタグ失敗時に「合言葉違い」と「写し間違い」を区別できないため、
メッセージは両方の可能性を示す。ただし単語列形式はリスト外の語を復号前に検出できるので、
写し間違いの一部は `InvalidWord` で捕捉できる。

### 暗号化側のバリデーション

| 項目 | 動作 |
|------|------|
| メモ本文が空 | 暗号化ボタン無効化 |
| 合言葉が空 | 暗号化ボタン無効化 |
| 合言葉 ≠ 合言葉確認 | 暗号化ボタン無効化＋不一致表示 |
| 弱い合言葉 | **警告表示のみ・ブロックしない**（語数や長さで強度を推定表示） |
| 上級者オプションのArgon2値が範囲外 | ブロック＋範囲を提示（m_cost下限/上限、t_cost≥1、p_cost≥1） |

### WASM技術的注意

m_cost=64MiB の Argon2id は WASM 線形メモリを確保する。wasm-pack/wasm-bindgen でメモリ拡張を
許可し、上級者が 256MiB 等を選んだ場合のブラウザ上限も考慮する（範囲チェックで過大値を防ぐ）。

## 5. テスト戦略

### test-vector.json（「decryptable forever」の要）

固定の合言葉・平文・salt・nonce・Argon2パラメータから決定論的に既知のpayloadを生成する。
これにより任意の再実装が正しさを検証できる。各ケースの構造:

```json
{
  "name": "ascii-basic",
  "passphrase": "紙袋、みかん、夜道、ラジオ",
  "plaintext_utf8": "secret note",
  "argon2": { "m_cost": 65536, "t_cost": 1, "p_cost": 1 },
  "salt_hex": "...(16B)",
  "nonce_hex": "...(12B)",
  "payload_hex": "...",
  "standard": "OSM1....",
  "words": "あいこくしん、..."
}
```

### ユニットテスト（TDDで実装）

- encrypt → decrypt ラウンドトリップ
- 合言葉違いで `AuthenticationFailed`
- ヘッダ改竄（AAD）で失敗 → パラメータ/salt/nonce の保護を実証
- NFKC正規化: 半角/全角の合言葉が同一鍵になる、trim挙動
- 単語列 encode ⇄ decode ラウンドトリップ、リスト外語の検出
- エッジ: 空平文・1バイト・大きめ平文
- `detect_and_decode` が標準/単語列を正しく振り分け
- malformed入力（壊れたbase64・長さ不正・未知version）
- test-vector.json の全ケースに一致することを検証（再実装の基準と同一テスト）

### プロパティテスト（proptest）

- 任意の平文 + 任意の合言葉 + 有効パラメータ → `decrypt(encrypt(x)) == x`
- 任意のpayload → `decode(encode(p)) == p`（標準形式・単語列形式の両方）
- 単語列 ⇄ バイト列の基数2048変換が任意長で可逆

### ファズテスト（cargo-fuzz / libFuzzer）

- ターゲット: `decode_standard` / `decode_words` / `detect_and_decode` / `decrypt`
- 任意のバイト列・文字列を投入しても panic せず、必ず `Ok`/`Err` を返すことを保証する。
- 信頼できない入力（貼り付けられた壊れた暗号文）を扱う境界なので、クラッシュ・範囲外参照・
  無限ループがないことを継続的に検査する。
- CIでは短時間（時間/反復上限）、コーパスはリポジトリに保持する。

### CLI / wasm / web

- CLI: `verify` サブコマンドで test-vector.json との一致を確認できる（復号キット利用者が自検証可能）。
- wasm: wasm出力が core と同一結果を返すスモークテスト。
- web: 暗号化/復号フローのコンポーネントテスト。PWAオフライン/e2e（Playwright）はMVP後。

テスト層は「ユニット + test-vector検証 + プロパティ + ファズ」の4段構成とする。

## 6. UI/UX

### 画面構成

単一ページ + タブ（「暗号化」/「復号」）。トップに概要と「このサイトは安全？」アコーディオン。

### 暗号化タブ

- メモ本文（textarea）
- 合言葉／合言葉確認（マスク表示＋表示切替、不一致表示、弱い場合は強度警告のみ）
- 保存形式セレクタ（標準／単語列、デフォルト=標準）
- 上級者オプション（折りたたみプルダウン: Argon2 m_cost / t_cost / p_cost、デフォルトOWASP値）
- 「暗号化後にメモ本文・合言葉をクリア」チェック（**デフォルトOFF**）
- 暗号化ボタン → 出力: 暗号化済みテキスト＋コピー＋txt保存
- 常時表示: 「この内容はサーバーに送信されません」「合言葉を忘れると復号できません」

### 復号タブ

- 暗号化済みテキスト（textarea、**自動判別**）
- 合言葉（マスク＋表示切替）
- 復号ボタン → 出力: 復号済みメモ＋コピー＋「表示を隠す」（**手動のみ・自動非表示なし**）
- エラーは第4章の分類メッセージを表示

### 「このサイトは安全？」アコーディオン

送信しないもの／保存しないもの／通信なしで動く／ソースコード(GitHub)／仕様書／IPFS CID／
バージョン／暗号方式 を表示する。

### 合言葉ガイド文（UI文案）

```
合言葉には日本語も使えます。
おすすめは、無関係な単語を「、」で6個以上つなげる方法です。
文章型の合言葉も使えます。
合言葉を忘れると復号できません。
このサイトにも保存されません。
```

推奨パスフレーズ強度の目安: 単語型は最低4語・推奨6語・重要情報8語以上。文章型も許容する。

## 7. PWA

- manifest + service worker
- HTML/JS/CSS/**WASM** をプリキャッシュする
- オフラインで暗号化・復号が完全動作する
- ホーム画面に追加・アプリ風起動ができる
- やらないこと: Push通知／バックグラウンド同期／端末間データ同期

## 8. localStorage 方針

MVP では暗号文も含め一切保存しない（UI設定・テーマ・暗号文の保存機能はすべて MVP後）。

## 実装で確定した仕様（後追い反映）

- 日本語単語列形式は、可逆性のため **4バイトのビッグエンディアン長さプレフィックス**（payloadバイト長）をビット列の先頭に付けてから11ビット単位でエンコードする。復号時はこの長さを読んで正確なバイト数を復元する。
- wasm 復号エラーの安定キー（UIが文言にマップ）: `malformed` / `unsupported_version` / `invalid_word` / `auth_failed` / `not_utf8`。
- 暗号処理（Argon2id）は UI を固めないよう **Web Worker** 上で実行する。

## 9. MVP スコープ整理

### MVP に含む

- core（Argon2id + AES-256-GCM、標準形式、日本語単語列形式、自動判別）
- wasm ラッパ + web（暗号化/復号タブ、上級者オプション、安全説明アコーディオン）
- CLI（暗号化/復号 + `verify`）
- PWA（オフライン動作）
- SPEC.md + test-vector.json
- 復号キット（GitHub Releases）
- テスト4段（ユニット/test-vector/プロパティ/ファズ）
- Cloudflare Pages デプロイ（`osm.syou.io`）

### MVP後

- 目立ちにくい形式、漢字混じりメモ風形式、単語列チェックサムワード版
- localStorage 保存機能（暗号文・UI設定）
- IPFS への実アップロード（CID欄はMVPで用意）
- 復号画面の自動非表示
- Filecoin 長期保存
- web の e2e/PWAテスト
