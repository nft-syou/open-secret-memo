# Open Secret Memo — WASM Wrapper + Web/PWA Implementation Plan (Plan 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wrap `osm-core` with `wasm-bindgen`, and build the Vite + React + TypeScript + Tailwind PWA with Encrypt/Decrypt tabs that run crypto off the main thread in a Web Worker, fully offline-capable.

**Architecture:** A thin `osm-wasm` crate exposes `encrypt`/`decrypt` to JS via wasm-bindgen. `wasm-pack` emits an ES module into `web/src/wasm/`. The web app loads the wasm inside a Web Worker (so a 1–2s Argon2id never freezes the UI) and talks to it through a promise-based client. UI components are small and single-purpose. vite-plugin-pwa precaches the JS/CSS/WASM so encrypt/decrypt work offline.

**Tech Stack:** Rust + wasm-bindgen + wasm-pack; pnpm; Vite; React 18; TypeScript; Tailwind CSS; vite-plugin-pwa (Workbox); Vitest + @testing-library/react; wasm-bindgen-test.

**Prerequisite:** Plan 1 (`osm-core`) merged. `crates/wasm` is added to the workspace `members` in this plan.

## Global Constraints

- The `osm-wasm` crate MUST only re-expose `osm-core`; it contains no crypto logic of its own.
- All secret material (memo, passphrase, decrypted text) stays in-process. No network calls, no logging of secrets, no persistence. MVP persists nothing (no localStorage).
- Argon2id runs in a **Web Worker**, never on the main thread.
- Default Argon2 params surfaced in UI: m_cost 65536 KiB, t_cost 1, p_cost 1. Advanced options expose all three.
- Decrypt error kinds from wasm are stable strings: `malformed`, `unsupported_version`, `invalid_word`, `auth_failed`, `not_utf8`. The UI maps these to Japanese messages.
- Default save format = standard. Default "clear after encrypt" = OFF. Decrypt reveal/hide is manual only (no auto-hide).
- Build/runtime: Node 20+, pnpm 9+. wasm built with `wasm-pack build --target web`.

---

### Task 1: `osm-wasm` crate — wasm-bindgen wrappers

**Files:**
- Modify: `Cargo.toml` (add `crates/wasm` to members)
- Create: `crates/wasm/Cargo.toml`
- Create: `crates/wasm/src/lib.rs`
- Test: `crates/wasm/tests/web.rs`

**Interfaces:**
- Consumes: `osm_core` public API.
- Produces (JS-visible after wasm-pack):
  - `encrypt(plaintext: string, passphrase: string, m_cost: number, t_cost: number, p_cost: number, format: string): string` (throws on invalid params)
  - `decrypt(ciphertext: string, passphrase: string): DecryptOutcome`
  - `class DecryptOutcome { ok: boolean; text: string; error_kind: string; error_word: string }`

- [ ] **Step 1: Add crate to workspace and write manifest**

Edit root `Cargo.toml` members to:

```toml
members = ["crates/core", "crates/cli", "crates/wasm"]
```

Create `crates/wasm/Cargo.toml`:

```toml
[package]
name = "osm-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
osm-core = { path = "../core" }
wasm-bindgen = "0.2"
# osm-core's OsRng uses getrandom; the js backend is required on wasm32.
getrandom = { version = "0.2", features = ["js"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

- [ ] **Step 2: Write the failing wasm test**

Create `crates/wasm/tests/web.rs`:

```rust
use wasm_bindgen_test::*;
use osm_wasm::{decrypt, encrypt};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn roundtrip_standard() {
    // m_cost kept low for a fast test.
    let ct = encrypt("hello", "pw", 8192, 1, 1, "standard").unwrap();
    assert!(ct.starts_with("OSM1."));
    let outcome = decrypt(&ct, "pw");
    assert!(outcome.ok);
    assert_eq!(outcome.text, "hello");
}

#[wasm_bindgen_test]
fn wrong_passphrase_reports_auth_failed() {
    let ct = encrypt("hello", "right", 8192, 1, 1, "standard").unwrap();
    let outcome = decrypt(&ct, "wrong");
    assert!(!outcome.ok);
    assert_eq!(outcome.error_kind, "auth_failed");
}

#[wasm_bindgen_test]
fn malformed_input_reports_malformed() {
    let outcome = decrypt("not a ciphertext", "pw");
    assert!(!outcome.ok);
    assert_eq!(outcome.error_kind, "malformed");
}
```

- [ ] **Step 3: Implement the wrapper**

Create `crates/wasm/src/lib.rs`:

```rust
use wasm_bindgen::prelude::*;

use osm_core::{
    decrypt as core_decrypt, detect_and_decode, encode_standard, encode_words, encrypt as core_encrypt,
    Argon2Params, DecryptError, FormatError, OsRng,
};

/// Result of a decrypt attempt. `ok` distinguishes success from a recoverable
/// error; on failure `error_kind` is one of the stable kind strings.
#[wasm_bindgen(getter_with_clone)]
pub struct DecryptOutcome {
    pub ok: bool,
    pub text: String,
    pub error_kind: String,
    pub error_word: String,
}

#[wasm_bindgen]
pub fn encrypt(
    plaintext: &str,
    passphrase: &str,
    m_cost: u32,
    t_cost: u32,
    p_cost: u8,
    format: &str,
) -> Result<String, JsError> {
    let params = Argon2Params { m_cost, t_cost, p_cost };
    params.validate().map_err(|e| JsError::new(&e.to_string()))?;
    let mut rng = OsRng;
    let payload = core_encrypt(plaintext.as_bytes(), passphrase, params, &mut rng);
    Ok(match format {
        "words" => encode_words(&payload),
        _ => encode_standard(&payload),
    })
}

#[wasm_bindgen]
pub fn decrypt(ciphertext: &str, passphrase: &str) -> DecryptOutcome {
    let payload = match detect_and_decode(ciphertext) {
        Ok(p) => p,
        Err(e) => return from_format_error(e),
    };
    match core_decrypt(&payload, passphrase) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) => DecryptOutcome { ok: true, text, error_kind: String::new(), error_word: String::new() },
            Err(_) => err("not_utf8", ""),
        },
        Err(DecryptError::AuthenticationFailed) => err("auth_failed", ""),
        Err(DecryptError::Format(e)) => from_format_error(e),
    }
}

fn from_format_error(e: FormatError) -> DecryptOutcome {
    match e {
        FormatError::Malformed => err("malformed", ""),
        FormatError::UnsupportedVersion(_) => err("unsupported_version", ""),
        FormatError::InvalidWord(w) => err("invalid_word", &w),
    }
}

fn err(kind: &str, word: &str) -> DecryptOutcome {
    DecryptOutcome { ok: false, text: String::new(), error_kind: kind.to_string(), error_word: word.to_string() }
}
```

- [ ] **Step 4: Run the wasm tests**

Run: `wasm-pack test --headless --firefox crates/wasm`
(Alternatively `--node`.) Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/wasm/
git commit -m "feat(wasm): wasm-bindgen wrappers over osm-core"
```

---

### Task 2: Web app scaffold + wasm build wiring

**Files:**
- Create: `web/package.json`, `web/vite.config.ts`, `web/tsconfig.json`, `web/index.html`
- Create: `web/tailwind.config.js`, `web/postcss.config.js`, `web/src/index.css`
- Create: `web/src/main.tsx`, `web/src/App.tsx`
- Create: `web/.gitignore`
- Create: `web/scripts/build-wasm.sh`

**Interfaces:**
- Produces: a running `pnpm dev` app shell with two tab buttons (暗号化 / 復号) and an empty body. wasm output lands in `web/src/wasm/` (gitignored, regenerated by `build-wasm.sh`).

- [ ] **Step 1: Create the wasm build script**

Create `web/scripts/build-wasm.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
wasm-pack build crates/wasm --target web --out-dir "$(pwd)/web/src/wasm" --out-name osm
echo "wasm built to web/src/wasm"
```

Run:
```bash
chmod +x web/scripts/build-wasm.sh
./web/scripts/build-wasm.sh
```
Expected: `web/src/wasm/osm.js`, `web/src/wasm/osm_bg.wasm`, and `web/src/wasm/osm.d.ts` exist.

- [ ] **Step 2: Create package.json and tooling configs**

Create `web/package.json`:

```json
{
  "name": "open-secret-memo-web",
  "private": true,
  "type": "module",
  "scripts": {
    "build:wasm": "bash scripts/build-wasm.sh",
    "dev": "vite",
    "build": "pnpm build:wasm && tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.4.0",
    "@testing-library/react": "^16.0.0",
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "autoprefixer": "^10.4.0",
    "jsdom": "^24.0.0",
    "postcss": "^8.4.0",
    "tailwindcss": "^3.4.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0",
    "vite-plugin-pwa": "^0.20.0",
    "vitest": "^2.0.0"
  }
}
```

Create `web/tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable", "WebWorker"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "types": ["vitest/globals", "@testing-library/jest-dom"]
  },
  "include": ["src"]
}
```

Create `web/vite.config.ts`:

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";

export default defineConfig({
  base: "/",
  plugins: [
    react(),
    VitePWA({
      registerType: "autoUpdate",
      includeAssets: ["favicon.svg"],
      workbox: {
        // Precache the wasm so decrypt/encrypt work fully offline.
        globPatterns: ["**/*.{js,css,html,wasm,svg}"],
        maximumFileSizeToCacheInBytes: 5 * 1024 * 1024,
      },
      manifest: {
        name: "Open Secret Memo",
        short_name: "OSM",
        description: "ブラウザ内だけで秘密メモを暗号化・復号",
        theme_color: "#0f172a",
        background_color: "#0f172a",
        display: "standalone",
        start_url: "/",
        icons: [
          { src: "icon-192.png", sizes: "192x192", type: "image/png" },
          { src: "icon-512.png", sizes: "512x512", type: "image/png" }
        ]
      }
    })
  ],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"]
  }
});
```

Create `web/tailwind.config.js`:

```js
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: []
};
```

Create `web/postcss.config.js`:

```js
export default { plugins: { tailwindcss: {}, autoprefixer: {} } };
```

Create `web/src/index.css`:

```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

Create `web/.gitignore`:

```
node_modules
dist
src/wasm
dev-dist
```

Create `web/src/test-setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 3: Create the app shell**

Create `web/index.html`:

```html
<!doctype html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Open Secret Memo</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

Create `web/src/main.tsx`:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

Create `web/src/App.tsx`:

```tsx
import { useState } from "react";

type Tab = "encrypt" | "decrypt";

export default function App() {
  const [tab, setTab] = useState<Tab>("encrypt");
  return (
    <main className="mx-auto max-w-2xl p-4 text-slate-100">
      <h1 className="text-2xl font-bold mb-2">Open Secret Memo</h1>
      <p className="text-sm text-slate-300 mb-4">
        この内容はサーバーに送信されません。ブラウザ内だけで暗号化・復号します。
      </p>
      <div className="flex gap-2 mb-4" role="tablist">
        <button
          role="tab"
          aria-selected={tab === "encrypt"}
          onClick={() => setTab("encrypt")}
          className={tab === "encrypt" ? "font-bold underline" : ""}
        >
          暗号化
        </button>
        <button
          role="tab"
          aria-selected={tab === "decrypt"}
          onClick={() => setTab("decrypt")}
          className={tab === "decrypt" ? "font-bold underline" : ""}
        >
          復号
        </button>
      </div>
      <div>{tab === "encrypt" ? <p>encrypt</p> : <p>decrypt</p>}</div>
    </main>
  );
}
```

- [ ] **Step 4: Verify the app builds and runs**

Run:
```bash
cd web && pnpm install && pnpm build:wasm && pnpm test --run --reporter=basic; pnpm exec tsc --noEmit
```
Expected: `tsc` reports no type errors; wasm output present. (No component tests yet — `vitest run` with no tests exits 0.)

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): Vite+React+Tailwind scaffold with wasm build wiring"
```

---

### Task 3: Crypto Web Worker + promise client

**Files:**
- Create: `web/src/crypto/worker.ts`
- Create: `web/src/crypto/client.ts`
- Test: `web/src/crypto/client.test.ts`

**Interfaces:**
- Produces:
  - `encryptMemo(req: EncryptRequest): Promise<string>`
  - `decryptMemo(ciphertext: string, passphrase: string): Promise<DecryptResult>`
  - types `EncryptRequest { plaintext; passphrase; mCost; tCost; pCost; format: "standard" | "words" }`, `DecryptResult { ok: boolean; text?: string; errorKind?: string; errorWord?: string }`

- [ ] **Step 1: Write the worker**

Create `web/src/crypto/worker.ts`:

```ts
import init, { encrypt, decrypt } from "../wasm/osm";

let ready: Promise<unknown> | null = null;
function ensureReady() {
  if (!ready) ready = init();
  return ready;
}

type InMsg =
  | { id: number; kind: "encrypt"; plaintext: string; passphrase: string; mCost: number; tCost: number; pCost: number; format: string }
  | { id: number; kind: "decrypt"; ciphertext: string; passphrase: string };

self.onmessage = async (e: MessageEvent<InMsg>) => {
  const msg = e.data;
  await ensureReady();
  try {
    if (msg.kind === "encrypt") {
      const text = encrypt(msg.plaintext, msg.passphrase, msg.mCost, msg.tCost, msg.pCost, msg.format);
      self.postMessage({ id: msg.id, ok: true, text });
    } else {
      const outcome = decrypt(msg.ciphertext, msg.passphrase);
      self.postMessage({
        id: msg.id,
        ok: outcome.ok,
        text: outcome.text,
        errorKind: outcome.error_kind,
        errorWord: outcome.error_word
      });
    }
  } catch (err) {
    self.postMessage({ id: msg.id, ok: false, errorKind: "exception", text: String(err) });
  }
};
```

- [ ] **Step 2: Write the failing client test**

Create `web/src/crypto/client.test.ts`:

```ts
import { describe, expect, it, vi, beforeEach } from "vitest";

// The worker is mocked: encrypt echoes a fake ciphertext, decrypt validates passphrase.
const handlers: Record<string, (msg: any) => any> = {
  encrypt: (m) => ({ ok: true, text: "OSM1.FAKE" }),
  decrypt: (m) =>
    m.passphrase === "right"
      ? { ok: true, text: "secret", errorKind: "", errorWord: "" }
      : { ok: false, text: "", errorKind: "auth_failed", errorWord: "" }
};

class FakeWorker {
  onmessage: ((e: MessageEvent) => void) | null = null;
  postMessage(msg: any) {
    const res = handlers[msg.kind](msg);
    queueMicrotask(() => this.onmessage?.({ data: { id: msg.id, ...res } } as MessageEvent));
  }
  terminate() {}
}

beforeEach(() => {
  vi.stubGlobal("Worker", FakeWorker);
});

describe("crypto client", () => {
  it("encryptMemo resolves to ciphertext", async () => {
    const { encryptMemo } = await import("./client");
    const ct = await encryptMemo({
      plaintext: "secret", passphrase: "x", mCost: 8192, tCost: 1, pCost: 1, format: "standard"
    });
    expect(ct).toBe("OSM1.FAKE");
  });

  it("decryptMemo returns ok on right passphrase", async () => {
    const { decryptMemo } = await import("./client");
    const r = await decryptMemo("OSM1.FAKE", "right");
    expect(r).toEqual({ ok: true, text: "secret", errorKind: "", errorWord: "" });
  });

  it("decryptMemo returns auth_failed on wrong passphrase", async () => {
    const { decryptMemo } = await import("./client");
    const r = await decryptMemo("OSM1.FAKE", "wrong");
    expect(r.ok).toBe(false);
    expect(r.errorKind).toBe("auth_failed");
  });
});
```

- [ ] **Step 3: Write the client**

Create `web/src/crypto/client.ts`:

```ts
export type SaveFormat = "standard" | "words";

export interface EncryptRequest {
  plaintext: string;
  passphrase: string;
  mCost: number;
  tCost: number;
  pCost: number;
  format: SaveFormat;
}

export interface DecryptResult {
  ok: boolean;
  text?: string;
  errorKind?: string;
  errorWord?: string;
}

let worker: Worker | null = null;
let nextId = 1;
const pending = new Map<number, (data: any) => void>();

function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (e: MessageEvent) => {
      const { id, ...rest } = e.data;
      const resolve = pending.get(id);
      if (resolve) {
        pending.delete(id);
        resolve(rest);
      }
    };
  }
  return worker;
}

function call(message: object): Promise<any> {
  const id = nextId++;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    getWorker().postMessage({ id, ...message });
  });
}

export async function encryptMemo(req: EncryptRequest): Promise<string> {
  const res = await call({ kind: "encrypt", ...req });
  if (!res.ok) throw new Error(res.text ?? "encryption failed");
  return res.text as string;
}

export async function decryptMemo(ciphertext: string, passphrase: string): Promise<DecryptResult> {
  const res = await call({ kind: "decrypt", ciphertext, passphrase });
  return { ok: res.ok, text: res.text, errorKind: res.errorKind, errorWord: res.errorWord };
}
```

- [ ] **Step 4: Run the client tests**

Run: `cd web && pnpm test --run`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/crypto/
git commit -m "feat(web): crypto Web Worker and promise client"
```

---

### Task 4: Passphrase strength estimation + error messages

**Files:**
- Create: `web/src/lib/strength.ts`
- Create: `web/src/lib/errors.ts`
- Test: `web/src/lib/strength.test.ts`

**Interfaces:**
- Produces:
  - `estimateStrength(passphrase: string): { level: "weak" | "ok" | "strong"; message: string }`
  - `decryptErrorMessage(kind: string, word?: string): string`

- [ ] **Step 1: Write the failing strength test**

Create `web/src/lib/strength.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { estimateStrength } from "./strength";

describe("estimateStrength", () => {
  it("flags short passphrases as weak", () => {
    expect(estimateStrength("abc").level).toBe("weak");
  });

  it("treats 4-5 comma-separated words as ok", () => {
    expect(estimateStrength("紙袋、みかん、夜道、ラジオ").level).toBe("ok");
  });

  it("treats 6+ comma-separated words as strong", () => {
    expect(estimateStrength("紙袋、みかん、夜道、ラジオ、階段、ペンギン").level).toBe("strong");
  });

  it("treats a long sentence as strong", () => {
    expect(estimateStrength("紙袋を持ったカエルが夜の図書館でカレーの作り方を読んでいた").level).toBe("strong");
  });
});
```

- [ ] **Step 2: Implement strength + error messages**

Create `web/src/lib/strength.ts`:

```ts
export interface Strength {
  level: "weak" | "ok" | "strong";
  message: string;
}

/**
 * Heuristic only — never blocks encryption (warning UI). Considers both the
 * number of 「、」-separated tokens (word-style) and total length (sentence-style).
 */
export function estimateStrength(passphrase: string): Strength {
  const p = passphrase.trim();
  const wordCount = p.split("、").map((w) => w.trim()).filter(Boolean).length;
  const len = [...p].length;

  if (wordCount >= 6 || len >= 24) {
    return { level: "strong", message: "強い合言葉です。" };
  }
  if (wordCount >= 4 || len >= 12) {
    return { level: "ok", message: "まずまずです。単語を6個以上にするとより安全です。" };
  }
  return {
    level: "weak",
    message: "弱い合言葉です。無関係な単語を「、」で6個以上つなげる方法がおすすめです。"
  };
}
```

Create `web/src/lib/errors.ts`:

```ts
/** Maps a wasm decrypt error kind to a Japanese user-facing message. */
export function decryptErrorMessage(kind: string, word?: string): string {
  switch (kind) {
    case "auth_failed":
      return "復号できませんでした。合言葉が違うか、暗号文が壊れている可能性があります。";
    case "malformed":
      return "暗号文の形式が正しくありません。全体をコピーできているか確認してください。";
    case "unsupported_version":
      return "この暗号文は新しい版で作られています。最新版アプリで復号してください。";
    case "invalid_word":
      return `「${word ?? ""}」は単語リストにありません。写し間違いの可能性があります。`;
    case "not_utf8":
      return "復号できましたが、テキストとして読み取れませんでした。";
    default:
      return "復号中に予期しないエラーが発生しました。";
  }
}
```

- [ ] **Step 3: Run the strength tests**

Run: `cd web && pnpm test --run strength`
Expected: PASS (4 tests).

- [ ] **Step 4: Commit**

```bash
git add web/src/lib/
git commit -m "feat(web): passphrase strength heuristic and error message map"
```

---

### Task 5: PassphraseField + Encrypt tab

**Files:**
- Create: `web/src/components/PassphraseField.tsx`
- Create: `web/src/components/EncryptTab.tsx`
- Modify: `web/src/App.tsx`
- Test: `web/src/components/EncryptTab.test.tsx`

**Interfaces:**
- Consumes: `encryptMemo`, `estimateStrength`, `SaveFormat`.
- Produces: `<EncryptTab />`, `<PassphraseField label value onChange />` (masked with show/hide toggle).

- [ ] **Step 1: Write the failing component test**

Create `web/src/components/EncryptTab.test.tsx`:

```tsx
import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import EncryptTab from "./EncryptTab";

vi.mock("../crypto/client", () => ({
  encryptMemo: vi.fn(async () => "OSM1.RESULT")
}));

beforeEach(() => vi.clearAllMocks());

describe("EncryptTab", () => {
  it("disables encrypt button until memo, passphrase, and matching confirm are present", async () => {
    render(<EncryptTab />);
    const button = screen.getByRole("button", { name: "暗号化する" });
    expect(button).toBeDisabled();

    fireEvent.change(screen.getByLabelText("メモ本文"), { target: { value: "secret" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "pass" } });
    fireEvent.change(screen.getByLabelText("合言葉（確認）"), { target: { value: "different" } });
    expect(button).toBeDisabled();
    expect(screen.getByText("合言葉が一致しません。")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("合言葉（確認）"), { target: { value: "pass" } });
    expect(button).toBeEnabled();
  });

  it("shows the ciphertext after encrypting", async () => {
    render(<EncryptTab />);
    fireEvent.change(screen.getByLabelText("メモ本文"), { target: { value: "secret" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "pass" } });
    fireEvent.change(screen.getByLabelText("合言葉（確認）"), { target: { value: "pass" } });
    fireEvent.click(screen.getByRole("button", { name: "暗号化する" }));
    await waitFor(() => expect(screen.getByDisplayValue("OSM1.RESULT")).toBeInTheDocument());
  });
});
```

- [ ] **Step 2: Implement PassphraseField**

Create `web/src/components/PassphraseField.tsx`:

```tsx
import { useState } from "react";

interface Props {
  label: string;
  value: string;
  onChange: (v: string) => void;
}

export default function PassphraseField({ label, value, onChange }: Props) {
  const [show, setShow] = useState(false);
  return (
    <label className="block mb-2">
      <span className="block text-sm">{label}</span>
      <span className="flex gap-2">
        <input
          aria-label={label}
          type={show ? "text" : "password"}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 border rounded px-2 py-1 text-slate-900"
        />
        <button type="button" onClick={() => setShow((s) => !s)}>
          {show ? "隠す" : "表示"}
        </button>
      </span>
    </label>
  );
}
```

- [ ] **Step 3: Implement EncryptTab**

Create `web/src/components/EncryptTab.tsx`:

```tsx
import { useState } from "react";
import PassphraseField from "./PassphraseField";
import { encryptMemo, type SaveFormat } from "../crypto/client";
import { estimateStrength } from "../lib/strength";

export default function EncryptTab() {
  const [memo, setMemo] = useState("");
  const [pass, setPass] = useState("");
  const [confirm, setConfirm] = useState("");
  const [format, setFormat] = useState<SaveFormat>("standard");
  const [mCost, setMCost] = useState(65536);
  const [tCost, setTCost] = useState(1);
  const [pCost, setPCost] = useState(1);
  const [clearAfter, setClearAfter] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [output, setOutput] = useState("");
  const [busy, setBusy] = useState(false);

  const mismatch = confirm.length > 0 && pass !== confirm;
  const canEncrypt = memo.length > 0 && pass.length > 0 && pass === confirm && !busy;
  const strength = pass.length > 0 ? estimateStrength(pass) : null;

  async function onEncrypt() {
    setBusy(true);
    try {
      const ct = await encryptMemo({ plaintext: memo, passphrase: pass, mCost, tCost, pCost, format });
      setOutput(ct);
      if (clearAfter) {
        setMemo("");
        setPass("");
        setConfirm("");
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <section>
      <label className="block mb-2">
        <span className="block text-sm">メモ本文</span>
        <textarea
          aria-label="メモ本文"
          value={memo}
          onChange={(e) => setMemo(e.target.value)}
          className="w-full border rounded px-2 py-1 text-slate-900"
          rows={4}
        />
      </label>

      <PassphraseField label="合言葉" value={pass} onChange={setPass} />
      <PassphraseField label="合言葉（確認）" value={confirm} onChange={setConfirm} />
      {mismatch && <p className="text-red-400 text-sm">合言葉が一致しません。</p>}
      {strength && (
        <p className={strength.level === "weak" ? "text-amber-400 text-sm" : "text-slate-300 text-sm"}>
          {strength.message}
        </p>
      )}

      <label className="block my-2">
        <span className="block text-sm">保存形式</span>
        <select
          aria-label="保存形式"
          value={format}
          onChange={(e) => setFormat(e.target.value as SaveFormat)}
          className="text-slate-900 rounded"
        >
          <option value="standard">標準形式</option>
          <option value="words">日本語単語列形式</option>
        </select>
      </label>

      <details open={showAdvanced} onToggle={(e) => setShowAdvanced((e.target as HTMLDetailsElement).open)}>
        <summary>上級者オプション（Argon2id）</summary>
        <label className="block text-sm">メモリ(KiB)
          <input type="number" aria-label="m_cost" value={mCost} onChange={(e) => setMCost(Number(e.target.value))} className="text-slate-900 ml-2 rounded" />
        </label>
        <label className="block text-sm">反復回数
          <input type="number" aria-label="t_cost" value={tCost} onChange={(e) => setTCost(Number(e.target.value))} className="text-slate-900 ml-2 rounded" />
        </label>
        <label className="block text-sm">並列数
          <input type="number" aria-label="p_cost" value={pCost} onChange={(e) => setPCost(Number(e.target.value))} className="text-slate-900 ml-2 rounded" />
        </label>
      </details>

      <label className="block my-2 text-sm">
        <input type="checkbox" checked={clearAfter} onChange={(e) => setClearAfter(e.target.checked)} className="mr-2" />
        暗号化後にメモ本文・合言葉をクリアする
      </label>

      <button
        onClick={onEncrypt}
        disabled={!canEncrypt}
        className="bg-sky-600 disabled:opacity-40 rounded px-3 py-1"
      >
        暗号化する
      </button>
      <p className="text-xs text-slate-400 mt-1">合言葉を忘れると復号できません。この内容はサーバーに送信されません。</p>

      {output && (
        <div className="mt-3">
          <textarea aria-label="暗号化済みテキスト" readOnly value={output} className="w-full border rounded px-2 py-1 text-slate-900" rows={4} />
          <div className="flex gap-2 mt-1">
            <button onClick={() => navigator.clipboard.writeText(output)}>コピー</button>
            <button
              onClick={() => {
                const blob = new Blob([output], { type: "text/plain" });
                const a = document.createElement("a");
                a.href = URL.createObjectURL(blob);
                a.download = "secret.osm.txt";
                a.click();
              }}
            >
              txt保存
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
```

- [ ] **Step 4: Wire into App**

Edit `web/src/App.tsx`: replace the `import { useState }` line region to import the tab and render it. Change the body line:

```tsx
import EncryptTab from "./components/EncryptTab";
```
and replace `{tab === "encrypt" ? <p>encrypt</p> : <p>decrypt</p>}` with:

```tsx
{tab === "encrypt" ? <EncryptTab /> : <p>decrypt</p>}
```

- [ ] **Step 5: Run the component tests**

Run: `cd web && pnpm test --run EncryptTab`
Expected: PASS (2 tests).

- [ ] **Step 6: Commit**

```bash
git add web/src/components/PassphraseField.tsx web/src/components/EncryptTab.tsx web/src/App.tsx web/src/components/EncryptTab.test.tsx
git commit -m "feat(web): encrypt tab with strength warning and advanced options"
```

---

### Task 6: Decrypt tab

**Files:**
- Create: `web/src/components/DecryptTab.tsx`
- Modify: `web/src/App.tsx`
- Test: `web/src/components/DecryptTab.test.tsx`

**Interfaces:**
- Consumes: `decryptMemo`, `decryptErrorMessage`.
- Produces: `<DecryptTab />` with manual reveal/hide (no auto-hide).

- [ ] **Step 1: Write the failing component test**

Create `web/src/components/DecryptTab.test.tsx`:

```tsx
import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import DecryptTab from "./DecryptTab";
import * as client from "../crypto/client";

vi.mock("../crypto/client", () => ({ decryptMemo: vi.fn() }));

beforeEach(() => vi.clearAllMocks());

describe("DecryptTab", () => {
  it("shows decrypted text on success", async () => {
    (client.decryptMemo as any).mockResolvedValue({ ok: true, text: "my secret" });
    render(<DecryptTab />);
    fireEvent.change(screen.getByLabelText("暗号化済みテキスト"), { target: { value: "OSM1.X" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "pw" } });
    fireEvent.click(screen.getByRole("button", { name: "復号する" }));
    await waitFor(() => expect(screen.getByText("my secret")).toBeInTheDocument());
  });

  it("shows the mapped error message on auth failure", async () => {
    (client.decryptMemo as any).mockResolvedValue({ ok: false, errorKind: "auth_failed" });
    render(<DecryptTab />);
    fireEvent.change(screen.getByLabelText("暗号化済みテキスト"), { target: { value: "OSM1.X" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "wrong" } });
    fireEvent.click(screen.getByRole("button", { name: "復号する" }));
    await waitFor(() =>
      expect(screen.getByText(/合言葉が違うか/)).toBeInTheDocument()
    );
  });
});
```

- [ ] **Step 2: Implement DecryptTab**

Create `web/src/components/DecryptTab.tsx`:

```tsx
import { useState } from "react";
import PassphraseField from "./PassphraseField";
import { decryptMemo } from "../crypto/client";
import { decryptErrorMessage } from "../lib/errors";

export default function DecryptTab() {
  const [ciphertext, setCiphertext] = useState("");
  const [pass, setPass] = useState("");
  const [plain, setPlain] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hidden, setHidden] = useState(false);
  const [busy, setBusy] = useState(false);

  const canDecrypt = ciphertext.trim().length > 0 && pass.length > 0 && !busy;

  async function onDecrypt() {
    setBusy(true);
    setError(null);
    setPlain(null);
    try {
      const r = await decryptMemo(ciphertext, pass);
      if (r.ok) {
        setPlain(r.text ?? "");
        setHidden(false);
      } else {
        setError(decryptErrorMessage(r.errorKind ?? "", r.errorWord));
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <section>
      <label className="block mb-2">
        <span className="block text-sm">暗号化済みテキスト</span>
        <textarea
          aria-label="暗号化済みテキスト"
          value={ciphertext}
          onChange={(e) => setCiphertext(e.target.value)}
          className="w-full border rounded px-2 py-1 text-slate-900"
          rows={4}
        />
      </label>
      <PassphraseField label="合言葉" value={pass} onChange={setPass} />
      <button onClick={onDecrypt} disabled={!canDecrypt} className="bg-sky-600 disabled:opacity-40 rounded px-3 py-1">
        復号する
      </button>

      {error && <p className="text-red-400 text-sm mt-2">{error}</p>}

      {plain !== null && (
        <div className="mt-3">
          <div className="border rounded px-2 py-1 bg-slate-800 min-h-[3rem] whitespace-pre-wrap">
            {hidden ? "••••••••" : plain}
          </div>
          <div className="flex gap-2 mt-1">
            <button onClick={() => navigator.clipboard.writeText(plain)}>コピー</button>
            <button onClick={() => setHidden((h) => !h)}>{hidden ? "表示" : "隠す"}</button>
          </div>
        </div>
      )}
    </section>
  );
}
```

- [ ] **Step 3: Wire into App**

Edit `web/src/App.tsx`: add `import DecryptTab from "./components/DecryptTab";` and replace `<p>decrypt</p>` with `<DecryptTab />`.

- [ ] **Step 4: Run the component tests**

Run: `cd web && pnpm test --run DecryptTab`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add web/src/components/DecryptTab.tsx web/src/App.tsx web/src/components/DecryptTab.test.tsx
git commit -m "feat(web): decrypt tab with manual reveal and mapped errors"
```

---

### Task 7: "このサイトは安全？" accordion + passphrase guide

**Files:**
- Create: `web/src/components/SafetyAccordion.tsx`
- Create: `web/src/lib/appInfo.ts`
- Modify: `web/src/App.tsx`
- Test: `web/src/components/SafetyAccordion.test.tsx`

**Interfaces:**
- Consumes: `APP_VERSION`, `IPFS_CID`, `GITHUB_URL`, `SPEC_URL` from `appInfo.ts`.
- Produces: `<SafetyAccordion />`.

- [ ] **Step 1: Write app info constants**

Create `web/src/lib/appInfo.ts`:

```ts
// Build/version metadata surfaced in the "is this safe?" section.
export const APP_VERSION = "0.1.0";
export const GITHUB_URL = "https://github.com/<owner>/open-secret-memo";
export const SPEC_URL = "https://github.com/<owner>/open-secret-memo/blob/main/spec/SPEC.md";
// Set after the first IPFS pin (MVP後); empty means "not yet mirrored".
export const IPFS_CID = "";
export const CRYPTO_SUMMARY = "Argon2id + AES-256-GCM";
```

> The `<owner>` placeholders are replaced once the GitHub repo exists (Plan 3 deploy task). They are intentionally explicit, not silent TODOs.

- [ ] **Step 2: Write the failing test**

Create `web/src/components/SafetyAccordion.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import SafetyAccordion from "./SafetyAccordion";

describe("SafetyAccordion", () => {
  it("reveals the safety details when expanded", () => {
    render(<SafetyAccordion />);
    fireEvent.click(screen.getByText("このサイトは安全？"));
    expect(screen.getByText(/サーバーに送信しません/)).toBeInTheDocument();
    expect(screen.getByText(/Argon2id \+ AES-256-GCM/)).toBeInTheDocument();
  });
});
```

- [ ] **Step 3: Implement the accordion**

Create `web/src/components/SafetyAccordion.tsx`:

```tsx
import { APP_VERSION, CRYPTO_SUMMARY, GITHUB_URL, IPFS_CID, SPEC_URL } from "../lib/appInfo";

export default function SafetyAccordion() {
  return (
    <details className="mt-6 border-t pt-3">
      <summary className="cursor-pointer font-semibold">このサイトは安全？</summary>
      <div className="text-sm text-slate-300 mt-2 space-y-1">
        <p>合言葉・メモ本文・復号結果をサーバーに送信しません。</p>
        <p>合言葉・平文・復号結果を保存しません（このMVPでは暗号文も保存しません）。</p>
        <p>通信なし・オフラインでも使えます（PWA）。</p>
        <p>暗号方式: {CRYPTO_SUMMARY}</p>
        <p>バージョン: {APP_VERSION}</p>
        <p>ソースコード: <a className="underline" href={GITHUB_URL}>GitHub</a></p>
        <p>仕様書: <a className="underline" href={SPEC_URL}>SPEC.md</a></p>
        <p>IPFS CID: {IPFS_CID || "（ミラー準備中）"}</p>
        <hr className="my-2 border-slate-700" />
        <p className="font-semibold">合言葉のヒント</p>
        <p>日本語も使えます。無関係な単語を「、」で6個以上つなげるのがおすすめです。</p>
        <p>文章型の合言葉も使えます。合言葉を忘れると復号できません。このサイトにも保存されません。</p>
      </div>
    </details>
  );
}
```

- [ ] **Step 4: Wire into App**

Edit `web/src/App.tsx`: add `import SafetyAccordion from "./components/SafetyAccordion";` and render `<SafetyAccordion />` after the tab body `<div>`.

- [ ] **Step 5: Run the test**

Run: `cd web && pnpm test --run SafetyAccordion`
Expected: PASS (1 test).

- [ ] **Step 6: Commit**

```bash
git add web/src/components/SafetyAccordion.tsx web/src/lib/appInfo.ts web/src/App.tsx web/src/components/SafetyAccordion.test.tsx
git commit -m "feat(web): safety accordion and passphrase guidance"
```

---

### Task 8: PWA assets + offline verification

**Files:**
- Create: `web/public/favicon.svg`, `web/public/icon-192.png`, `web/public/icon-512.png`
- Verify: production build registers a service worker and precaches the wasm.

**Interfaces:** None (assets + build verification).

- [ ] **Step 1: Add PWA icons**

Create a simple `web/public/favicon.svg` (a lock glyph) and generate the two PNG icons:

```bash
cat > web/public/favicon.svg <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#0ea5e9">
  <path d="M12 1a5 5 0 0 0-5 5v3H5v13h14V9h-2V6a5 5 0 0 0-5-5zm-3 8V6a3 3 0 1 1 6 0v3H9z"/>
</svg>
SVG
```

Generate PNGs from the SVG (requires `rsvg-convert` or `inkscape`; if unavailable, hand-create 192/512 PNG placeholders):

```bash
rsvg-convert -w 192 -h 192 web/public/favicon.svg -o web/public/icon-192.png
rsvg-convert -w 512 -h 512 web/public/favicon.svg -o web/public/icon-512.png
```
Expected: both PNGs exist.

- [ ] **Step 2: Build and confirm the service worker + wasm precache**

Run:
```bash
cd web && pnpm build
ls dist/sw.js dist/assets/*.wasm
grep -o 'osm_bg[^"]*\.wasm' dist/sw.js | head
```
Expected: `dist/sw.js` exists, a hashed `.wasm` is emitted under `dist/assets/`, and the wasm filename appears in the precache manifest inside `sw.js`.

- [ ] **Step 3: Manually verify offline operation**

Run:
```bash
cd web && pnpm preview --port 4173
```
Then in a browser: load `http://localhost:4173`, encrypt a memo, open DevTools → Application → Service Workers, check "Offline", reload, and confirm encrypt + decrypt still work. (This is a manual smoke check; automated e2e is deferred to post-MVP per the spec.)

- [ ] **Step 4: Commit**

```bash
git add web/public/
git commit -m "feat(web): PWA icons and offline precache verification"
```

---

## Self-Review

**Spec coverage (design doc §6, §7, §8 + UI bits of §4):**
- Tabs (暗号化/復号) + top description → Task 2, App ✓
- Encrypt: memo, passphrase+confirm (masked, toggle), mismatch + strength warning (warn-only), save format default standard, advanced Argon2 pulldown, clear-after default OFF, output + copy + txt save, notice text → Task 5 ✓
- Decrypt: input (auto-detect via core), passphrase, output + copy + manual reveal/hide (no auto-hide), mapped error messages → Task 6 ✓
- Safety accordion (sends/stores nothing, offline, source, spec, IPFS CID, version, crypto) + passphrase guide → Task 7 ✓
- PWA: manifest + SW precaching JS/CSS/WASM, offline encrypt/decrypt; no push/bg-sync → Tasks 2, 8 ✓
- Crypto off main thread (responsiveness/実用性) → Task 3 ✓
- No persistence (no localStorage) → enforced by absence; Global Constraints ✓
- Error kinds map to taxonomy from design §4 → Task 1 (wasm) + Task 4 (messages) ✓

**Placeholder scan:** The only placeholders are the explicit `<owner>` repo-URL markers in `appInfo.ts` and `IPFS_CID = ""`, both called out and resolved in Plan 3's deploy task — not silent TODOs. All code steps are complete. ✓

**Type consistency:** `EncryptRequest`/`DecryptResult`/`SaveFormat` (client) used identically in EncryptTab/DecryptTab; wasm `DecryptOutcome` field names (`ok`,`text`,`error_kind`,`error_word`) consumed consistently by worker → client. ✓
