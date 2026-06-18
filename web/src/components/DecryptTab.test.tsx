import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import DecryptTab from "./DecryptTab";
import * as client from "../crypto/client";

vi.mock("../crypto/client", () => ({ decryptMemo: vi.fn() }));

beforeEach(() => vi.clearAllMocks());

describe("DecryptTab", () => {
  it("shows decrypted text on success", async () => {
    (client.decryptMemo as any).mockResolvedValue({ ok: true, text: "my secret" });
    render(<DecryptTab />);
    fireEvent.change(screen.getByLabelText("暗号化済みテキスト"), { target: { value: "OSM1.X" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "pw" } });
    fireEvent.click(screen.getByRole("button", { name: "復号する" }));
    await waitFor(() => expect(screen.getByText("my secret")).toBeInTheDocument());
  });

  it("shows the mapped error message on auth failure", async () => {
    (client.decryptMemo as any).mockResolvedValue({ ok: false, errorKind: "auth_failed" });
    render(<DecryptTab />);
    fireEvent.change(screen.getByLabelText("暗号化済みテキスト"), { target: { value: "OSM1.X" } });
    fireEvent.change(screen.getByLabelText("合言葉"), { target: { value: "wrong" } });
    fireEvent.click(screen.getByRole("button", { name: "復号する" }));
    await waitFor(() =>
      expect(screen.getByText(/合言葉が違うか/)).toBeInTheDocument()
    );
  });
});
