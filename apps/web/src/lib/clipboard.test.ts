import { describe, it, expect, vi, beforeEach } from "vitest";
import { copyWithAutoClear } from "./clipboard";

const writeText = vi.fn();
const readText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  readText.mockReset();
  Object.assign(navigator, {
    clipboard: { writeText, readText },
  });
  vi.useFakeTimers();
});

describe("copyWithAutoClear", () => {
  it("writes the value to the clipboard immediately", async () => {
    writeText.mockResolvedValue(undefined);
    await copyWithAutoClear("L1aBc", 20_000);
    expect(writeText).toHaveBeenCalledWith("L1aBc");
  });

  it("clears the clipboard after the delay if it still contains the value", async () => {
    writeText.mockResolvedValue(undefined);
    readText.mockResolvedValue("L1aBc");
    await copyWithAutoClear("L1aBc", 20_000);
    await vi.advanceTimersByTimeAsync(20_000);
    expect(writeText).toHaveBeenLastCalledWith("");
  });

  it("does not clear the clipboard if the user has overwritten it", async () => {
    writeText.mockResolvedValue(undefined);
    readText.mockResolvedValue("something-else");
    await copyWithAutoClear("L1aBc", 20_000);
    await vi.advanceTimersByTimeAsync(20_000);
    expect(writeText).toHaveBeenCalledTimes(1);
  });
});
