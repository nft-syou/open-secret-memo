import { describe, expect, it } from "vitest";
import { decryptErrorMessage } from "./errors";

describe("decryptErrorMessage", () => {
  it("maps each known kind to its Japanese message", () => {
    expect(decryptErrorMessage("auth_failed")).toMatch(/合言葉が違うか/);
    expect(decryptErrorMessage("malformed")).toMatch(/形式が正しくありません/);
    expect(decryptErrorMessage("unsupported_version")).toMatch(/新しい版/);
    expect(decryptErrorMessage("not_utf8")).toMatch(/読み取れませんでした/);
  });

  it("interpolates the offending word for invalid_word", () => {
    expect(decryptErrorMessage("invalid_word", "ぴよぴよ")).toContain("ぴよぴよ");
  });

  it("falls back to a generic message for unknown kinds", () => {
    expect(decryptErrorMessage("something-else")).toMatch(/予期しないエラー/);
  });
});
