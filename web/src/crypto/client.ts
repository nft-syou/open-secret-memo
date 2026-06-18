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
