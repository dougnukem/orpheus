export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function isFsAccess(): boolean {
  return (
    typeof window !== "undefined" &&
    typeof (window as unknown as { showDirectoryPicker?: unknown })
      .showDirectoryPicker === "function"
  );
}

export type InputMethod = "tauri" | "fs-access" | "dropzone";

export function inputMethod(): InputMethod {
  if (isTauri()) return "tauri";
  if (isFsAccess()) return "fs-access";
  return "dropzone";
}

export async function tauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri()) throw new Error("tauriInvoke called outside Tauri");
  const mod = await import("@tauri-apps/api/core");
  return mod.invoke<T>(command, args);
}
