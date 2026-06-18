import init, { decrypt } from "./osm.js";

const errors = {
  auth_failed: "復号できませんでした。合言葉が違うか、暗号文が壊れている可能性があります。",
  malformed: "暗号文の形式が正しくありません。全体をコピーできているか確認してください。",
  unsupported_version: "この暗号文は新しい版で作られています。",
  invalid_word: "単語リストにない語があります。写し間違いの可能性があります。",
  not_utf8: "復号できましたが、テキストとして読み取れませんでした。"
};

async function main() {
  await init();
  const out = document.getElementById("out");
  document.getElementById("go").addEventListener("click", () => {
    out.classList.remove("err");
    const ct = document.getElementById("ct").value;
    const pw = document.getElementById("pw").value;
    const r = decrypt(ct, pw);
    if (r.ok) {
      out.textContent = r.text;
    } else {
      out.classList.add("err");
      out.textContent = errors[r.error_kind] || "復号中にエラーが発生しました。";
    }
  });
}

main();
