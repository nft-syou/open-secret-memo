import { useState } from "react";

interface Props {
  label: string;
  value: string;
  onChange: (v: string) => void;
}

export default function PassphraseField({ label, value, onChange }: Props) {
  const [show, setShow] = useState(false);
  return (
    <label className="block">
      <span className="field-label">{label}</span>
      <span className="mt-1.5 flex gap-2">
        <input
          aria-label={label}
          type={show ? "text" : "password"}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="input-surface min-w-0 flex-1"
        />
        <button type="button" onClick={() => setShow((s) => !s)} className="button-secondary w-20 shrink-0">
          {show ? "隠す" : "表示"}
        </button>
      </span>
    </label>
  );
}
