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
  const [error, setError] = useState<string | null>(null);

  const mismatch = confirm.length > 0 && pass !== confirm;
  const canEncrypt = memo.length > 0 && pass.length > 0 && pass === confirm && !busy;
  const strength = pass.length > 0 ? estimateStrength(pass) : null;

  async function onEncrypt() {
    setBusy(true);
    setError(null);
    try {
      const ct = await encryptMemo({ plaintext: memo, passphrase: pass, mCost, tCost, pCost, format });
      setOutput(ct);
      if (clearAfter) { setMemo(""); setPass(""); setConfirm(""); }
    } catch {
      setError("暗号化に失敗しました。上級者オプションのArgon2パラメータが範囲内か確認してください。");
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
          <input type="number" aria-label="m_cost" value={mCost} min={8192} max={1048576} onChange={(e) => setMCost(Number(e.target.value))} className="text-slate-900 ml-2 rounded" />
        </label>
        <label className="block text-sm">反復回数
          <input type="number" aria-label="t_cost" value={tCost} min={1} onChange={(e) => setTCost(Number(e.target.value))} className="text-slate-900 ml-2 rounded" />
        </label>
        <label className="block text-sm">並列数
          <input type="number" aria-label="p_cost" value={pCost} min={1} onChange={(e) => setPCost(Number(e.target.value))} className="text-slate-900 ml-2 rounded" />
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
      {error && <p className="text-red-400 text-sm mt-1">{error}</p>}
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
                URL.revokeObjectURL(a.href);
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
