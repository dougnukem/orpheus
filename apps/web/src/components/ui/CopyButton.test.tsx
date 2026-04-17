import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CopyButton } from "./CopyButton";

const writeText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  writeText.mockResolvedValue(undefined);
  Object.assign(navigator, { clipboard: { writeText } });
});

describe("CopyButton", () => {
  it("writes the value to the clipboard on click", async () => {
    render(<CopyButton value="bc1qabc" />);
    await userEvent.click(screen.getByRole("button"));
    expect(writeText).toHaveBeenCalledWith("bc1qabc");
  });

  it("shows the 'copied' label briefly after click", async () => {
    render(<CopyButton value="bc1qabc" />);
    await userEvent.click(screen.getByRole("button"));
    expect(screen.getByRole("button")).toHaveAccessibleName(/copied/i);
  });
});
