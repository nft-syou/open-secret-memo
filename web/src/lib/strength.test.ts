import { describe, expect, it } from "vitest";
import { estimateStrength } from "./strength";

describe("estimateStrength", () => {
  it("flags short passphrases as weak", () => {
    expect(estimateStrength("abc").level).toBe("weak");
  });

  it("treats 4-5 comma-separated words as ok", () => {
    expect(estimateStrength("紙袋、みかん、夜道、ラジオ").level).toBe("ok");
  });

  it("treats 6+ comma-separated words as strong", () => {
    expect(estimateStrength("紙袋、みかん、夜道、ラジオ、階段、ペンギン").level).toBe("strong");
  });

  it("treats a long sentence as strong", () => {
    expect(estimateStrength("紙袋を持ったカエルが夜の図書館でカレーの作り方を読んでいた").level).toBe("strong");
  });
});
