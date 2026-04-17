import { describe, it, expect } from "vitest";
import { walletId, keyId } from "./ids";

describe("walletId", () => {
  it("is deterministic for same source_file + source_type", async () => {
    const a = await walletId("/path/wallet.dat", "bitcoin_core");
    const b = await walletId("/path/wallet.dat", "bitcoin_core");
    expect(a).toBe(b);
  });

  it("differs for different source_file", async () => {
    const a = await walletId("/path/a.dat", "bitcoin_core");
    const b = await walletId("/path/b.dat", "bitcoin_core");
    expect(a).not.toBe(b);
  });

  it("differs for different source_type", async () => {
    const a = await walletId("/path/a.dat", "bitcoin_core");
    const b = await walletId("/path/a.dat", "multibit");
    expect(a).not.toBe(b);
  });

  it("returns a 16-char lowercase hex string", async () => {
    const id = await walletId("/path/wallet.dat", "bitcoin_core");
    expect(id).toMatch(/^[0-9a-f]{16}$/);
  });
});

describe("keyId", () => {
  it("is deterministic for an address", async () => {
    const a = await keyId("bc1qxyzabcdefghijk123456789kp8z");
    const b = await keyId("bc1qxyzabcdefghijk123456789kp8z");
    expect(a).toBe(b);
  });

  it("returns a 16-char lowercase hex string", async () => {
    const id = await keyId("bc1qxyzabcdefghijk123456789kp8z");
    expect(id).toMatch(/^[0-9a-f]{16}$/);
  });
});
