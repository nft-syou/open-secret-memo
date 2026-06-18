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
  vi.resetModules();
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
