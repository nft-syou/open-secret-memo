# Open Secret Memo

**サイトが消えても復号できる、オープン仕様の秘密メモツール。**

🔗 **https://osm.syou.io**  ・  ソース: このリポジトリ  ・  仕様書: [`spec/SPEC.md`](spec/SPEC.md)

見られたくないメモを **ブラウザの中だけ** で暗号化し、読めない文字列として好きな場所（クラウドメモ・チャット・自分宛メール・付箋など）に保存できます。サーバーには何も送りません。そして万一このサイトが消えても、**OSS・仕様書・復号キット**で後から復号できるよう設計しています。

---

## 解決する悩み

APIキー、予約番号、2FAのバックアップコード、Wi‑Fiパスワード、緊急連絡先、ちょっとした個人情報——
「クラウドメモに書いておきたいけど、そのまま置くのは不安。でも専用パスワード管理アプリに入れるほどでもない」。

Open Secret Memo は、メモを **合言葉** で暗号化し、**暗号化済みテキストだけ** を手元の好きな場所に保存できるようにします。元の文章も合言葉も、どこにも送信・保存されません。

---

## こだわり・特徴

- 🔒 **ブラウザ内だけで暗号化／復号** — 通信なし。WASM（Rust製）で処理。
- 🧱 **実績ある暗号方式** — Argon2id（鍵導出）+ AES‑256‑GCM（認証付き暗号）。独自暗号は作りません。
- 🈶 **日本語の「合言葉」** — 「パスワード」ではなく合言葉。日本語の単語列・文章型パスフレーズを正式サポート（NFKC正規化）。
- 🪂 **サイトが消えても復号できる** — OSS + バイト単位の仕様書 + テストベクター + オフライン復号キット。
- 📦 **オフライン対応PWA** — ホーム画面に追加、ネットなしで暗号化／復号。WASMもキャッシュ。
- 🗂️ **複数の保存形式** — 標準形式（`OSM1.…`）と、BIP‑39日本語ワードリストによる日本語単語列形式。
- 🙈 **復号結果を保存しない** — 表示は手動で隠せます。MVPでは暗号文も含め一切保存しません。
- 💻 **CLIも同梱** — 同じRustコアでローカルCLI復号が可能。

---

## 使い方（Webアプリ）

1. **暗号化**タブで、メモ本文と合言葉（＋確認）を入力 → 「暗号化する」。
2. 出力された暗号化済みテキストをコピー or `.txt` 保存して、好きな場所に保管。
3. 読みたいときは **復号**タブに暗号化済みテキストと合言葉を貼って「復号する」。形式は自動判別されます。

> ⚠️ **合言葉を忘れると復号できません。** 合言葉はどこにも保存されません（このサイトにも）。
> おすすめは、無関係な単語を「、」で6個以上つなげる方法です。文章型でもOK。

---

## サーバーに送らない／保存しないもの

| 送らない | 保存しない |
|---|---|
| メモ本文 | 平文メモ |
| 合言葉 | 合言葉 |
| 復号結果 | 復号結果 |

通信なし・オフラインで動作します。秘密メモ・暗号文・合言葉を外部サービス（AI含む）に送る前提にはしていません。

---

## 「サイトが消えても復号できる」設計

このプロジェクトの中心的なこだわりです。万一サイトやホスティングが失われても、以下で復号手段が残ります。

1. **OSS** — このリポジトリにすべての実装。
2. **仕様書 [`spec/SPEC.md`](spec/SPEC.md)** — 暗号フォーマットをバイト単位で記述。任意の言語で復号器を再実装できます。
3. **テストベクター [`spec/test-vector.json`](spec/test-vector.json)** — 固定の合言葉・salt・nonce から決定論的に生成した正準データ。再実装が正しいか自己検証できます。
4. **復号キット** — GitHub Releases で配布する `open-secret-memo-recovery-kit.zip`。HTML + WASM + 仕様書 + テストベクターを同梱し、**オフラインで復号専用ページが動きます**。

### 復号キットの使い方

`v*` タグを打つと、CIが復号キットをビルドして Release に添付します（[`.github/workflows/release.yml`](.github/workflows/release.yml)）。利用者はzipを展開し、`file://` ではWASMが動かないため簡易サーバーで開きます:

```bash
unzip open-secret-memo-recovery-kit.zip
cd open-secret-memo-recovery-kit
python3 -m http.server 8080
# ブラウザで http://localhost:8080 を開き、暗号化済みテキストと合言葉を入力
```

---

## 暗号方式（概要）

```
鍵 = Argon2id( NFKC(trim(合言葉)) , salt, m_cost, t_cost, p_cost )   // 32バイト
暗号文+タグ = AES-256-GCM( 鍵, nonce, 平文, AAD = ヘッダ )
```

- 既定の Argon2id パラメータ（OWASP推奨）: メモリ 64 MiB・反復 1・並列 1。上級者オプションでUIから変更可能。
- 自己記述型のバイナリペイロード（マジック `OSM`・バージョン・パラメータ・salt(16)・nonce(12)・暗号文+タグ）。先頭41バイトのヘッダを **AAD** として認証し、改竄を検出します。
- テキスト表現:
  - **標準形式**: `OSM1.` + base64url（既定）
  - **日本語単語列形式**: BIP‑39 日本語(ひらがな) 2048語を基数2048の文字盤として利用

詳細・正確な定義は [`spec/SPEC.md`](spec/SPEC.md) を参照してください。

---

## リポジトリ構成

```
crates/
  core/    # 暗号ロジック（純粋Rust・依存最小・唯一の真実）  ── osm-core
  wasm/    # wasm-bindgen ラッパ（Webから利用）            ── osm-wasm
  cli/     # ローカルCLI（encrypt / decrypt / verify）      ── osm (osm-cli)
web/        # Vite + React + TypeScript + Tailwind の PWA
recovery/   # オフライン復号専用ページ（復号キットに同梱）
spec/       # SPEC.md（バイト単位仕様）＋ test-vector.json（正準テストベクター）
scripts/    # build-recovery-kit.sh（復号キットのパッケージング）
docs/       # 設計・要件ドキュメント
```

---

## 開発

必要なツール: **Rust**（stable）, **wasm-pack**, **Node 22+**, **pnpm 11**。

```bash
# Rustコアのテスト（ユニット + テストベクター適合 + プロパティ）
cargo test --workspace

# WASMラッパのテスト（ブラウザが無ければ --node）
wasm-pack test --node crates/wasm

# Webアプリ（WASMをビルドしてから起動）
cd web
pnpm install
pnpm build:wasm     # crates/wasm を web/src/wasm/ に出力
pnpm dev            # 開発サーバ
pnpm test           # コンポーネント/ロジックのテスト
pnpm build          # 本番ビルド（dist/）
```

ファズテスト（任意・nightly + cargo-fuzz）:

```bash
cargo +nightly fuzz run decode_standard   -- -max_total_time=30
cargo +nightly fuzz run decode_words      -- -max_total_time=30
cargo +nightly fuzz run detect_and_decode -- -max_total_time=30
cargo +nightly fuzz run decrypt           -- -max_total_time=30
```

---

## CLI

同じRustコアをCLIからも使えます（サイト不要のローカル復号）。

```bash
# 暗号化（メモは標準入力、合言葉はフラグ）
echo -n "秘密のメモ" | cargo run -p osm-cli -- encrypt --passphrase "紙袋、みかん、夜道、ラジオ"
# → OSM1.… を出力（--words で日本語単語列形式）

# 復号（暗号化済みテキストは標準入力）
echo -n "OSM1.…" | cargo run -p osm-cli -- decrypt --passphrase "紙袋、みかん、夜道、ラジオ"

# 仕様適合チェック（このビルドがテストベクターを再現できるか）
cargo run -p osm-cli -- verify --vectors spec/test-vector.json
```

---

## デプロイ

- ホスティング: **Cloudflare Pages**（`main` への push で自動デプロイ — [`.github/workflows/deploy.yml`](.github/workflows/deploy.yml)）
- CI: Rustワークスペース / WASM / Web をテスト（[`.github/workflows/ci.yml`](.github/workflows/ci.yml)）
- リリース: `v*` タグで復号キットを Release に添付（[`.github/workflows/release.yml`](.github/workflows/release.yml)）

---

## セキュリティに関する注意

- 本ツールは実績ある標準暗号（Argon2id + AES‑256‑GCM）を使いますが、**「絶対安全」とは言いません**。安全性は最終的に合言葉の強さに依存します。
- 合言葉を忘れると復号は不可能です（バックドアはありません）。
- 目立ちにくい保存形式は補助的な工夫であり、主たる防御ではありません。

---

## ライセンス

MIT License（`LICENSE` を参照）。

---

> Open Secret Memo は、秘密メモをブラウザ内で暗号化し、サイトが消えても復号できるように、OSS・仕様書・復号キットまで用意したローカル完結型の秘密メモツールです。
