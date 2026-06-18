import { useState } from "react";
import EncryptTab from "./components/EncryptTab";
import DecryptTab from "./components/DecryptTab";
import SafetyAccordion from "./components/SafetyAccordion";

type Tab = "encrypt" | "decrypt";

export default function App() {
  const [tab, setTab] = useState<Tab>("encrypt");
  return (
    <main className="min-h-screen">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-4 py-5 sm:px-6 lg:px-8">
        <header className="grid gap-5 border-b border-stone-300/70 pb-5 lg:grid-cols-[minmax(0,1fr)_22rem] lg:items-end">
          <div>
            <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-teal-800">
              Local-only private memo
            </p>
            <h1 className="text-3xl font-bold tracking-normal text-stone-950 sm:text-4xl">
              Open Secret Memo
            </h1>
            <p className="mt-3 max-w-2xl text-sm leading-6 text-stone-700 sm:text-base">
              見られたくないメモをブラウザの中だけで暗号化し、好きな場所に保存できるテキストへ変換します。
            </p>
          </div>
          <div className="grid grid-cols-3 gap-2 text-center text-xs text-stone-700 sm:text-sm">
            <div className="rounded border border-stone-300 bg-white/70 px-2 py-3">
              <span className="block font-bold text-stone-950">送信なし</span>
              <span>ローカル処理</span>
            </div>
            <div className="rounded border border-stone-300 bg-white/70 px-2 py-3">
              <span className="block font-bold text-stone-950">保存なし</span>
              <span>平文非保持</span>
            </div>
            <div className="rounded border border-stone-300 bg-white/70 px-2 py-3">
              <span className="block font-bold text-stone-950">復旧可</span>
              <span>仕様公開</span>
            </div>
          </div>
        </header>

        <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_20rem] lg:items-start">
          <section className="rounded-lg border border-stone-300 bg-stone-50/85 p-2 shadow-sm">
            <div className="grid grid-cols-2 gap-2" role="tablist" aria-label="操作の選択">
              <button
                role="tab"
                aria-selected={tab === "encrypt"}
                onClick={() => setTab("encrypt")}
                className={[
                  "min-h-12 rounded px-3 text-sm font-semibold transition",
                  tab === "encrypt"
                    ? "bg-teal-700 text-white shadow-sm"
                    : "bg-transparent text-stone-700 hover:bg-white hover:text-stone-950",
                ].join(" ")}
              >
                暗号化
              </button>
              <button
                role="tab"
                aria-selected={tab === "decrypt"}
                onClick={() => setTab("decrypt")}
                className={[
                  "min-h-12 rounded px-3 text-sm font-semibold transition",
                  tab === "decrypt"
                    ? "bg-teal-700 text-white shadow-sm"
                    : "bg-transparent text-stone-700 hover:bg-white hover:text-stone-950",
                ].join(" ")}
              >
                復号
              </button>
            </div>
            <div className="mt-2 rounded-md bg-white p-4 shadow-sm sm:p-6">
              <div hidden={tab !== "encrypt"}>
                <EncryptTab />
              </div>
              <div hidden={tab !== "decrypt"}>
                <DecryptTab />
              </div>
            </div>
          </section>

          <aside className="space-y-4">
            <div className="section-panel">
              <p className="text-sm font-semibold text-stone-950">この画面で完結</p>
              <p className="mt-2 text-sm leading-6 text-stone-600">
                メモ本文・合言葉・復号結果はサーバーへ送られません。暗号化済みテキストだけをコピーして保存します。
              </p>
            </div>
            <SafetyAccordion />
          </aside>
        </div>
      </div>
    </main>
  );
}
