export interface Strength {
  level: "weak" | "ok" | "strong";
  message: string;
}

/**
 * Heuristic only — never blocks encryption (warning UI). Considers both the
 * number of 「、」-separated tokens (word-style) and total length (sentence-style).
 */
export function estimateStrength(passphrase: string): Strength {
  const p = passphrase.trim();
  const wordCount = p.split("、").map((w) => w.trim()).filter(Boolean).length;
  const len = [...p].length;

  if (wordCount >= 6 || len >= 24) {
    return { level: "strong", message: "強い合言葉です。" };
  }
  if (wordCount >= 4 || len >= 12) {
    return { level: "ok", message: "まずまずです。単語を6個以上にするとより安全です。" };
  }
  return {
    level: "weak",
    message: "弱い合言葉です。無関係な単語を「、」で6個以上つなげる方法がおすすめです。"
  };
}
