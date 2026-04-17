import { describe, it, expect, afterEach } from "vitest";
import { isTauri, isFsAccess } from "./tauri";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    showDirectoryPicker?: unknown;
  }
}

afterEach(() => {
  delete window.__TAURI_INTERNALS__;
  delete window.showDirectoryPicker;
});

describe("tauri detection", () => {
  it("isTauri is false when __TAURI_INTERNALS__ is absent", () => {
    expect(isTauri()).toBe(false);
  });

  it("isTauri is true when __TAURI_INTERNALS__ is present", () => {
    window.__TAURI_INTERNALS__ = {};
    expect(isTauri()).toBe(true);
  });
});

describe("fs-access detection", () => {
  it("is false when showDirectoryPicker is absent", () => {
    expect(isFsAccess()).toBe(false);
  });

  it("is true when showDirectoryPicker is a function", () => {
    window.showDirectoryPicker = () => {};
    expect(isFsAccess()).toBe(true);
  });

  it("is false when showDirectoryPicker is a non-function value", () => {
    window.showDirectoryPicker = "not-a-function";
    expect(isFsAccess()).toBe(false);
  });
});
