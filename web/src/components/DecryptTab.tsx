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
