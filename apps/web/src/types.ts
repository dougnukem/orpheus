export type SourceType =
  | "bitcoin_core"
  | "multibit"
  | "bip39"
  | "blockchain_com"
  | "wallet_dump"
  | "encrypted"
  | "unknown";

export interface ExtractedKey {
  wif: string;
  address_compressed: string;
  address_uncompressed?: string | null;
  address_p2sh_segwit?: string | null;
  address_bech32?: string | null;
  source_file: string;
  source_type: SourceType;
  derivation_path?: string | null;
  balance_sat?: number | null;
  total_received_sat?: number | null;
  tx_count?: number | null;
  notes?: string | null;
}

export interface WalletScanResult {
  source_file: string;
  source_type: SourceType;
  keys: ExtractedKey[];
  error?: string | null;
}

export interface DecodedMnemonic {
  password: string;
  word_count: number;
  version: string;
}

export type TabId = "scan" | "extract" | "mnemonic" | "results";
