import { APP_VERSION, CRYPTO_SUMMARY, GITHUB_URL, IPFS_CID, SPEC_URL } from "../lib/appInfo";

export default function SafetyAccordion() {
  return (
    <details className="section-panel" open>
      <summary className="cursor-pointer text-sm font-semibold text-stone-950">このサイトは安全？</summary>
      <div className="mt-3 space-y-2 text-sm leading-6 text-stone-600">
        <p>合言葉・メモ本文・復号結果をサーバーに送信しません。</p>
        <p>合言葉・平文・復号結果を保存しません（このMVPでは暗号文も保存しません）。</p>
        <p>通信なし・オフラインでも使えます（PWA）。</p>
        <div className="rounded border border-stone-200 bg-stone-50 p-3 text-xs leading-5 text-stone-600">
          <p>暗号方式: {CRYPTO_SUMMARY}</p>
          <p>バージョン: {APP_VERSION}</p>
          <p>ソースコード: <a className="font-semibold text-teal-800 underline" href={GITHUB_URL}>GitHub</a></p>
          <p>仕様書: <a className="font-semibold text-teal-800 underline" href={SPEC_URL}>SPEC.md</a></p>
          <p className="break-all">IPFS CID: {IPFS_CID || "（ミラー準備中）"}</p>
        </div>
        <p className="font-semibold text-stone-950">合言葉のヒント</p>
        <p>日本語も使えます。無関係な単語を「、」で6個以上つなげるのがおすすめです。</p>
        <p>文章型の合言葉も使えます。合言葉を忘れると復号できません。このサイトにも保存されません。</p>
      </div>
    </details>
  );
}
