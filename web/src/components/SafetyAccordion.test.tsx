import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import SafetyAccordion from "./SafetyAccordion";

describe("SafetyAccordion", () => {
  it("reveals the safety details when expanded", () => {
    render(<SafetyAccordion />);
    fireEvent.click(screen.getByText("このサイトは安全？"));
    expect(screen.getByText(/サーバーに送信しません/)).toBeInTheDocument();
    expect(screen.getByText(/Argon2id \+ AES-256-GCM/)).toBeInTheDocument();
  });
});
