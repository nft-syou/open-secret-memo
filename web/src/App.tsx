import { useState } from "react";
import EncryptTab from "./components/EncryptTab";

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
      <div>{tab === "encrypt" ? <EncryptTab /> : <p>decrypt</p>}</div>
    </main>
  );
}
