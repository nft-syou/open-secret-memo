# Open Secret Memo — Recovery Kit + Deploy Implementation Plan (Plan 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the project survive its own website disappearing: a self-contained, offline recovery kit attached to GitHub Releases, plus CI that builds/tests everything and deploys the PWA to Cloudflare Pages at `osm.syou.io`.

**Architecture:** A standalone recovery page (`recovery/`) reuses the same `osm-wasm` output but offers decrypt only, runnable from a downloaded zip with a local static server. `scripts/build-recovery-kit.sh` assembles the zip from built artifacts + SPEC + test vectors + restore instructions. GitHub Actions runs the full test matrix, packages the kit on tagged releases, and deploys the web build to Cloudflare Pages.

**Tech Stack:** Bash; GitHub Actions; Cloudflare Pages (Wrangler or Pages Git integration); reuses Plan 1 (`osm-core`, `spec/`) and Plan 2 (`osm-wasm`, `web/`).

**Prerequisite:** Plans 1 and 2 merged.

## Global Constraints

- The recovery kit must decrypt **fully offline** with no build step — only "download zip, run a local static server, open index.html".
- The kit bundles: `index.html`, `app.js`, the wasm glue + `.wasm`, `SPEC.md`, `README_RESTORE.md`, `test-vector.json`. No secrets, ever.
- Production domain: `osm.syou.io` (Cloudflare Pages). Vite `base` is `/` (root domain, set in Plan 2).
- Releases are tag-driven (`v*`). The kit filename is `open-secret-memo-recovery-kit.zip`.
- CI must run `cargo test` (workspace) + `wasm-pack test` + `pnpm test` and fail the build on any failure before packaging/deploying.

---

### Task 1: Standalone recovery page (decrypt-only)

**Files:**
- Create: `recovery/index.html`
- Create: `recovery/app.js`
- Create: `recovery/README_RESTORE.md`

**Interfaces:**
- Consumes: the `osm-wasm` ES module (`osm.js` + `osm_bg.wasm`), copied in at packaging time (Task 2).
- Produces: a zero-build HTML page that decrypts a pasted ciphertext with a passphrase.

- [ ] **Step 1: Write the recovery page HTML**

Create `recovery/index.html`:

```html
<!doctype html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Open Secret Memo — 復号キット</title>
    <style>
      body { font-family: sans-serif; max-width: 640px; margin: 2rem auto; padding: 0 1rem; }
      textarea, input { width: 100%; box-sizing: border-box; margin: 0.25rem 0; }
      #out { white-space: pre-wrap; border: 1px solid #ccc; padding: 0.5rem; min-height: 3rem; }
      .err { color: #b00; }
    </style>
  </head>
  <body>
    <h1>Open Secret Memo — 復号キット</h1>
    <p>このページはオフラインで動作します。合言葉・暗号文は送信されません。</p>
    <label>暗号化済みテキスト<textarea id="ct" rows="4"></textarea></label>
    <label>合言葉<input id="pw" type="password" /></label>
    <button id="go">復号する</button>
    <h2>結果</h2>
    <div id="out"></div>
    <script type="module" src="./app.js"></script>
  </body>
</html>
```

- [ ] **Step 2: Write the recovery page logic**

Create `recovery/app.js`:

```js
import init, { decrypt } from "./osm.js";

const errors = {
  auth_failed: "復号できませんでした。合言葉が違うか、暗号文が壊れている可能性があります。",
  malformed: "暗号文の形式が正しくありません。全体をコピーできているか確認してください。",
  unsupported_version: "この暗号文は新しい版で作られています。",
  invalid_word: "単語リストにない語があります。写し間違いの可能性があります。",
  not_utf8: "復号できましたが、テキストとして読み取れませんでした。"
};

async function main() {
  await init();
  const out = document.getElementById("out");
  document.getElementById("go").addEventListener("click", () => {
    out.classList.remove("err");
    const ct = document.getElementById("ct").value;
    const pw = document.getElementById("pw").value;
    const r = decrypt(ct, pw);
    if (r.ok) {
      out.textContent = r.text;
    } else {
      out.classList.add("err");
      out.textContent = errors[r.error_kind] || "復号中にエラーが発生しました。";
    }
  });
}

main();
```

- [ ] **Step 3: Write the restore instructions**

Create `recovery/README_RESTORE.md`:

```markdown
# Open Secret Memo 復号キット

このキットだけで、サイトが無くても暗号文を復号できます。

## 使い方

WASM はファイルを直接開く (`file://`) と動かないため、簡易サーバーを起動してください。

```bash
# このフォルダ内で
python3 -m http.server 8080
```

ブラウザで http://localhost:8080 を開き、暗号化済みテキストと合言葉を入力して
「復号する」を押します。

## 含まれるもの

- index.html / app.js / osm.js / osm_bg.wasm … 復号アプリ本体
- SPEC.md … 暗号フォーマットの完全仕様（別言語での再実装が可能）
- test-vector.json … 実装が正しいか検証するためのテストベクター

## サイトもキットも失われた場合

SPEC.md と test-vector.json があれば、任意の言語で復号器を再実装できます。
合言葉・暗号文・復号結果は決してAIや外部サービスに送らず、ローカルで処理してください。
```

- [ ] **Step 4: Commit**

```bash
git add recovery/
git commit -m "feat(recovery): offline decrypt-only recovery page and restore docs"
```

---

### Task 2: Recovery kit packaging script

**Files:**
- Create: `scripts/build-recovery-kit.sh`

**Interfaces:**
- Consumes: `recovery/`, `spec/SPEC.md`, `spec/test-vector.json`, and a freshly built `osm-wasm` (`--target web`).
- Produces: `dist/open-secret-memo-recovery-kit.zip`.

- [ ] **Step 1: Write the packaging script**

Create `scripts/build-recovery-kit.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

STAGE="$(mktemp -d)"
KIT="$STAGE/open-secret-memo-recovery-kit"
mkdir -p "$KIT"

# 1. Build the wasm for plain web (no bundler) into the kit.
wasm-pack build crates/wasm --target web --out-dir "$KIT" --out-name osm
# wasm-pack writes package.json/.gitignore we do not need in the kit.
rm -f "$KIT/package.json" "$KIT/.gitignore" "$KIT/README.md"

# 2. Copy the recovery page and docs.
cp recovery/index.html recovery/app.js recovery/README_RESTORE.md "$KIT/"
cp spec/SPEC.md spec/test-vector.json "$KIT/"

# 3. Zip it.
mkdir -p dist
( cd "$STAGE" && zip -r -q open-secret-memo-recovery-kit.zip open-secret-memo-recovery-kit )
mv "$STAGE/open-secret-memo-recovery-kit.zip" dist/
rm -rf "$STAGE"
echo "built dist/open-secret-memo-recovery-kit.zip"
```

- [ ] **Step 2: Build and smoke-test the kit locally**

Run:
```bash
chmod +x scripts/build-recovery-kit.sh
./scripts/build-recovery-kit.sh
unzip -l dist/open-secret-memo-recovery-kit.zip
```
Expected: the listing contains `index.html`, `app.js`, `osm.js`, `osm_bg.wasm`, `SPEC.md`, `test-vector.json`, `README_RESTORE.md`.

- [ ] **Step 3: Verify the kit decrypts offline**

Run:
```bash
cd dist && unzip -o open-secret-memo-recovery-kit.zip >/dev/null
cd open-secret-memo-recovery-kit && python3 -m http.server 8099 &
SERVER=$!
sleep 1
# Sanity: the wasm and app are served.
curl -fsS http://localhost:8099/osm_bg.wasm -o /dev/null && echo "wasm served OK"
kill $SERVER
```
Then manually open `http://localhost:8099`, paste a ciphertext produced by the web app, and confirm decryption. (Browser step is manual.)

- [ ] **Step 4: Commit**

```bash
git add scripts/build-recovery-kit.sh
git commit -m "feat(recovery): build-recovery-kit.sh packaging script"
```

---

### Task 3: CI — test workspace + web on every push/PR

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:** None (CI). Gates merges on green tests.

- [ ] **Step 1: Write the CI workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Vendor BIP-39 wordlist
        run: |
          test -f crates/core/data/bip39-japanese.txt || \
            curl -fsSL https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/japanese.txt \
              -o crates/core/data/bip39-japanese.txt
      - name: cargo test (workspace)
        run: cargo test --workspace
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: wasm-pack test
        run: wasm-pack test --headless --firefox crates/wasm

  web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
          cache-dependency-path: web/pnpm-lock.yaml
      - name: Vendor wordlist + build wasm + test web
        run: |
          test -f crates/core/data/bip39-japanese.txt || \
            curl -fsSL https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/japanese.txt \
              -o crates/core/data/bip39-japanese.txt
          cd web
          pnpm install --frozen-lockfile
          pnpm build:wasm
          pnpm test --run
          pnpm exec tsc --noEmit
```

> Note: `data/wordlist_array.in` is generated in Plan 1 Task 6 and committed, so CI only needs the raw wordlist if that file references it via `include!`. If `wordlist_array.in` is committed, the vendor step is belt-and-suspenders; keep it for robustness.

- [ ] **Step 2: Validate the workflow locally (syntax)**

Run:
```bash
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('valid yaml')"
```
Expected: `valid yaml`.

- [ ] **Step 3: Commit and confirm CI runs green**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: test rust workspace, wasm, and web"
git push
```
Then check the Actions tab: both `rust` and `web` jobs pass.

---

### Task 4: Release workflow — build + attach recovery kit

**Files:**
- Create: `.github/workflows/release.yml`

**Interfaces:** None (CI). Triggered by `v*` tags.

- [ ] **Step 1: Write the release workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  kit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Vendor wordlist
        run: |
          test -f crates/core/data/bip39-japanese.txt || \
            curl -fsSL https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/japanese.txt \
              -o crates/core/data/bip39-japanese.txt
      - name: Build recovery kit
        run: bash scripts/build-recovery-kit.sh
      - name: Attach kit to the release
        uses: softprops/action-gh-release@v2
        with:
          files: dist/open-secret-memo-recovery-kit.zip
```

- [ ] **Step 2: Validate YAML**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('valid yaml')"
```
Expected: `valid yaml`.

- [ ] **Step 3: Commit and dry-run with a test tag**

```bash
git add .github/workflows/release.yml
git commit -m "ci: package and attach recovery kit on version tags"
git push
git tag v0.1.0-rc1
git push origin v0.1.0-rc1
```
Then confirm the Release workflow runs and `open-secret-memo-recovery-kit.zip` appears on the `v0.1.0-rc1` GitHub Release. Delete the pre-release/tag afterward if it was only a dry run.

---

### Task 5: Cloudflare Pages deploy to `osm.syou.io`

**Files:**
- Create: `.github/workflows/deploy.yml`
- Modify: `web/src/lib/appInfo.ts` (replace `<owner>` URLs with the real repo)

**Interfaces:** None (deploy). Publishes `web/dist` to Cloudflare Pages.

**One-time manual setup (document in PR description, not code):**
1. Create a Cloudflare Pages project (e.g. `open-secret-memo`).
2. Add custom domain `osm.syou.io` in the Pages project (DNS via Cloudflare).
3. Create repo secrets: `CLOUDFLARE_API_TOKEN` (Pages:Edit), `CLOUDFLARE_ACCOUNT_ID`.

- [ ] **Step 1: Replace placeholder repo URLs**

Edit `web/src/lib/appInfo.ts`: replace both `<owner>` occurrences with the real GitHub owner/repo (e.g. `github.com/<your-user>/open-secret-memo`). Leave `IPFS_CID` empty until the first pin (post-MVP).

- [ ] **Step 2: Write the deploy workflow**

Create `.github/workflows/deploy.yml`:

```yaml
name: Deploy

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
          cache-dependency-path: web/pnpm-lock.yaml
      - name: Vendor wordlist
        run: |
          test -f crates/core/data/bip39-japanese.txt || \
            curl -fsSL https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/japanese.txt \
              -o crates/core/data/bip39-japanese.txt
      - name: Build web (includes wasm)
        run: |
          cd web
          pnpm install --frozen-lockfile
          pnpm build
      - name: Deploy to Cloudflare Pages
        uses: cloudflare/wrangler-action@v3
        with:
          apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          accountId: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
          command: pages deploy web/dist --project-name=open-secret-memo
```

- [ ] **Step 3: Validate YAML**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/deploy.yml')); print('valid yaml')"
```
Expected: `valid yaml`.

- [ ] **Step 4: Commit, push, and verify the live site**

```bash
git add .github/workflows/deploy.yml web/src/lib/appInfo.ts
git commit -m "ci: deploy web PWA to Cloudflare Pages (osm.syou.io)"
git push
```
Then confirm:
- The Deploy workflow succeeds.
- `https://osm.syou.io` loads, encrypt/decrypt work, and the "このサイトは安全？" links point at the real repo.
- Lighthouse/DevTools shows the PWA is installable and the service worker is active.

---

## Self-Review

**Spec coverage (design doc §2 distribution, §1 "decryptable forever", contest publish requirement):**
- Recovery kit (index.html, app.js, wasm, SPEC.md, README_RESTORE.md, test-vector.json) → Tasks 1–2 ✓
- Kit delivered via GitHub Releases (not committed build output) → Task 4 ✓
- `file://` caveat + local server instructions → Task 1 README_RESTORE.md ✓
- Cloudflare Pages + custom domain `osm.syou.io` → Task 5 ✓
- Source on GitHub, public before result announcement (contest rule) → CI/deploy make publishing routine; repo is the source of truth ✓
- IPFS: CID field wired in Plan 2's accordion, real pin deferred (post-MVP) — appInfo `IPFS_CID` left empty intentionally ✓
- Full test gate before packaging/deploy (cargo + wasm-pack + pnpm) → Task 3 ✓

**Placeholder scan:** `<owner>` URL placeholders from Plan 2 are explicitly resolved in Task 5 Step 1. No silent TODOs. Manual one-time Cloudflare setup is documented as steps, not code. ✓

**Type/name consistency:** Kit reuses the exact `osm-wasm` `decrypt`/`DecryptOutcome` (`ok`, `text`, `error_kind`) from Plan 2; recovery `app.js` reads the same field names. Project name `open-secret-memo` and domain `osm.syou.io` consistent across scripts and workflows. ✓
