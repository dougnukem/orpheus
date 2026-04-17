export const WALLET_FILE_PATTERNS = [
  ".dat",
  ".wallet",
  ".aes.json",
  ".json",
  ".txt",
] as const;

const DEFAULT_MAX_BYTES = 50 * 1024 * 1024; // 50 MiB

function hasWalletExt(name: string): boolean {
  const lower = name.toLowerCase();
  return WALLET_FILE_PATTERNS.some((ext) => lower.endsWith(ext));
}

export async function walkDirectory(
  dir: FileSystemDirectoryHandle,
  opts: { maxFileBytes?: number } = {},
): Promise<File[]> {
  const maxBytes = opts.maxFileBytes ?? DEFAULT_MAX_BYTES;
  const out: File[] = [];
  await visit(dir, out, maxBytes);
  return out;
}

async function visit(
  dir: FileSystemDirectoryHandle,
  out: File[],
  maxBytes: number,
): Promise<void> {
  for await (const entry of dir.values()) {
    if (entry.kind === "directory") {
      await visit(entry, out, maxBytes);
    } else if (hasWalletExt(entry.name)) {
      const file = await entry.getFile();
      if (file.size <= maxBytes) out.push(file);
    }
  }
}
