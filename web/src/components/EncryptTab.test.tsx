import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import EncryptTab from "./EncryptTab";

vi.mock("../crypto/client", () => ({
  encryptMemo: vi.fn(async () => "OSM1.RESULT")
}));

beforeEach(() => vi.clearAllMocks());

describe("EncryptTab", () => {
  it("disables encrypt button until memo, passphrase, and matching confirm are present", async () => {
    render(<EncryptTab />);
    const button = screen.getByRole("button", { name: "暗号化する" });
    expect(button).toBeDisabled();

    fireEvent.change(screen.getByLabelText("メモ本文"), { target: { value: "secret" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "pass" } });
    fireEvent.change(screen.getByLabelText("合言葉（確認）"), { target: { value: "different" } });
    expect(button).toBeDisabled();
    expect(screen.getByText("合言葉が一致しません。")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("合言葉（確認）"), { target: { value: "pass" } });
    expect(button).toBeEnabled();
  });

  it("shows the ciphertext after encrypting", async () => {
    render(<EncryptTab />);
    fireEvent.change(screen.getByLabelText("メモ本文"), { target: { value: "secret" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "pass" } });
    fireEvent.change(screen.getByLabelText("合言葉（確認）"), { target: { value: "pass" } });
    fireEvent.click(screen.getByRole("button", { name: "暗号化する" }));
    await waitFor(() => expect(screen.getByDisplayValue("OSM1.RESULT")).toBeInTheDocument());
  });
});
