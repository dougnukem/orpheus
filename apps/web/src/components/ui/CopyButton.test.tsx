import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, act, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CopyButton } from "./CopyButton";

const writeText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  writeText.mockResolvedValue(undefined);
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText },
    configurable: true,
    writable: true,
  });
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

  describe("with fake timers", () => {
    beforeEach(() => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it("resets the label after 1500ms", async () => {
      render(<CopyButton value="bc1qabc" />);
      await act(async () => {
        fireEvent.click(screen.getByRole("button"));
      });
      expect(screen.getByRole("button")).toHaveAccessibleName(/copied/i);
      await act(async () => {
        vi.advanceTimersByTime(1500);
      });
      expect(screen.getByRole("button")).toHaveAccessibleName(/^copy$/i);
    });

    it("clears the prior timer when clicked twice in quick succession", async () => {
      render(<CopyButton value="bc1qabc" />);
      await act(async () => {
        fireEvent.click(screen.getByRole("button"));
      });
      expect(screen.getByRole("button")).toHaveAccessibleName(/copied/i);
      await act(async () => {
        vi.advanceTimersByTime(1000);
      });
      await act(async () => {
        fireEvent.click(screen.getByRole("button"));
      });
      expect(screen.getByRole("button")).toHaveAccessibleName(/copied/i);
      await act(async () => {
        vi.advanceTimersByTime(1000);
      });
      expect(screen.getByRole("button")).toHaveAccessibleName(/copied/i);
    });
  });

  it("does not flip to 'Copied' when the clipboard write rejects", async () => {
    const err = vi.spyOn(console, "error").mockImplementation(() => {});
    writeText.mockRejectedValueOnce(new Error("denied"));
    render(<CopyButton value="bc1qabc" />);
    await userEvent.click(screen.getByRole("button"));
    expect(screen.getByRole("button")).toHaveAccessibleName(/^copy$/i);
    expect(err).toHaveBeenCalled();
    err.mockRestore();
  });
});
