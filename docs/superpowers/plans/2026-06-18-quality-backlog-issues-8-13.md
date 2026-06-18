# Quality Backlog (Issues #8–#13) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the post-MVP quality backlog for Open Secret Memo — fill test gaps, harden the CLI, tighten wasm/core ergonomics, improve web accessibility, gate deployment on CI, and sync the design docs with what was actually built.

**Architecture:** A Cargo workspace (`osm-core`, `osm-wasm`, `osm-cli`) plus a Vite/React/Tailwind PWA in `web/`. This plan only adds tests, hardens error paths, adds docs/attributes, and edits CI YAML — **no change to the on-disk ciphertext format, the crypto, or any success-path behavior.**

**Tech Stack:** Rust (stable; nightly only for existing fuzz), `wasm-pack`, Node 22, pnpm 11, Vitest + @testing-library/react, Playwright (new), GitHub Actions, Cloudflare Pages.

## Global Constraints

- **No format/crypto change.** Do not touch the binary payload layout, Argon2id/AES-GCM logic, or the `encrypt`/`decrypt` success path. These are quality/robustness/test/docs changes only.
- **Preserve privacy invariants:** no `console.*` logging of secrets, no network calls from app code, no persistence (no localStorage), decrypt reveal/hide stays manual (no auto-hide), clear-after-encrypt default OFF, weak-passphrase warning-only.
- Stable wasm decrypt `error_kind` values stay exactly: `malformed`, `unsupported_version`, `invalid_word`, `auth_failed`, `not_utf8`.
- Rust tests use small Argon2 memory (`m_cost = 8192`) for speed.
- Toolchain: pnpm 11 / Node 22; if `cargo`/`wasm-pack` not on PATH run `. "$HOME/.cargo/env"`. No browser locally → wasm tests use `wasm-pack test --node`; Playwright downloads its own Chromium.
- Japanese is the UI/user-facing language; keep existing Japanese strings verbatim unless a task changes them.
- **Already done (do NOT redo):** CLI `verify` graceful file read + `hex_decode` returning `Result` (issue #9), and the `words.rs` `WORDLIST` comment fix (issue #10) — both already merged. This plan covers only the remaining sub-items.

---

### Task 1: core — public doc comments + edge-case unit tests (#10 docs, #8 core tests)

**Files:**
- Modify: `crates/core/src/encoding/standard.rs`
- Modify: `crates/core/src/encoding/words.rs`

**Interfaces:**
- Consumes: existing `encode_standard`/`decode_standard` (standard.rs), `decode_words` (words.rs), `FormatError`.
- Produces: no API change; adds `///` docs and tests only.

- [ ] **Step 1: Add the failing tests**

In `crates/core/src/encoding/standard.rs`, inside the existing `#[cfg(test)] mod tests { ... }` block, add:

```rust
    #[test]
    fn rejects_multi_digit_version_prefix() {
        // "OSM12." has a two-character version field; only a single ASCII digit is valid.
        assert_eq!(decode_standard("OSM12.AAAA"), Err(FormatError::Malformed));
    }
```

In `crates/core/src/encoding/words.rs`, inside its `#[cfg(test)] mod tests { ... }` block, add:

```rust
    #[test]
    fn empty_input_is_malformed() {
        assert_eq!(decode_words(""), Err(FormatError::Malformed));
        assert_eq!(decode_words("   "), Err(FormatError::Malformed));
    }
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p osm-core encoding:: 2>&1 | tail -20`
Expected: PASS. (Both behaviors already hold — `decode_standard` rejects via the `ver_str.len() != 1` guard, and `decode_words` returns `Malformed` via its `bytes.len() < 4` guard. These tests pin that behavior.)

- [ ] **Step 3: Add doc comments to the public encoding functions**

In `crates/core/src/encoding/standard.rs`, add a `///` line directly above `encode_standard` and `decode_standard`:

```rust
/// Encode a payload as the standard text form: `"OSM" + <version digit> + "." + base64url(no pad)`.
pub fn encode_standard(payload: &Payload) -> String {
```

```rust
/// Decode the standard text form (`OSM<digit>.<base64url>`). Surrounding whitespace is trimmed.
/// Returns [`FormatError::Malformed`] if the prefix, version digit, or base64 body is invalid.
pub fn decode_standard(s: &str) -> Result<Payload, FormatError> {
```

In `crates/core/src/encoding/words.rs`, add a `///` line above `encode_words` and `decode_words`:

```rust
/// Encode a payload as a Japanese BIP-39 word sequence (base-2048), words joined by `、` (U+3001).
pub fn encode_words(payload: &Payload) -> String {
```

```rust
/// Decode a Japanese BIP-39 word sequence back into a payload.
/// Returns [`FormatError::InvalidWord`] for an unknown word, or [`FormatError::Malformed`] for empty/short input.
pub fn decode_words(s: &str) -> Result<Payload, FormatError> {
```

- [ ] **Step 4: Verify the crate still builds, tests pass, and docs are warning-free**

Run: `cargo test -p osm-core 2>&1 | grep -E "test result|warning: " | tail`
Expected: all tests PASS; no new warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/encoding/standard.rs crates/core/src/encoding/words.rs
git commit -m "docs(core): document public encoding fns; test edge cases (#8 #10)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: wasm — strict format, version diagnostics, not_utf8 test (#10 wasm, #8 wasm test)

**Files:**
- Modify: `crates/wasm/src/lib.rs`
- Modify: `crates/wasm/tests/web.rs`

**Interfaces:**
- Consumes: `osm_core` (`encrypt`/`decrypt`/`encode_standard`/`FixedRng`/`Argon2Params`), already a normal dependency so usable from the integration test.
- Produces: `encrypt(..., format)` now rejects unknown `format` values via `JsError`; `decrypt` carries the unsupported version number in `error_word`. Stable `error_kind` strings unchanged.

- [ ] **Step 1: Make `encrypt` reject unknown formats and `decrypt` report the version**

In `crates/wasm/src/lib.rs`, replace the `Ok(match format { ... })` block in `encrypt` with an explicit match that errors on anything other than the two known formats:

```rust
    match format {
        "standard" => Ok(encode_standard(&payload)),
        "words" => Ok(encode_words(&payload)),
        other => Err(JsError::new(&format!("unknown format: {other}"))),
    }
}
```

In the same file, change the `UnsupportedVersion` arm of `from_format_error` to surface the version number in `error_word`:

```rust
        FormatError::UnsupportedVersion(v) => err("unsupported_version", &v.to_string()),
```

- [ ] **Step 2: Add the failing tests**

In `crates/wasm/tests/web.rs`, add `use osm_core::{encrypt as core_encrypt, encode_standard, Argon2Params, FixedRng};` to the imports, then add:

```rust
#[wasm_bindgen_test]
fn unknown_format_is_rejected() {
    assert!(encrypt("hi", "pw", 8192, 1, 1, "STANDARD").is_err());
    assert!(encrypt("hi", "pw", 8192, 1, 1, "bogus").is_err());
}

#[wasm_bindgen_test]
fn non_utf8_plaintext_reports_not_utf8() {
    // Build a ciphertext whose plaintext is invalid UTF-8, using the core directly.
    let mut rng = FixedRng::new(vec![7u8]);
    let params = Argon2Params { m_cost: 8192, t_cost: 1, p_cost: 1 };
    let payload = core_encrypt(&[0xff, 0xfe, 0xfd], "pw", params, &mut rng);
    let ciphertext = encode_standard(&payload);
    let outcome = decrypt(&ciphertext, "pw");
    assert!(!outcome.ok);
    assert_eq!(outcome.error_kind, "not_utf8");
}
```

- [ ] **Step 3: Run the wasm tests (node runner — no browser here)**

Run: `wasm-pack test --node crates/wasm 2>&1 | tail -20`
Expected: PASS (existing 4 + 2 new = 6). Note: the existing `roundtrip_standard` still uses `"standard"`, which remains valid.

- [ ] **Step 4: Commit**

```bash
git add crates/wasm/src/lib.rs crates/wasm/tests/web.rs
git commit -m "feat(wasm): reject unknown format, report unsupported version; test not_utf8 (#8 #10)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: CLI — explicit non-UTF-8 stdin error + graceful IO (#9 remaining)

**Files:**
- Modify: `crates/cli/src/main.rs`
- Test: `crates/cli/tests/cli.rs`

**Interfaces:**
- Consumes: existing CLI subcommands.
- Produces: `decrypt` now exits non-zero with a clear message on non-UTF-8 stdin instead of silently treating it as empty; `read_stdin`/stdout write no longer `panic`.

- [ ] **Step 1: Write the failing test**

In `crates/cli/tests/cli.rs`, add:

```rust
#[test]
fn decrypt_non_utf8_stdin_errors_cleanly() {
    Command::cargo_bin("osm")
        .unwrap()
        .args(["decrypt", "--passphrase", "pw"])
        .write_stdin(vec![0xff, 0xfe, 0x00])
        .assert()
        .failure()
        .stderr(predicates::str::contains("UTF-8"));
}
```

(`predicates` is a dev-dependency already used by the test crate.)

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p osm-cli decrypt_non_utf8_stdin_errors_cleanly 2>&1 | tail -15`
Expected: FAIL — current code does `unwrap_or_default()`, so it does not emit a UTF-8 error (it proceeds with an empty string and prints a generic decode error or different text).

- [ ] **Step 3: Make stdin reading fallible and the decrypt path explicit**

In `crates/cli/src/main.rs`, change `read_stdin` to return a `Result` and update callers. Replace the `read_stdin` fn:

```rust
fn read_stdin() -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::io::stdin().read_to_end(&mut buf)?;
    Ok(buf)
}
```

In the `Encrypt` arm, replace `let plaintext = read_stdin();` with:

```rust
            let plaintext = match read_stdin() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("failed to read stdin: {e}");
                    return ExitCode::FAILURE;
                }
            };
```

In the `Decrypt` arm, replace `let input = String::from_utf8(read_stdin()).unwrap_or_default();` with:

```rust
            let raw = match read_stdin() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("failed to read stdin: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let input = match String::from_utf8(raw) {
                Ok(s) => s,
                Err(_) => {
                    eprintln!("input is not valid UTF-8 text");
                    return ExitCode::FAILURE;
                }
            };
```

In the `Decrypt` success branch, replace `std::io::stdout().write_all(&plaintext).unwrap();` with:

```rust
                    if let Err(e) = std::io::stdout().write_all(&plaintext) {
                        eprintln!("failed to write output: {e}");
                        return ExitCode::FAILURE;
                    }
                    ExitCode::SUCCESS
```

- [ ] **Step 4: Run the CLI tests**

Run: `cargo test -p osm-cli 2>&1 | tail -15`
Expected: PASS (existing 3 + 1 new = 4), including the new non-UTF-8 test.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/main.rs crates/cli/tests/cli.rs
git commit -m "fix(cli): explicit error on non-UTF-8 stdin; graceful stdin/stdout IO (#9)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: web — errors.ts unit tests + client test isolation (#8 web units)

**Files:**
- Create: `web/src/lib/errors.test.ts`
- Modify: `web/src/crypto/client.test.ts`

**Interfaces:**
- Consumes: `decryptErrorMessage(kind, word?)` from `web/src/lib/errors.ts`.
- Produces: tests only.

- [ ] **Step 1: Write the errors.ts unit test**

Create `web/src/lib/errors.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { decryptErrorMessage } from "./errors";

describe("decryptErrorMessage", () => {
  it("maps each known kind to its Japanese message", () => {
    expect(decryptErrorMessage("auth_failed")).toMatch(/合言葉が違うか/);
    expect(decryptErrorMessage("malformed")).toMatch(/形式が正しくありません/);
    expect(decryptErrorMessage("unsupported_version")).toMatch(/新しい版/);
    expect(decryptErrorMessage("not_utf8")).toMatch(/読み取れませんでした/);
  });

  it("interpolates the offending word for invalid_word", () => {
    expect(decryptErrorMessage("invalid_word", "ぴよぴよ")).toContain("ぴよぴよ");
  });

  it("falls back to a generic message for unknown kinds", () => {
    expect(decryptErrorMessage("something-else")).toMatch(/予期しないエラー/);
  });
});
```

- [ ] **Step 2: Make the crypto client test isolate module state**

`web/src/crypto/client.ts` keeps the worker/`pending`/`nextId` as module-level singletons, so tests must reset modules between cases. In `web/src/crypto/client.test.ts`, add `vi.resetModules()` to `beforeEach` (keep the existing `vi.stubGlobal("Worker", FakeWorker)`):

```ts
beforeEach(() => {
  vi.resetModules();
  vi.stubGlobal("Worker", FakeWorker);
});
```

(The existing tests already `await import("./client")` inside each case, so a fresh module instance is picked up after `resetModules`.)

- [ ] **Step 3: Run the web tests**

Run: `cd web && pnpm test --run 2>&1 | grep -E "Test Files|Tests "`
Expected: PASS — previous count + the new `errors.test.ts` (3 tests), client tests still green.

- [ ] **Step 4: Commit**

```bash
git add web/src/lib/errors.test.ts web/src/crypto/client.test.ts
git commit -m "test(web): cover decryptErrorMessage; isolate client module state (#8)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: web a11y — tabpanel wiring + button types (#11)

**Files:**
- Modify: `web/src/App.tsx`
- Modify: `web/src/components/EncryptTab.tsx`
- Modify: `web/src/components/DecryptTab.tsx`
- Test: `web/src/App.test.tsx`

**Interfaces:**
- Consumes: existing tab UI.
- Produces: each tab `<button>` has `id`, `type="button"`, and `aria-controls`; each panel `<div>` has `role="tabpanel"`, `id`, `aria-labelledby`; action buttons get `type="button"`. No behavior change.

- [ ] **Step 1: Write the failing a11y test**

In `web/src/App.test.tsx`, add (keep existing tests):

```tsx
it("wires tabs to their panels with ARIA", () => {
  render(<App />);
  const encryptTab = screen.getByRole("tab", { name: "暗号化" });
  expect(encryptTab).toHaveAttribute("aria-controls", "panel-encrypt");
  expect(encryptTab).toHaveAttribute("type", "button");
  const panel = document.getElementById("panel-encrypt");
  expect(panel).toHaveAttribute("role", "tabpanel");
  expect(panel).toHaveAttribute("aria-labelledby", "tab-encrypt");
});
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cd web && pnpm test --run App 2>&1 | tail -20`
Expected: FAIL — current tabs have no `id`/`aria-controls`/`type`, panels have no `role`/`id`.

- [ ] **Step 3: Wire the tabs and panels in App.tsx**

In `web/src/App.tsx`, give the two tab buttons `id`, `type="button"`, and `aria-controls`. For the 暗号化 button add: `id="tab-encrypt" type="button" aria-controls="panel-encrypt"`. For the 復号 button add: `id="tab-decrypt" type="button" aria-controls="panel-decrypt"`. (Keep the existing `role`, `aria-selected`, `onClick`, `className`.)

Then update the two panel wrappers. Replace:

```tsx
              <div hidden={tab !== "encrypt"}>
                <EncryptTab />
              </div>
              {decryptMounted && (
                <div hidden={tab !== "decrypt"}>
                  <Suspense fallback={<div className="field-hint py-8 text-center">読み込み中...</div>}>
                    <DecryptTab />
                  </Suspense>
                </div>
              )}
```

with:

```tsx
              <div id="panel-encrypt" role="tabpanel" aria-labelledby="tab-encrypt" hidden={tab !== "encrypt"}>
                <EncryptTab />
              </div>
              {decryptMounted && (
                <div id="panel-decrypt" role="tabpanel" aria-labelledby="tab-decrypt" hidden={tab !== "decrypt"}>
                  <Suspense fallback={<div className="field-hint py-8 text-center">読み込み中...</div>}>
                    <DecryptTab />
                  </Suspense>
                </div>
              )}
```

- [ ] **Step 4: Add `type="button"` to action buttons**

In `web/src/components/EncryptTab.tsx` and `web/src/components/DecryptTab.tsx`, add `type="button"` to every `<button>` that lacks it (the encrypt/decrypt button, コピー, txt保存, 表示/隠す). Example — change `<button onClick={onEncrypt} disabled={!canEncrypt} className="button-primary sm:w-40">` to `<button type="button" onClick={onEncrypt} disabled={!canEncrypt} className="button-primary sm:w-40">`; apply the same to the others. (`PassphraseField`'s toggle already has `type="button"`.)

- [ ] **Step 5: Run the web tests**

Run: `cd web && pnpm test --run 2>&1 | grep -E "Test Files|Tests "` then `pnpm exec tsc --noEmit && echo tsc-ok`
Expected: all PASS (incl. the new a11y assertion), tsc clean.

- [ ] **Step 6: Commit**

```bash
git add web/src/App.tsx web/src/components/EncryptTab.tsx web/src/components/DecryptTab.tsx web/src/App.test.tsx
git commit -m "a11y(web): wire tablist/tabpanel ARIA and button types (#11)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: web — Playwright offline e2e (#8 e2e)

**Files:**
- Modify: `web/package.json` (add `@playwright/test` dev-dep + `test:e2e` script)
- Create: `web/playwright.config.ts`
- Create: `web/e2e/roundtrip.spec.ts`
- Modify: `web/.gitignore` (ignore `playwright-report`, `test-results`)

**Interfaces:**
- Consumes: the built app served by `pnpm preview`.
- Produces: an end-to-end round-trip test exercising the real WASM + Web Worker in a browser, plus an offline-reload check.

- [ ] **Step 1: Add Playwright and scripts**

In `web/package.json`, add to `devDependencies`: `"@playwright/test": "^1.47.0"`. Add to `scripts`: `"test:e2e": "playwright test"`. Append to `web/.gitignore`:

```
playwright-report
test-results
```

- [ ] **Step 2: Install Playwright + its Chromium**

Run:
```bash
cd web && pnpm install && pnpm exec playwright install chromium
```
Expected: Chromium downloaded. (CI uses `--with-deps`; locally in WSL plain `install chromium` is sufficient.)

- [ ] **Step 3: Create the Playwright config**

Create `web/playwright.config.ts`:

```ts
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  use: { baseURL: "http://localhost:4173" },
  webServer: {
    command: "pnpm build && pnpm preview --port 4173",
    url: "http://localhost:4173",
    timeout: 180_000,
    reuseExistingServer: !process.env.CI,
  },
});
```

- [ ] **Step 4: Write the e2e test**

Create `web/e2e/roundtrip.spec.ts`:

```ts
import { expect, test } from "@playwright/test";

test("encrypt then decrypt round-trips in a real browser", async ({ page }) => {
  await page.goto("/");

  // Encrypt
  await page.getByLabel("メモ本文").fill("playwright secret");
  await page.getByLabel("合言葉", { exact: true }).fill("pw-e2e");
  await page.getByLabel("合言葉（確認）").fill("pw-e2e");
  await page.getByRole("button", { name: "暗号化する" }).click();

  const ciphertext = await page.getByLabel("暗号化済みテキスト").inputValue();
  expect(ciphertext).toMatch(/^OSM1\./);

  // Decrypt
  await page.getByRole("tab", { name: "復号" }).click();
  await page.getByLabel("暗号化済みテキスト").fill(ciphertext);
  await page.getByLabel("合言葉", { exact: true }).fill("pw-e2e");
  await page.getByRole("button", { name: "復号する" }).click();

  await expect(page.getByText("playwright secret")).toBeVisible();
});

test("still works offline after first load (PWA precache)", async ({ page, context }) => {
  await page.goto("/");
  // Let the service worker install and cache assets.
  await page.waitForTimeout(2000);
  await context.setOffline(true);
  await page.reload();

  await page.getByLabel("メモ本文").fill("offline secret");
  await page.getByLabel("合言葉", { exact: true }).fill("pw-off");
  await page.getByLabel("合言葉（確認）").fill("pw-off");
  await page.getByRole("button", { name: "暗号化する" }).click();
  await expect(page.getByLabel("暗号化済みテキスト")).toHaveValue(/^OSM1\./);
});
```

> Note: the memo and passphrase field labels (`メモ本文`, `合言葉`, `合言葉（確認）`, `暗号化済みテキスト`) and button names (`暗号化する`, `復号する`) match the current components. If the offline test is flaky in CI due to service-worker timing, mark it `test.fixme` and keep the round-trip test as the gate; do not weaken the round-trip test.

- [ ] **Step 5: Run the e2e suite**

Run: `cd web && pnpm test:e2e 2>&1 | tail -25`
Expected: the round-trip test PASSES (it builds, serves, and drives a real browser). If the offline test cannot pass in this environment, convert it to `test.fixme` with a one-line reason and re-run so the suite is green.

- [ ] **Step 6: Add an e2e CI job**

In `.github/workflows/ci.yml`, add a third job (sibling to `rust` and `web`):

```yaml
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - uses: pnpm/action-setup@v4
        with:
          version: 11
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: web/pnpm-lock.yaml
      - name: Vendor wordlist
        run: |
          test -f crates/core/data/bip39-japanese.txt || \
            curl -fsSL https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/japanese.txt \
              -o crates/core/data/bip39-japanese.txt
      - name: Install deps + browser
        run: |
          cd web
          pnpm install --frozen-lockfile
          pnpm exec playwright install --with-deps chromium
      - name: Run e2e
        run: cd web && pnpm test:e2e
```

- [ ] **Step 7: Validate CI YAML + commit**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('valid yaml')"`
Expected: `valid yaml`.

```bash
git add web/package.json web/pnpm-lock.yaml web/playwright.config.ts web/e2e/roundtrip.spec.ts web/.gitignore .github/workflows/ci.yml
git commit -m "test(web): Playwright offline round-trip e2e + CI job (#8)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 7: CI/CD — gate deploy on CI, cleanups (#12)

**Files:**
- Modify: `.github/workflows/deploy.yml`
- Modify: `.github/workflows/ci.yml`
- Modify: `scripts/build-recovery-kit.sh`

**Interfaces:** None (CI/script). Deploy runs only after CI succeeds on `main`.

- [ ] **Step 1: Gate Deploy on CI success via `workflow_run`**

Replace the `on:` block at the top of `.github/workflows/deploy.yml`:

```yaml
on:
  workflow_run:
    workflows: ["CI"]
    types: [completed]
    branches: [main]
  workflow_dispatch:
```

Add a job-level guard so it only deploys when CI passed (skip on `workflow_dispatch`, where there is no `workflow_run` payload). Change the `deploy:` job header to:

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    if: ${{ github.event_name == 'workflow_dispatch' || github.event.workflow_run.conclusion == 'success' }}
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.event.workflow_run.head_sha || github.ref }}
```

(The `ref` ensures a `workflow_run`-triggered deploy checks out the exact commit CI validated. The rest of the job is unchanged.)

- [ ] **Step 2: Drop the redundant `--run` flag in CI**

In `.github/workflows/ci.yml`, in the web job, change `pnpm test --run` to `pnpm test` (the `test` script is already `vitest run --passWithNoTests`).

- [ ] **Step 3: Harden the recovery-kit script (EXIT trap + drop .d.ts)**

In `scripts/build-recovery-kit.sh`, add an EXIT trap immediately after `STAGE="$(mktemp -d)"`:

```bash
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
```

Remove the now-redundant `rm -rf "$STAGE"` on the final line. And after the `rm -f "$KIT/package.json" ...` line, also drop the TypeScript declaration files that ship by default:

```bash
rm -f "$KIT/package.json" "$KIT/.gitignore" "$KIT/README.md" "$KIT"/*.d.ts
```

- [ ] **Step 4: Verify YAML + script**

Run:
```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/deploy.yml')); yaml.safe_load(open('.github/workflows/ci.yml')); print('valid yaml')"
bash -n scripts/build-recovery-kit.sh && echo "script syntax ok"
```
Expected: `valid yaml` and `script syntax ok`. (If `wasm-pack`/`zip` are available, optionally run `./scripts/build-recovery-kit.sh` and confirm `unzip -l dist/open-secret-memo-recovery-kit.zip` shows no `.d.ts` entries.)

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/deploy.yml .github/workflows/ci.yml scripts/build-recovery-kit.sh
git commit -m "ci: gate deploy on CI success; drop redundant flag; harden kit script (#12)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 8: docs — sync design/requirements with the implementation (#13)

**Files:**
- Modify: `docs/要件定義.md`
- Modify: `docs/superpowers/specs/2026-06-17-open-secret-memo-design.md`

**Interfaces:** None (documentation). Reflect three already-shipped decisions.

- [ ] **Step 1: Append an "実装で確定した仕様" section to the design doc**

Add this section near the end of `docs/superpowers/specs/2026-06-17-open-secret-memo-design.md` (before the MVP scope section if present, otherwise at the end):

```markdown
## 実装で確定した仕様（後追い反映）

- 日本語単語列形式は、可逆性のため **4バイトのビッグエンディアン長さプレフィックス**（payloadバイト長）をビット列の先頭に付けてから11ビット単位でエンコードする。復号時はこの長さを読んで正確なバイト数を復元する。
- wasm 復号エラーの安定キー（UIが文言にマップ）: `malformed` / `unsupported_version` / `invalid_word` / `auth_failed` / `not_utf8`。
- 暗号処理（Argon2id）は UI を固めないよう **Web Worker** 上で実行する。
```

- [ ] **Step 2: Reflect the same three points in 要件定義.md**

In `docs/要件定義.md`, under the 暗号方式 / 技術構成 area, add a short bullet list:

```markdown
## 実装で確定した補足仕様

- 日本語単語列形式は4バイトBEの長さプレフィックス付き（可逆エンコード）。
- 暗号化・復号はWeb Workerで実行（メインスレッドを固めない）。
- 復号エラーの種別キー: malformed / unsupported_version / invalid_word / auth_failed / not_utf8。
```

- [ ] **Step 3: Commit**

```bash
git add docs/要件定義.md docs/superpowers/specs/2026-06-17-open-secret-memo-design.md
git commit -m "docs: sync design/requirements with shipped behavior (#13)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

**Issue coverage:**
- **#8** (tests): core edge cases → Task 1; wasm not_utf8 → Task 2; web errors.ts + client isolation → Task 4; Playwright e2e → Task 6. ✓
- **#9** (CLI): non-UTF-8 stdin + graceful IO → Task 3. (verify file-read + hex_decode already done — excluded, noted in Global Constraints.) ✓
- **#10** (core/wasm): wasm strict format + version diagnostics → Task 2; core doc comments → Task 1. (words.rs comment already done — excluded.) ✓
- **#11** (a11y): tablist/tabpanel ARIA + button types → Task 5. ✓
- **#12** (CI/CD): deploy gated on CI, `--run` cleanup, kit script EXIT trap + drop .d.ts → Task 7. ✓
- **#13** (docs): length prefix + error keys + Web Worker into both docs → Task 8. ✓

**Placeholder scan:** No TBD/TODO; each code step shows the exact code or exact attribute additions with surrounding context. The one conditional ("if offline e2e is flaky, mark `test.fixme`") is an explicit, bounded fallback, not a placeholder.

**Type/name consistency:** `decryptErrorMessage(kind, word?)`, `error_kind`/`error_word`, `encode_standard`/`decode_standard`/`encode_words`/`decode_words`, `FixedRng`/`Argon2Params`/`core_encrypt`, panel ids `panel-encrypt`/`panel-decrypt` and tab ids `tab-encrypt`/`tab-decrypt`, and the Japanese labels used in the Playwright selectors all match the current code read during planning.

**Independence:** Tasks 1–5, 7, 8 are independent and can be implemented/reviewed in any order. Task 6 (Playwright) adds a CI job; Task 7 also edits `ci.yml` (the `--run` line and is separate from the e2e job) — if both land, ensure the final `ci.yml` keeps the e2e job AND uses `pnpm test` (no `--run`). No other conflicts.
```
