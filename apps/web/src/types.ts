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
  total_sent_sat?: number | null;
  tx_count?: number | null;
  notes?: string | null;
}

export interface SourceTypeStats {
  source_type: SourceType;
  wallets: number;
  keys: number;
  balance_sat: number;
}

export interface ScanSummary {
  wallets: number;
  total_keys: number;
  unique_addresses: number;
  funded_addresses: number;
  spent_addresses: number;
  unfunded_addresses: number;
  total_received_sat: number;
  total_sent_sat: number;
  total_balance_sat: number;
  by_source_type: SourceTypeStats[];
  provider?: string | null;
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

export type TabId = "scan" | "mnemonic" | "results";

export interface Tx {
  txid: string;
  time: number;
  value_sat: number;
  fee_sat: number | null;
  confirmations: number | null;
  block_height: number | null;
}

export type Provider = "blockstream" | "blockchain" | "mock" | "none";
