import type {
  DecodedMnemonic,
  ExtractedKey,
  Provider,
  ScanSummary,
  Tx,
  WalletScanResult,
} from "@/types";
import { isTauri, tauriInvoke } from "./tauri";

const BASE = "/api";

export interface ScanReply {
  results: WalletScanResult[];
  summary: ScanSummary | null;
}

export interface MnemonicReply {
  keys?: ExtractedKey[];
  decoded?: DecodedMnemonic;
}

async function httpJson<T>(path: string, init?: RequestInit): Promise<T> {
  const r = init
    ? await fetch(`${BASE}${path}`, init)
    : await fetch(`${BASE}${path}`);
  const text = await r.text();
  let body: unknown;
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    throw new Error(`Non-JSON from ${path}: ${text.slice(0, 200)}`);
  }
  if (!r.ok) {
    const msg =
      body && typeof body === "object" && body !== null && "error" in body
        ? String((body as { error: unknown }).error)
        : r.statusText;
    throw new Error(msg);
  }
  return body as T;
}

export async function scan(
  files: File[],
  passwords: string,
  provider: Provider,
): Promise<ScanReply> {
  const fd = new FormData();
  for (const f of files) fd.append("wallet", f);
  if (passwords) fd.append("passwords", passwords);
  fd.append("provider", provider);
  return httpJson("/scan", { method: "POST", body: fd });
}

export async function scanDirectory(
  path: string,
  passwords: string,
  provider: Provider,
): Promise<ScanReply> {
  if (isTauri()) {
    return tauriInvoke<ScanReply>("scan_directory", {
      path,
      passwords,
      provider,
    });
  }
  return httpJson("/scan-directory", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path, passwords, provider }),
  });
}

export async function mnemonic(payload: {
  phrase: string;
  kind: "bip39" | "blockchain";
  passphrase?: string;
  gap_limit?: number;
  wordlist?: string;
}): Promise<MnemonicReply> {
  if (isTauri()) {
    return tauriInvoke<MnemonicReply>("mnemonic_cmd", { payload });
  }
  return httpJson("/mnemonic", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
}

export async function demo(): Promise<ScanReply> {
  return httpJson("/demo", { method: "POST" });
}

export async function addressTransactions(
  address: string,
  provider: Provider,
  limit = 50,
): Promise<Tx[]> {
  if (isTauri()) {
    return tauriInvoke<Tx[]>("address_transactions", {
      address,
      provider,
      limit,
    });
  }
  const qs = new URLSearchParams({ provider, limit: String(limit) });
  return httpJson(
    `/address/${encodeURIComponent(address)}/transactions?${qs}`,
  );
}
