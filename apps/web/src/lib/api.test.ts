import { describe, it, expect, vi, beforeEach } from "vitest";

describe("api.scan", () => {
  beforeEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("POSTs multipart to /api/scan outside Tauri", async () => {
    const fetchMock = vi.fn(async () =>
      new Response(JSON.stringify({ results: [], summary: null }), {
        status: 200,
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const { scan } = await import("./api");
    await scan([new File(["x"], "a.dat")], "", "blockstream");

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/scan",
      expect.objectContaining({ method: "POST" }),
    );
  });
});

describe("api.addressTransactions", () => {
  beforeEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown })
      .__TAURI_INTERNALS__;
  });

  it("GETs /api/address/:addr/transactions with provider query", async () => {
    const fetchMock = vi.fn(async () => new Response("[]", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    const { addressTransactions } = await import("./api");
    await addressTransactions("bc1qxyz", "blockstream", 50);

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/address/bc1qxyz/transactions?provider=blockstream&limit=50",
    );
  });
});
