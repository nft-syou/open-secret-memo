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
