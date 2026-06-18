/** Maps a wasm decrypt error kind to a Japanese user-facing message. */
export function decryptErrorMessage(kind: string, word?: string): string {
  switch (kind) {
    case "auth_failed":
      return "復号できませんでした。合言葉が違うか、暗号文が壊れている可能性があります。";
    case "malformed":
      return "暗号文の形式が正しくありません。全体をコピーできているか確認してください。";
    case "unsupported_version":
      return "この暗号文は新しい版で作られています。最新版アプリで復号してください。";
    case "invalid_word":
      return `「${word ?? ""}」は単語リストにありません。写し間違いの可能性があります。`;
    case "not_utf8":
      return "復号できましたが、テキストとして読み取れませんでした。";
    default:
      return "復号中に予期しないエラーが発生しました。";
  }
}
