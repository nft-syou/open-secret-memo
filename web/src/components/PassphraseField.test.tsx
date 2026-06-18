import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import PassphraseField from "./PassphraseField";

describe("PassphraseField", () => {
  it("shows the passphrase by default and can hide it", () => {
    render(<PassphraseField label="合言葉" value="ひみつ" onChange={vi.fn()} />);

    expect(screen.getByLabelText("合言葉")).toHaveAttribute("type", "text");

    fireEvent.click(screen.getByRole("button", { name: "隠す" }));
    expect(screen.getByLabelText("合言葉")).toHaveAttribute("type", "password");
  });
});
