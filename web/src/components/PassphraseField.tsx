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
