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
    <section className="space-y-5">
      <div>
        <h2 className="text-xl font-bold text-stone-950">メモを復号</h2>
        <p className="mt-1 text-sm leading-6 text-stone-600">
          保存しておいた暗号化済みテキストと合言葉から、元のメモをこの画面内で復元します。
        </p>
      </div>

      <label className="block">
        <span className="field-label">暗号化済みテキスト</span>
        <textarea
          aria-label="暗号化済みテキスト"
          value={ciphertext}
          onChange={(e) => setCiphertext(e.target.value)}
          className="input-surface mt-1.5 min-h-36 font-mono text-sm"
          rows={6}
          placeholder="OSM1..."
        />
      </label>

      <PassphraseField label="合言葉" value={pass} onChange={setPass} />

      <div className="flex flex-col gap-3 border-t border-stone-200 pt-4 sm:flex-row sm:items-center">
        <button onClick={onDecrypt} disabled={!canDecrypt} className="button-primary sm:w-40">
          {busy ? "処理中..." : "復号する"}
        </button>
        <p className="text-xs leading-5 text-stone-500">
          復号結果は保存されません。必要なときだけ表示して、使い終わったら隠せます。
        </p>
      </div>

      {error && <p className="rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>}

      {plain !== null && (
        <div className="section-panel">
          <div className="mb-2 flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h3 className="font-bold text-stone-950">復号結果</h3>
              <p className="field-hint">必要に応じてコピーできます。</p>
            </div>
            <div className="flex gap-2">
              <button className="button-secondary" onClick={() => navigator.clipboard.writeText(plain)}>コピー</button>
              <button className="button-secondary" onClick={() => setHidden((h) => !h)}>{hidden ? "表示" : "隠す"}</button>
            </div>
          </div>
          <div className="min-h-24 whitespace-pre-wrap rounded border border-stone-300 bg-stone-950 px-3 py-3 text-sm leading-6 text-stone-50 shadow-inner">
            {hidden ? "••••••••" : plain}
          </div>
        </div>
      )}
    </section>
  );
}
