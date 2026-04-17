import type { WalletScanResult, ExtractedKey, DecodedMnemonic, ScanSummary } from "@/types";

const BASE = "/api";

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const r = await fetch(`${BASE}${path}`, init);
  const text = await r.text();
  let body: unknown;
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    throw new Error(`Non-JSON from ${path}: ${text.slice(0, 200)}`);
  }
  if (!r.ok) {
    const msg =
      body && typeof body === "object" && "error" in body
        ? String((body as { error: unknown }).error)
        : r.statusText;
    throw new Error(msg);
  }
  return body as T;
}

export interface ScanReply {
  results: WalletScanResult[];
  summary: ScanSummary;
}

export async function scan(files: File[], passwords: string, provider: string): Promise<ScanReply> {
  const fd = new FormData();
  for (const f of files) fd.append("wallet", f);
  if (passwords) fd.append("passwords", passwords);
  fd.append("provider", provider);
  return call("/scan", { method: "POST", body: fd });
}

export async function demo(): Promise<ScanReply> {
  return call("/demo", { method: "POST" });
}

export async function mnemonic(payload: {
  phrase: string;
  kind: "bip39" | "blockchain";
  passphrase?: string;
  gap_limit?: number;
  wordlist?: string;
}): Promise<{ keys?: ExtractedKey[]; decoded?: DecodedMnemonic }> {
  return call("/mnemonic", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
}
