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
    expect(screen.getByLabelText(/private key/i)).toBeInTheDocument();
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
});
