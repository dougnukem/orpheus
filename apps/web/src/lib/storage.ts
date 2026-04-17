import type { WalletScanResult } from "@/types";

export const STORAGE_KEY = "orpheus:v1:results";
const SCHEMA_VERSION = "v1";

interface Envelope {
  version: string;
  data: WalletScanResult[];
}

export function getResults(): WalletScanResult[] | null {
  const raw = sessionStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Envelope;
    if (parsed.version !== SCHEMA_VERSION) return null;
    return parsed.data;
  } catch {
    return null;
  }
}

export function setResults(data: WalletScanResult[]): void {
  const envelope: Envelope = { version: SCHEMA_VERSION, data };
  sessionStorage.setItem(STORAGE_KEY, JSON.stringify(envelope));
}

export function clearResults(): void {
  sessionStorage.removeItem(STORAGE_KEY);
}
