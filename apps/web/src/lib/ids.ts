async function sha256Hex(input: string): Promise<string> {
  const buf = new TextEncoder().encode(input);
  const digest = await crypto.subtle.digest("SHA-256", buf);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

export async function walletId(
  sourceFile: string,
  sourceType: string,
): Promise<string> {
  const hex = await sha256Hex(`${sourceFile}|${sourceType}`);
  return hex.slice(0, 16);
}

export async function keyId(address: string): Promise<string> {
  const hex = await sha256Hex(address);
  return hex.slice(0, 16);
}
