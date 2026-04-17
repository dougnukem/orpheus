import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WifReveal } from "./WifReveal";

const writeText = vi.fn();
const readText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  readText.mockReset();
  writeText.mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText, readText },
    configurable: true,
    writable: true,
  });
});

describe("WifReveal", () => {
  it("hides the WIF by default", () => {
    render(<WifReveal wif="L1abcDEF" />);
    expect(screen.queryByText("L1abcDEF")).not.toBeInTheDocument();
  });

  it("has an accessible label for the hidden state", () => {
    render(<WifReveal wif="L1abcDEF" />);
    expect(screen.getByLabelText(/private key \(hidden/i)).toBeInTheDocument();
  });

  it("reveals the WIF after clicking Reveal", async () => {
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    expect(screen.getByText("L1abcDEF")).toBeInTheDocument();
  });

  it("copies to clipboard when Copy is clicked after reveal", async () => {
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    await userEvent.click(screen.getByRole("button", { name: /copy/i }));
    expect(writeText).toHaveBeenCalledWith("L1abcDEF");
  });

  it("can be hidden again after revealing", async () => {
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    await userEvent.click(screen.getByRole("button", { name: /hide/i }));
    expect(screen.queryByText("L1abcDEF")).not.toBeInTheDocument();
  });

  it("moves focus to the Hide button after reveal", async () => {
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    expect(screen.getByRole("button", { name: /hide/i })).toHaveFocus();
  });

  it("logs an error when the clipboard write rejects", async () => {
    const err = vi.spyOn(console, "error").mockImplementation(() => {});
    writeText.mockRejectedValueOnce(new Error("denied"));
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    await userEvent.click(screen.getByRole("button", { name: /copy/i }));
    await vi.waitFor(() => expect(err).toHaveBeenCalled());
    err.mockRestore();
  });
});
