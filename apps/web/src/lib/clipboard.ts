export async function copyWithAutoClear(
  value: string,
  clearAfterMs: number,
): Promise<void> {
  await navigator.clipboard.writeText(value);

  setTimeout(async () => {
    try {
      const current = await navigator.clipboard.readText();
      if (current === value) {
        await navigator.clipboard.writeText("");
      }
    } catch {
      // Some browsers deny readText without user gesture; skip auto-clear.
    }
  }, clearAfterMs);
}
