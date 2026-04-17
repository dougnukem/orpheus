import { describe, it, expect, beforeEach } from "vitest";
import { getResults, setResults, clearResults, STORAGE_KEY } from "./storage";
import type { WalletScanResult } from "@/types";

const sample: WalletScanResult[] = [
  { source_file: "/a.dat", source_type: "bitcoin_core", keys: [] },
];

describe("storage", () => {
  beforeEach(() => sessionStorage.clear());

  it("returns null when nothing is stored", () => {
    expect(getResults()).toBeNull();
  });

  it("round-trips results through sessionStorage", () => {
    setResults(sample);
    expect(getResults()).toEqual(sample);
  });

  it("clearResults removes the entry", () => {
    setResults(sample);
    clearResults();
    expect(getResults()).toBeNull();
  });

  it("discards data with mismatched schema version", () => {
    sessionStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ version: "v0", data: sample }),
    );
    expect(getResults()).toBeNull();
  });

  it("discards malformed JSON", () => {
    sessionStorage.setItem(STORAGE_KEY, "not-json{");
    expect(getResults()).toBeNull();
  });
});
