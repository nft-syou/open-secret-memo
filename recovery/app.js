import init, { decrypt } from "./osm.js";

const errors = {
  auth_failed: "復号できませんでした。合言葉が違うか、暗号文が壊れている可能性があります。",
  malformed: "暗号文の形式が正しくありません。全体をコピーできているか確認してください。",
  unsupported_version: "この暗号文は新しい版で作られています。",
  invalid_word: "単語リストにない語があります。写し間違いの可能性があります。",
  not_utf8: "復号できましたが、テキストとして読み取れませんでした。"
};

async function main() {
  const out = document.getElementById("out");
  const btn = document.getElementById("go");
  try {
    await init();
  } catch (e) {
    out.classList.add("err");
    out.textContent = "復号エンジンの読み込みに失敗しました。簡易サーバー (python3 -m http.server) 経由で開いているか確認してください。";
    btn.disabled = true;
    return;
  }
  btn.addEventListener("click", () => {
    out.classList.remove("err");
    try {
      const r = decrypt(document.getElementById("ct").value, document.getElementById("pw").value);
      if (r.ok) {
        out.textContent = r.text;
      } else {
        out.classList.add("err");
        out.textContent = errors[r.error_kind] || "復号中にエラーが発生しました。";
      }
    } catch (e) {
      out.classList.add("err");
      out.textContent = "復号中に予期しないエラーが発生しました。";
    }
  });
}

main();
