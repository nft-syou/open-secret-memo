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
    <section className="space-y-5">
      <div>
        <h2 className="text-xl font-bold text-stone-950">メモを暗号化</h2>
        <p className="mt-1 text-sm leading-6 text-stone-600">
          暗号化済みテキストをクラウドメモやチャットなどに保存できます。
        </p>
      </div>

      <label className="block">
        <span className="field-label">メモ本文</span>
        <textarea
          aria-label="メモ本文"
          value={memo}
          onChange={(e) => setMemo(e.target.value)}
          className="input-surface mt-1.5 min-h-36"
          rows={6}
          placeholder="ここに秘密メモを入力"
        />
      </label>

      <div className="grid gap-4 sm:grid-cols-2">
        <PassphraseField label="合言葉" value={pass} onChange={setPass} />
        <PassphraseField label="合言葉（確認）" value={confirm} onChange={setConfirm} />
      </div>
      <div className="min-h-5">
        {mismatch && <p className="text-sm font-medium text-red-700">合言葉が一致しません。</p>}
        {strength && (
          <p className={strength.level === "weak" ? "text-sm font-medium text-amber-700" : "text-sm text-stone-600"}>
            {strength.message}
          </p>
        )}
      </div>

      <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
        <label className="block">
          <span className="field-label">保存形式</span>
          <select
            aria-label="保存形式"
            value={format}
            onChange={(e) => setFormat(e.target.value as SaveFormat)}
            className="input-surface mt-1.5"
          >
            <option value="standard">標準形式</option>
            <option value="words">日本語単語列形式</option>
            <option value="kanji">漢字混じり（実験）</option>
          </select>
          {format === "kanji" && (
            <p className="field-hint mt-1 text-amber-700">
              実験的な形式です。長期保存には標準形式を推奨します。
            </p>
          )}
        </label>

        <label className="flex min-h-[4.75rem] items-center gap-3 rounded border border-stone-200 bg-stone-50 px-3 py-3 text-sm text-stone-700">
          <input
            type="checkbox"
            checked={clearAfter}
            onChange={(e) => setClearAfter(e.target.checked)}
            className="h-4 w-4 rounded border-stone-300 text-teal-700"
          />
          <span>暗号化後にメモ本文・合言葉をクリアする</span>
        </label>
      </div>

      <details
        className="rounded border border-stone-200 bg-stone-50 px-3 py-2"
        open={showAdvanced}
        onToggle={(e) => setShowAdvanced((e.target as HTMLDetailsElement).open)}
      >
        <summary className="cursor-pointer text-sm font-semibold text-stone-800">上級者オプション（Argon2id）</summary>
        <div className="mt-3 grid gap-3 sm:grid-cols-3">
          <label className="block text-sm font-medium text-stone-700">
            メモリ(KiB)
            <input type="number" aria-label="m_cost" value={mCost} min={8192} max={1048576} onChange={(e) => setMCost(Number(e.target.value))} className="input-surface mt-1" />
          </label>
          <label className="block text-sm font-medium text-stone-700">
            反復回数
            <input type="number" aria-label="t_cost" value={tCost} min={1} onChange={(e) => setTCost(Number(e.target.value))} className="input-surface mt-1" />
          </label>
          <label className="block text-sm font-medium text-stone-700">
            並列数
            <input type="number" aria-label="p_cost" value={pCost} min={1} onChange={(e) => setPCost(Number(e.target.value))} className="input-surface mt-1" />
          </label>
        </div>
      </details>

      <div className="flex flex-col gap-3 border-t border-stone-200 pt-4 sm:flex-row sm:items-center">
        <button type="button" onClick={onEncrypt} disabled={!canEncrypt} className="button-primary sm:w-40">
          {busy ? "処理中..." : "暗号化する"}
        </button>
        <p className="text-xs leading-5 text-stone-500">
          合言葉を忘れると復号できません。この内容はサーバーに送信されません。
        </p>
      </div>
      {error && <p className="rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{error}</p>}

      {output && (
        <div className="section-panel">
          <div className="mb-2 flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <h3 className="font-bold text-stone-950">暗号化済みテキスト</h3>
              <p className="field-hint">このテキストを保存してください。</p>
            </div>
            <div className="flex gap-2">
              <button type="button" className="button-secondary" onClick={() => navigator.clipboard.writeText(output)}>コピー</button>
              <button
                type="button"
                className="button-secondary"
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
          <textarea aria-label="暗号化済みテキスト" readOnly value={output} className="input-surface font-mono text-sm" rows={5} />
        </div>
      )}
    </section>
  );
}
