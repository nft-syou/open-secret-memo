import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("./crypto/client", () => ({
  decryptMemo: vi.fn(),
  encryptMemo: vi.fn(),
}));

describe("App", () => {
  it("keeps form input values when switching tabs", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("メモ本文"), { target: { value: "残したいメモ" } });
    fireEvent.change(screen.getAllByLabelText("合言葉")[0], { target: { value: "見える合言葉" } });
    fireEvent.change(screen.getByLabelText("合言葉（確認）"), { target: { value: "見える合言葉" } });

    fireEvent.click(screen.getByRole("tab", { name: "復号" }));
    fireEvent.change(screen.getByLabelText("暗号化済みテキスト"), { target: { value: "OSM1.saved" } });

    fireEvent.click(screen.getByRole("tab", { name: "暗号化" }));
    expect(screen.getByDisplayValue("残したいメモ")).toBeInTheDocument();
    expect(screen.getAllByDisplayValue("見える合言葉")).toHaveLength(2);

    fireEvent.click(screen.getByRole("tab", { name: "復号" }));
    expect(screen.getByDisplayValue("OSM1.saved")).toBeInTheDocument();
  });

  it("shows copyright and MIT license information in the footer", () => {
    render(<App />);

    expect(screen.getByText("Copyright © 2026 Open Secret Memo contributors.")).toBeInTheDocument();
    expect(screen.getByText("Released under the MIT License.")).toBeInTheDocument();
  });
});
