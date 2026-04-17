# Web app redesign — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild `apps/web` on a Linear/Vercel-style design system, add bookmarkable routes with navigable results / address / import-instructions views, and extend the backend to return transaction history per address — all on branch `feat/web-redesign`.

**Architecture:** Tailwind v4 `@theme` tokens + small hand-rolled primitives; React Router for URL-addressable results; Tauri-first folder picking with File System Access API and dropzone fallbacks; balance-provider trait grows a `fetch_transactions()` method with pinned fixture tests; a single `apps/web/src/lib/api.ts` adapter hides the HTTP-vs-Tauri split from route components.

**Tech Stack:** React 19 · TypeScript · Tailwind v4 · Vite · React Router v6 · Vitest + Testing Library · Rust 1.95 · axum 0.7 · Tauri v2 · reqwest blocking.

**Spec:** [`docs/superpowers/specs/2026-04-17-web-redesign-design.md`](../specs/2026-04-17-web-redesign-design.md)

---

## Working directory

All paths are relative to the worktree root. Before starting any task:

```bash
cd /Users/ddaniels/Dropbox/orpheus/.worktrees/web-redesign
git status   # should show branch feat/web-redesign, clean working tree
```

All `mise run …` commands work from the worktree root. For targeted web tests use `pnpm -C apps/web run <script>`. For targeted Rust tests use `cargo test -p <crate> <filter>` from the worktree root.

---

## Phase 0 — Setup

### Task 0.1: Install web dev dependencies

**Files:**

- Modify: `apps/web/package.json`

- [ ] **Step 1: Add runtime + dev dependencies**

```bash
pnpm -C apps/web add react-router-dom@^6.28.0 qrcode-generator@^1.4.4
pnpm -C apps/web add -D vitest@^2.1.8 @vitest/ui@^2.1.8 jsdom@^25.0.1 \
  @testing-library/react@^16.1.0 @testing-library/user-event@^14.5.2 \
  @testing-library/jest-dom@^6.6.3
```

- [ ] **Step 2: Add test scripts to `apps/web/package.json`**

In the `"scripts"` object, add:

```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 3: Verify install**

```bash
pnpm -C apps/web install --frozen-lockfile
pnpm -C apps/web run typecheck
```

Expected: both commands exit 0.

- [ ] **Step 4: Commit**

```bash
git add apps/web/package.json apps/web/pnpm-lock.yaml
git commit -m "build(web): add react-router, qrcode-generator, vitest + testing-library"
```

---

### Task 0.2: Configure Vitest

**Files:**

- Create: `apps/web/vitest.config.ts`
- Create: `apps/web/src/test-setup.ts`

- [ ] **Step 1: Create `apps/web/vitest.config.ts`**

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    css: true,
  },
});
```

- [ ] **Step 2: Create `apps/web/src/test-setup.ts`**

```ts
import "@testing-library/jest-dom/vitest";
```

- [ ] **Step 3: Write a smoke test to prove the harness works**

Create `apps/web/src/lib/__smoke__.test.ts`:

```ts
import { describe, it, expect } from "vitest";

describe("vitest harness", () => {
  it("runs a test", () => {
    expect(1 + 1).toBe(2);
  });
});
```

- [ ] **Step 4: Run vitest, verify green**

```bash
pnpm -C apps/web run test
```

Expected: 1 passed.

- [ ] **Step 5: Delete the smoke test (it has served its purpose) and commit**

```bash
rm apps/web/src/lib/__smoke__.test.ts
git add apps/web/vitest.config.ts apps/web/src/test-setup.ts apps/web/package.json apps/web/pnpm-lock.yaml
git commit -m "build(web): configure vitest with jsdom + testing-library"
```

---

### Task 0.3: Tokens + base CSS reset (fixes the undefined-variable bug)

**Files:**

- Rewrite: `apps/web/src/index.css`
- Modify: `apps/web/index.html`

- [ ] **Step 1: Rewrite `apps/web/src/index.css`**

Replace the entire file contents with:

```css
@import "tailwindcss";

@theme {
  --color-bg: #0b0d10;
  --color-surface: #11141a;
  --color-border: #1d2125;
  --color-text: #e6e8eb;
  --color-text-dim: #9ba1a6;
  --color-text-faint: #7a8087;
  --color-accent: #3b82f6;
  --color-accent-lit: #60a5fa;
  --color-success: #22c55e;
  --color-warn: #f59e0b;
  --color-danger: #f87171;

  --font-sans: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto,
    sans-serif;
  --font-mono: ui-monospace, "SF Mono", Menlo, monospace;
}

@media (prefers-color-scheme: light) {
  :root {
    --color-bg: #ffffff;
    --color-surface: #f6f8fa;
    --color-border: #d0d7de;
    --color-text: #1f2328;
    --color-text-dim: #57606a;
    --color-text-faint: #8c959f;
    --color-accent: #0969da;
    --color-accent-lit: #218bff;
    --color-success: #1a7f37;
    --color-warn: #9a6700;
    --color-danger: #d1242f;
  }
}

:root {
  font-family: var(--font-sans);
  font-size: 14px;
  line-height: 1.5;
  color: var(--color-text);
  background: var(--color-bg);
  color-scheme: dark light;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
}

html,
body,
#root {
  margin: 0;
  min-height: 100vh;
}

*,
*::before,
*::after {
  box-sizing: border-box;
}

*:focus-visible {
  outline: 2px solid var(--color-accent);
  outline-offset: 2px;
}

@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    transition-duration: 0.001ms !important;
  }
}
```

- [ ] **Step 2: Remove the Google Fonts link from `apps/web/index.html`**

Replace the entire `<head>` block with:

```html
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <meta name="color-scheme" content="dark light" />
  <title>Orpheus — wallet recovery</title>
</head>
```

(Google Fonts was pulling in Cormorant Garamond + IBM Plex Mono for the old serif/mono design. The new design uses only system fonts — no network font fetch.)

- [ ] **Step 3: Smoke test the dev server**

```bash
mise run web:dev
```

Open `http://localhost:5173` — expect the page to render **something** (current App.tsx is still the old broken one, but the CSS vars it references will now be defined). No terminal errors. Stop the dev server with Ctrl-C.

- [ ] **Step 4: Commit**

```bash
git add apps/web/src/index.css apps/web/index.html
git commit -m "style(web): add @theme tokens (light+dark) and base reset"
```

---

## Phase 1 — lib helpers (pure TS, TDD)

### Task 1.1: `lib/ids.ts` — walletId / keyId derivation

**Files:**

- Create: `apps/web/src/lib/ids.ts`
- Create: `apps/web/src/lib/ids.test.ts`

- [ ] **Step 1: Write the failing test**

Create `apps/web/src/lib/ids.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { walletId, keyId } from "./ids";

describe("walletId", () => {
  it("is deterministic for same source_file + source_type", async () => {
    const a = await walletId("/path/wallet.dat", "bitcoin_core");
    const b = await walletId("/path/wallet.dat", "bitcoin_core");
    expect(a).toBe(b);
  });

  it("differs for different source_file", async () => {
    const a = await walletId("/path/a.dat", "bitcoin_core");
    const b = await walletId("/path/b.dat", "bitcoin_core");
    expect(a).not.toBe(b);
  });

  it("differs for different source_type", async () => {
    const a = await walletId("/path/a.dat", "bitcoin_core");
    const b = await walletId("/path/a.dat", "multibit");
    expect(a).not.toBe(b);
  });

  it("returns a 16-char lowercase hex string", async () => {
    const id = await walletId("/path/wallet.dat", "bitcoin_core");
    expect(id).toMatch(/^[0-9a-f]{16}$/);
  });
});

describe("keyId", () => {
  it("is deterministic for an address", async () => {
    const a = await keyId("bc1qxyzabcdefghijk123456789kp8z");
    const b = await keyId("bc1qxyzabcdefghijk123456789kp8z");
    expect(a).toBe(b);
  });

  it("returns a 16-char lowercase hex string", async () => {
    const id = await keyId("bc1qxyzabcdefghijk123456789kp8z");
    expect(id).toMatch(/^[0-9a-f]{16}$/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm -C apps/web run test ids
```

Expected: FAIL — "Cannot find module './ids'".

- [ ] **Step 3: Implement `apps/web/src/lib/ids.ts`**

```ts
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
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test ids
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/lib/ids.ts apps/web/src/lib/ids.test.ts
git commit -m "feat(web): add walletId and keyId derivation in lib/ids.ts"
```

---

### Task 1.2: `lib/storage.ts` — sessionStorage with schema version

**Files:**

- Create: `apps/web/src/lib/storage.ts`
- Create: `apps/web/src/lib/storage.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect, beforeEach } from "vitest";
import { getResults, setResults, clearResults, STORAGE_KEY } from "./storage";
import type { WalletScanResult } from "@/types";

const sample: WalletScanResult[] = [
  { source_file: "/a.dat", source_type: "bitcoin_core", keys: [] },
];

describe("storage", () => {
  beforeEach(() => sessionStorage.clear());

  it("returns null when nothing is stored", () => {
    expect(getResults()).toBeNull();
  });

  it("round-trips results through sessionStorage", () => {
    setResults(sample);
    expect(getResults()).toEqual(sample);
  });

  it("clearResults removes the entry", () => {
    setResults(sample);
    clearResults();
    expect(getResults()).toBeNull();
  });

  it("discards data with mismatched schema version", () => {
    sessionStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ version: "v0", data: sample }),
    );
    expect(getResults()).toBeNull();
  });

  it("discards malformed JSON", () => {
    sessionStorage.setItem(STORAGE_KEY, "not-json{");
    expect(getResults()).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm -C apps/web run test storage
```

Expected: FAIL — "Cannot find module './storage'".

- [ ] **Step 3: Implement `apps/web/src/lib/storage.ts`**

```ts
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
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test storage
```

Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/lib/storage.ts apps/web/src/lib/storage.test.ts
git commit -m "feat(web): add schema-versioned sessionStorage wrapper in lib/storage.ts"
```

---

### Task 1.3: `lib/clipboard.ts` — copy + auto-clear timer

**Files:**

- Create: `apps/web/src/lib/clipboard.ts`
- Create: `apps/web/src/lib/clipboard.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { copyWithAutoClear } from "./clipboard";

const writeText = vi.fn();
const readText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  readText.mockReset();
  Object.assign(navigator, {
    clipboard: { writeText, readText },
  });
  vi.useFakeTimers();
});

describe("copyWithAutoClear", () => {
  it("writes the value to the clipboard immediately", async () => {
    writeText.mockResolvedValue(undefined);
    await copyWithAutoClear("L1aBc", 20_000);
    expect(writeText).toHaveBeenCalledWith("L1aBc");
  });

  it("clears the clipboard after the delay if it still contains the value", async () => {
    writeText.mockResolvedValue(undefined);
    readText.mockResolvedValue("L1aBc");
    await copyWithAutoClear("L1aBc", 20_000);
    await vi.advanceTimersByTimeAsync(20_000);
    expect(writeText).toHaveBeenLastCalledWith("");
  });

  it("does not clear the clipboard if the user has overwritten it", async () => {
    writeText.mockResolvedValue(undefined);
    readText.mockResolvedValue("something-else");
    await copyWithAutoClear("L1aBc", 20_000);
    await vi.advanceTimersByTimeAsync(20_000);
    expect(writeText).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm -C apps/web run test clipboard
```

- [ ] **Step 3: Implement `apps/web/src/lib/clipboard.ts`**

```ts
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
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test clipboard
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/lib/clipboard.ts apps/web/src/lib/clipboard.test.ts
git commit -m "feat(web): add copyWithAutoClear helper in lib/clipboard.ts"
```

---

### Task 1.4: `lib/tauri.ts` — feature detection + invoke wrappers

**Files:**

- Create: `apps/web/src/lib/tauri.ts`
- Create: `apps/web/src/lib/tauri.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect, afterEach } from "vitest";
import { isTauri, isFsAccess } from "./tauri";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    showDirectoryPicker?: unknown;
  }
}

afterEach(() => {
  delete window.__TAURI_INTERNALS__;
  delete window.showDirectoryPicker;
});

describe("tauri detection", () => {
  it("isTauri is false when __TAURI_INTERNALS__ is absent", () => {
    expect(isTauri()).toBe(false);
  });

  it("isTauri is true when __TAURI_INTERNALS__ is present", () => {
    window.__TAURI_INTERNALS__ = {};
    expect(isTauri()).toBe(true);
  });
});

describe("fs-access detection", () => {
  it("is false when showDirectoryPicker is absent", () => {
    expect(isFsAccess()).toBe(false);
  });

  it("is true when showDirectoryPicker is a function", () => {
    window.showDirectoryPicker = () => {};
    expect(isFsAccess()).toBe(true);
  });

  it("is false when showDirectoryPicker is a non-function value", () => {
    window.showDirectoryPicker = "not-a-function";
    expect(isFsAccess()).toBe(false);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm -C apps/web run test tauri
```

- [ ] **Step 3: Implement `apps/web/src/lib/tauri.ts`**

```ts
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
```

- [ ] **Step 4: Add the Tauri JS API as a dependency**

```bash
pnpm -C apps/web add @tauri-apps/api@^2.0.0
```

- [ ] **Step 5: Run test, verify green**

```bash
pnpm -C apps/web run test tauri
```

Expected: 5 passed.

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/lib/tauri.ts apps/web/src/lib/tauri.test.ts \
  apps/web/package.json apps/web/pnpm-lock.yaml
git commit -m "feat(web): add Tauri / FS Access feature detection in lib/tauri.ts"
```

---

### Task 1.5: `lib/fs-access.ts` — directory walker

**Files:**

- Create: `apps/web/src/lib/fs-access.ts`
- Create: `apps/web/src/lib/fs-access.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from "vitest";
import { walkDirectory, WALLET_FILE_PATTERNS } from "./fs-access";

function fakeFile(name: string, size = 1000): File {
  return new File([new Uint8Array(size)], name);
}

function fakeFileHandle(name: string, size = 1000) {
  return {
    kind: "file" as const,
    name,
    async getFile() {
      return fakeFile(name, size);
    },
  };
}

function fakeDirHandle(
  name: string,
  entries: Array<
    ReturnType<typeof fakeFileHandle> | ReturnType<typeof fakeDirHandle>
  >,
) {
  return {
    kind: "directory" as const,
    name,
    async *values() {
      for (const e of entries) yield e;
    },
  };
}

describe("walkDirectory", () => {
  it("finds wallet files by extension", async () => {
    const dir = fakeDirHandle("root", [
      fakeFileHandle("wallet.dat"),
      fakeFileHandle("readme.txt"),
      fakeFileHandle("backup.wallet"),
      fakeFileHandle("image.jpg"),
    ]);
    const files = await walkDirectory(
      dir as unknown as FileSystemDirectoryHandle,
    );
    expect(files.map((f) => f.name).sort()).toEqual([
      "backup.wallet",
      "readme.txt",
      "wallet.dat",
    ]);
  });

  it("recurses into subdirectories", async () => {
    const dir = fakeDirHandle("root", [
      fakeDirHandle("sub", [fakeFileHandle("inner.dat")]),
      fakeFileHandle("top.wallet"),
    ]);
    const files = await walkDirectory(
      dir as unknown as FileSystemDirectoryHandle,
    );
    expect(files.map((f) => f.name).sort()).toEqual([
      "inner.dat",
      "top.wallet",
    ]);
  });

  it("skips files larger than the limit", async () => {
    const dir = fakeDirHandle("root", [
      fakeFileHandle("big.dat", 60 * 1024 * 1024),
      fakeFileHandle("small.dat", 1024),
    ]);
    const files = await walkDirectory(
      dir as unknown as FileSystemDirectoryHandle,
      { maxFileBytes: 50 * 1024 * 1024 },
    );
    expect(files.map((f) => f.name)).toEqual(["small.dat"]);
  });

  it("exposes WALLET_FILE_PATTERNS as a source of truth", () => {
    expect(WALLET_FILE_PATTERNS).toContain(".dat");
    expect(WALLET_FILE_PATTERNS).toContain(".wallet");
    expect(WALLET_FILE_PATTERNS).toContain(".aes.json");
    expect(WALLET_FILE_PATTERNS).toContain(".txt");
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test fs-access
```

- [ ] **Step 3: Implement `apps/web/src/lib/fs-access.ts`**

```ts
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
  // @ts-expect-error values() is iterable on FileSystemDirectoryHandle but TS lib is behind
  for await (const entry of dir.values()) {
    if (entry.kind === "directory") {
      await visit(entry, out, maxBytes);
    } else if (hasWalletExt(entry.name)) {
      const file = await entry.getFile();
      if (file.size <= maxBytes) out.push(file);
    }
  }
}
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test fs-access
```

Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/lib/fs-access.ts apps/web/src/lib/fs-access.test.ts
git commit -m "feat(web): add File System Access API directory walker"
```

---

### Task 1.6: `lib/api.ts` — Tauri/HTTP adapter

**Files:**

- Rewrite: `apps/web/src/lib/api.ts`
- Modify: `apps/web/src/types.ts`
- Create: `apps/web/src/lib/api.test.ts`

- [ ] **Step 1: Extend types**

Append to `apps/web/src/types.ts`:

```ts
export interface Tx {
  txid: string;
  time: number;
  value_sat: number;
  fee_sat: number | null;
  confirmations: number | null;
  block_height: number | null;
}

export type Provider = "blockstream" | "blockchain" | "mock" | "none";
```

Also change the `TabId` type — remove `"extract"`:

```ts
export type TabId = "scan" | "mnemonic" | "results";
```

- [ ] **Step 2: Write the failing test**

Create `apps/web/src/lib/api.test.ts`:

```ts
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
```

- [ ] **Step 3: Run test, verify fails**

```bash
pnpm -C apps/web run test api
```

Expected: FAIL — `addressTransactions` not exported.

- [ ] **Step 4: Rewrite `apps/web/src/lib/api.ts`**

```ts
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
```

- [ ] **Step 5: Run test + typecheck clean**

```bash
pnpm -C apps/web run test api
pnpm -C apps/web run typecheck
```

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/lib/api.ts apps/web/src/types.ts apps/web/src/lib/api.test.ts
git commit -m "feat(web): rewrite lib/api.ts with Tauri/HTTP adapter and Tx types"
```

---

## Phase 2 — UI primitives

### Task 2.1: Button

**Files:**

- Create: `apps/web/src/components/ui/Button.tsx`
- Create: `apps/web/src/components/ui/Button.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Button } from "./Button";

describe("Button", () => {
  it("renders children and fires onClick", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Scan</Button>);
    await userEvent.click(screen.getByRole("button", { name: "Scan" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("does not fire onClick when disabled", async () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Scan
      </Button>,
    );
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).not.toHaveBeenCalled();
  });

  it("applies the primary variant class by default", () => {
    const { container } = render(<Button>Scan</Button>);
    expect(container.firstChild).toHaveClass("bg-[var(--color-accent)]");
  });

  it("applies the secondary variant class", () => {
    const { container } = render(<Button variant="secondary">Cancel</Button>);
    expect(container.firstChild).toHaveClass("border-[var(--color-border)]");
  });

  it("applies the success variant class", () => {
    const { container } = render(<Button variant="success">Import</Button>);
    expect(container.firstChild).toHaveClass("bg-[var(--color-success)]");
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test Button
```

- [ ] **Step 3: Implement `apps/web/src/components/ui/Button.tsx`**

```tsx
import type { ComponentProps } from "react";
import { cn } from "@/lib/utils";

type Variant = "primary" | "secondary" | "success" | "ghost";

interface Props extends ComponentProps<"button"> {
  variant?: Variant;
}

const base =
  "inline-flex items-center justify-center gap-2 rounded-[5px] px-3 py-1.5 " +
  "text-sm font-medium transition-colors " +
  "disabled:opacity-50 disabled:cursor-not-allowed";

const variants: Record<Variant, string> = {
  primary:
    "bg-[var(--color-accent)] text-white hover:bg-[var(--color-accent-lit)]",
  secondary:
    "border border-[var(--color-border)] bg-[var(--color-surface)] " +
    "text-[var(--color-text)] hover:bg-[var(--color-border)]",
  success:
    "bg-[var(--color-success)] text-white hover:opacity-90",
  ghost:
    "text-[var(--color-text-dim)] hover:text-[var(--color-text)] hover:bg-[var(--color-surface)]",
};

export function Button({ variant = "primary", className, ...rest }: Props) {
  return (
    <button {...rest} className={cn(base, variants[variant], className)} />
  );
}
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test Button
```

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/ui/Button.tsx apps/web/src/components/ui/Button.test.tsx
git commit -m "feat(web): add Button primitive with 4 variants"
```

---

### Task 2.2: Input, Textarea, Select, Field (grouped)

**Files:**

- Create: `apps/web/src/components/ui/FormFields.tsx`
- Create: `apps/web/src/components/ui/FormFields.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Input, Textarea, Select, Field } from "./FormFields";

describe("Input", () => {
  it("forwards value and onChange", async () => {
    let captured = "";
    render(<Input value="" onChange={(e) => (captured = e.target.value)} />);
    await userEvent.type(screen.getByRole("textbox"), "hi");
    expect(captured).toBe("i");
  });
});

describe("Textarea", () => {
  it("forwards value and rows", () => {
    render(<Textarea defaultValue="hello" rows={4} />);
    const ta = screen.getByRole("textbox") as HTMLTextAreaElement;
    expect(ta.value).toBe("hello");
    expect(ta.rows).toBe(4);
  });
});

describe("Select", () => {
  it("renders children and forwards value", () => {
    render(
      <Select value="a" onChange={() => {}}>
        <option value="a">A</option>
        <option value="b">B</option>
      </Select>,
    );
    const sel = screen.getByRole("combobox") as HTMLSelectElement;
    expect(sel.value).toBe("a");
  });
});

describe("Field", () => {
  it("renders a label above children", () => {
    render(
      <Field label="Provider">
        <Input />
      </Field>,
    );
    expect(screen.getByText("Provider")).toBeInTheDocument();
    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test FormFields
```

- [ ] **Step 3: Implement `apps/web/src/components/ui/FormFields.tsx`**

```tsx
import type { ComponentProps, PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

const control =
  "w-full rounded-[5px] border border-[var(--color-border)] " +
  "bg-[var(--color-surface)] text-[var(--color-text)] " +
  "px-3 py-2 text-sm font-mono transition-colors " +
  "focus:outline-none focus:border-[var(--color-accent)] " +
  "placeholder:text-[var(--color-text-faint)]";

export function Input(props: ComponentProps<"input">) {
  return <input {...props} className={cn(control, props.className)} />;
}

export function Textarea(props: ComponentProps<"textarea">) {
  return (
    <textarea
      rows={3}
      {...props}
      className={cn(control, "resize-y min-h-[3rem]", props.className)}
    />
  );
}

export function Select(props: ComponentProps<"select">) {
  return (
    <select
      {...props}
      className={cn(
        control,
        "appearance-none pr-8 cursor-pointer font-sans",
        props.className,
      )}
    />
  );
}

export function FieldLabel({ children }: PropsWithChildren) {
  return (
    <span className="text-[10px] tracking-[0.08em] uppercase text-[var(--color-text-faint)] mb-1 block font-sans">
      {children}
    </span>
  );
}

export function Field({
  label,
  children,
  className,
}: PropsWithChildren<{ label: string; className?: string }>) {
  return (
    <label className={cn("flex flex-col", className)}>
      <FieldLabel>{label}</FieldLabel>
      {children}
    </label>
  );
}
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test FormFields
```

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/ui/FormFields.tsx apps/web/src/components/ui/FormFields.test.tsx
git commit -m "feat(web): add Input / Textarea / Select / Field primitives"
```

---

### Task 2.3: Card, Pill, StatBar, Spinner, Banner

**Files:**

- Create: `apps/web/src/components/ui/Card.tsx`
- Create: `apps/web/src/components/ui/Pill.tsx`
- Create: `apps/web/src/components/ui/StatBar.tsx`
- Create: `apps/web/src/components/ui/Spinner.tsx`
- Create: `apps/web/src/components/ui/Banner.tsx`
- Create: `apps/web/src/components/ui/LayoutPrimitives.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Card } from "./Card";
import { Pill } from "./Pill";
import { StatBar } from "./StatBar";
import { Spinner } from "./Spinner";
import { Banner } from "./Banner";

describe("Card", () => {
  it("renders children inside a bordered surface", () => {
    const { container } = render(<Card>body</Card>);
    expect(container.firstChild).toHaveClass("border");
    expect(screen.getByText("body")).toBeInTheDocument();
  });
});

describe("Pill", () => {
  it("renders text and applies the tone color class", () => {
    render(<Pill tone="accent">BIP84</Pill>);
    expect(screen.getByText("BIP84")).toHaveClass("text-[var(--color-accent)]");
  });

  it("defaults to neutral tone", () => {
    render(<Pill>x</Pill>);
    expect(screen.getByText("x")).toHaveClass("text-[var(--color-text-dim)]");
  });
});

describe("StatBar", () => {
  it("renders each stat with a label and value", () => {
    render(
      <StatBar
        stats={[
          { label: "Wallets", value: "4" },
          { label: "Recovered", value: "0.038 BTC", hit: true },
        ]}
      />,
    );
    expect(screen.getByText("Wallets")).toBeInTheDocument();
    expect(screen.getByText("0.038 BTC")).toHaveClass(
      "text-[var(--color-success)]",
    );
  });
});

describe("Spinner", () => {
  it("renders a role=status element with an aria label", () => {
    render(<Spinner label="Scanning" />);
    expect(screen.getByRole("status")).toHaveAccessibleName("Scanning");
  });
});

describe("Banner", () => {
  it("renders the warn variant", () => {
    const { container } = render(<Banner variant="warn">heads up</Banner>);
    expect(container.firstChild).toHaveClass("border-[var(--color-warn)]");
  });

  it("renders the danger variant", () => {
    const { container } = render(<Banner variant="danger">oh no</Banner>);
    expect(container.firstChild).toHaveClass("border-[var(--color-danger)]");
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test LayoutPrimitives
```

- [ ] **Step 3: Implement Card**

```tsx
// apps/web/src/components/ui/Card.tsx
import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

export function Card({
  children,
  className,
}: PropsWithChildren<{ className?: string }>) {
  return (
    <div
      className={cn(
        "border border-[var(--color-border)] bg-[var(--color-surface)] " +
          "rounded-[6px] p-4",
        className,
      )}
    >
      {children}
    </div>
  );
}
```

- [ ] **Step 4: Implement Pill**

```tsx
// apps/web/src/components/ui/Pill.tsx
import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

type Tone = "neutral" | "accent" | "success" | "warn" | "danger";

const tones: Record<Tone, string> = {
  neutral: "text-[var(--color-text-dim)] bg-[var(--color-surface)]",
  accent:
    "text-[var(--color-accent)] bg-[color-mix(in_srgb,var(--color-accent)_12%,transparent)]",
  success:
    "text-[var(--color-success)] bg-[color-mix(in_srgb,var(--color-success)_12%,transparent)]",
  warn: "text-[var(--color-warn)] bg-[color-mix(in_srgb,var(--color-warn)_12%,transparent)]",
  danger:
    "text-[var(--color-danger)] bg-[color-mix(in_srgb,var(--color-danger)_12%,transparent)]",
};

export function Pill({
  tone = "neutral",
  className,
  children,
}: PropsWithChildren<{ tone?: Tone; className?: string }>) {
  return (
    <span
      className={cn(
        "inline-block px-1.5 py-0.5 rounded-[3px] text-[10px] font-medium tracking-wide",
        tones[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}
```

- [ ] **Step 5: Implement StatBar**

```tsx
// apps/web/src/components/ui/StatBar.tsx
import { cn } from "@/lib/utils";

export interface Stat {
  label: string;
  value: string;
  hit?: boolean;
}

export function StatBar({
  stats,
  className,
}: {
  stats: Stat[];
  className?: string;
}) {
  return (
    <div
      className={cn(
        "grid grid-cols-2 md:grid-cols-4 gap-3 " +
          "border border-[var(--color-border)] bg-[var(--color-surface)] " +
          "rounded-[6px] px-4 py-3",
        className,
      )}
    >
      {stats.map((s) => (
        <div key={s.label} className="flex flex-col">
          <span className="text-[10px] uppercase tracking-[0.08em] text-[var(--color-text-faint)]">
            {s.label}
          </span>
          <span
            className={cn(
              "text-base font-semibold tabular-nums",
              s.hit
                ? "text-[var(--color-success)]"
                : "text-[var(--color-text)]",
            )}
          >
            {s.value}
          </span>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 6: Implement Spinner**

```tsx
// apps/web/src/components/ui/Spinner.tsx
import { cn } from "@/lib/utils";

export function Spinner({
  label = "Loading",
  className,
}: {
  label?: string;
  className?: string;
}) {
  return (
    <span
      role="status"
      aria-label={label}
      className={cn(
        "inline-block h-4 w-4 border-2 border-[var(--color-border)] " +
          "border-t-[var(--color-accent)] rounded-full animate-spin",
        className,
      )}
    />
  );
}
```

- [ ] **Step 7: Implement Banner**

```tsx
// apps/web/src/components/ui/Banner.tsx
import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

type Variant = "info" | "warn" | "danger";

const variants: Record<Variant, string> = {
  info: "border-[var(--color-accent)] bg-[color-mix(in_srgb,var(--color-accent)_8%,transparent)]",
  warn: "border-[var(--color-warn)] bg-[color-mix(in_srgb,var(--color-warn)_8%,transparent)]",
  danger:
    "border-[var(--color-danger)] bg-[color-mix(in_srgb,var(--color-danger)_8%,transparent)]",
};

export function Banner({
  variant = "info",
  className,
  children,
}: PropsWithChildren<{ variant?: Variant; className?: string }>) {
  return (
    <div
      role="note"
      className={cn(
        "border rounded-[5px] px-4 py-3 text-sm text-[var(--color-text)]",
        variants[variant],
        className,
      )}
    >
      {children}
    </div>
  );
}
```

- [ ] **Step 8: Run tests, verify green**

```bash
pnpm -C apps/web run test LayoutPrimitives
```

Expected: 6 passed.

- [ ] **Step 9: Commit**

```bash
git add apps/web/src/components/ui/Card.tsx \
  apps/web/src/components/ui/Pill.tsx \
  apps/web/src/components/ui/StatBar.tsx \
  apps/web/src/components/ui/Spinner.tsx \
  apps/web/src/components/ui/Banner.tsx \
  apps/web/src/components/ui/LayoutPrimitives.test.tsx
git commit -m "feat(web): add Card, Pill, StatBar, Spinner, Banner primitives"
```

---

### Task 2.4: CopyButton

**Files:**

- Create: `apps/web/src/components/ui/CopyButton.tsx`
- Create: `apps/web/src/components/ui/CopyButton.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CopyButton } from "./CopyButton";

const writeText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  writeText.mockResolvedValue(undefined);
  Object.assign(navigator, { clipboard: { writeText } });
});

describe("CopyButton", () => {
  it("writes the value to the clipboard on click", async () => {
    render(<CopyButton value="bc1qabc" />);
    await userEvent.click(screen.getByRole("button"));
    expect(writeText).toHaveBeenCalledWith("bc1qabc");
  });

  it("shows the 'copied' label briefly after click", async () => {
    render(<CopyButton value="bc1qabc" />);
    await userEvent.click(screen.getByRole("button"));
    expect(screen.getByRole("button")).toHaveAccessibleName(/copied/i);
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test CopyButton
```

- [ ] **Step 3: Implement**

```tsx
// apps/web/src/components/ui/CopyButton.tsx
import { useState } from "react";
import { Button } from "./Button";

export function CopyButton({
  value,
  label = "Copy",
  className,
}: {
  value: string;
  label?: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);

  const onClick = async () => {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <Button
      variant="secondary"
      onClick={onClick}
      aria-label={copied ? "Copied" : label}
      className={className}
    >
      {copied ? "Copied" : label}
    </Button>
  );
}
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test CopyButton
```

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/ui/CopyButton.tsx apps/web/src/components/ui/CopyButton.test.tsx
git commit -m "feat(web): add CopyButton primitive"
```

---

### Task 2.5: WifReveal

**Files:**

- Create: `apps/web/src/components/ui/WifReveal.tsx`
- Create: `apps/web/src/components/ui/WifReveal.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WifReveal } from "./WifReveal";

const writeText = vi.fn();
const readText = vi.fn();

beforeEach(() => {
  writeText.mockReset();
  readText.mockReset();
  writeText.mockResolvedValue(undefined);
  Object.assign(navigator, { clipboard: { writeText, readText } });
});

describe("WifReveal", () => {
  it("hides the WIF by default", () => {
    render(<WifReveal wif="L1abcDEF" />);
    expect(screen.queryByText("L1abcDEF")).not.toBeInTheDocument();
  });

  it("has an accessible label for the hidden state", () => {
    render(<WifReveal wif="L1abcDEF" />);
    expect(screen.getByLabelText(/private key/i)).toBeInTheDocument();
  });

  it("reveals the WIF after clicking Reveal", async () => {
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    expect(screen.getByText("L1abcDEF")).toBeInTheDocument();
  });

  it("copies to clipboard when Copy is clicked after reveal", async () => {
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    await userEvent.click(screen.getByRole("button", { name: /copy/i }));
    expect(writeText).toHaveBeenCalledWith("L1abcDEF");
  });

  it("can be hidden again after revealing", async () => {
    render(<WifReveal wif="L1abcDEF" />);
    await userEvent.click(screen.getByRole("button", { name: /reveal/i }));
    await userEvent.click(screen.getByRole("button", { name: /hide/i }));
    expect(screen.queryByText("L1abcDEF")).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test WifReveal
```

- [ ] **Step 3: Implement**

```tsx
// apps/web/src/components/ui/WifReveal.tsx
import { useState } from "react";
import { Button } from "./Button";
import { copyWithAutoClear } from "@/lib/clipboard";

const CLEAR_AFTER_MS = 20_000;

export function WifReveal({ wif }: { wif: string }) {
  const [revealed, setRevealed] = useState(false);

  return (
    <div className="flex items-center gap-3 border border-[var(--color-danger)] rounded-[5px] px-3 py-2 bg-[color-mix(in_srgb,var(--color-danger)_5%,transparent)]">
      <span className="text-[var(--color-danger)] text-xs font-semibold tracking-wide">
        WIF
      </span>
      {revealed ? (
        <span className="font-mono text-xs text-[var(--color-text)] break-all flex-1">
          {wif}
        </span>
      ) : (
        <span
          aria-label="Private key (hidden — activate reveal to show)"
          className="font-mono text-xs text-[var(--color-text-faint)] tracking-[0.3em] flex-1"
        >
          ••••••••••••••••••••
        </span>
      )}
      {revealed ? (
        <>
          <Button
            variant="secondary"
            onClick={() => copyWithAutoClear(wif, CLEAR_AFTER_MS)}
          >
            Copy
          </Button>
          <Button variant="ghost" onClick={() => setRevealed(false)}>
            Hide
          </Button>
        </>
      ) : (
        <Button variant="secondary" onClick={() => setRevealed(true)}>
          Reveal
        </Button>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test WifReveal
```

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/ui/WifReveal.tsx apps/web/src/components/ui/WifReveal.test.tsx
git commit -m "feat(web): add WifReveal with auto-clear clipboard"
```

---

### Task 2.6: QRCode (safe JSX, no innerHTML)

**Files:**

- Create: `apps/web/src/components/ui/QRCode.tsx`
- Create: `apps/web/src/components/ui/QRCode.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { QRCode } from "./QRCode";

describe("QRCode", () => {
  it("renders an SVG with width and aria-label", () => {
    const { container, getByLabelText } = render(
      <QRCode value="bc1qxyz" size={128} />,
    );
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("width", "128");
    expect(getByLabelText(/bc1qxyz/)).toBeInTheDocument();
  });

  it("renders one <rect> per dark module", () => {
    const { container } = render(<QRCode value="x" size={100} />);
    const rects = container.querySelectorAll("rect");
    expect(rects.length).toBeGreaterThan(10);
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test QRCode
```

- [ ] **Step 3: Implement — use JSX, never `dangerouslySetInnerHTML`**

The `value` is a user-scoped address string. We compute dark-module positions (numbers only) via `qrcode-generator` and render them as JSX `<rect>` elements — the string itself only appears in `aria-label`, which React escapes automatically.

```tsx
// apps/web/src/components/ui/QRCode.tsx
import qrcode from "qrcode-generator";

export function QRCode({
  value,
  size = 128,
}: {
  value: string;
  size?: number;
}) {
  const qr = qrcode(0, "M");
  qr.addData(value);
  qr.make();

  const cells = qr.getModuleCount();
  const cellSize = size / cells;

  const rects: { x: number; y: number }[] = [];
  for (let y = 0; y < cells; y++) {
    for (let x = 0; x < cells; x++) {
      if (qr.isDark(y, x)) rects.push({ x, y });
    }
  }

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      role="img"
      aria-label={`QR code for ${value}`}
      className="text-[var(--color-text)] bg-white p-1 rounded"
    >
      {rects.map((r) => (
        <rect
          key={`${r.x}-${r.y}`}
          x={r.x * cellSize}
          y={r.y * cellSize}
          width={cellSize}
          height={cellSize}
          fill="currentColor"
        />
      ))}
    </svg>
  );
}
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test QRCode
```

Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/ui/QRCode.tsx apps/web/src/components/ui/QRCode.test.tsx
git commit -m "feat(web): add QRCode component using qrcode-generator (JSX rects)"
```

---

### Task 2.7: Table

**Files:**

- Create: `apps/web/src/components/ui/Table.tsx`
- Create: `apps/web/src/components/ui/Table.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Table, type Column } from "./Table";

interface Row {
  address: string;
  btc: number;
}

const columns: Column<Row>[] = [
  { key: "address", header: "Address" },
  {
    key: "btc",
    header: "BTC",
    align: "right",
    sortValue: (r) => r.btc,
    render: (r) => r.btc.toFixed(8),
  },
];

const rows: Row[] = [
  { address: "bc1qA", btc: 0.5 },
  { address: "bc1qB", btc: 1.2 },
  { address: "bc1qC", btc: 0.01 },
];

describe("Table", () => {
  it("renders rows and columns", () => {
    render(<Table columns={columns} rows={rows} rowKey={(r) => r.address} />);
    expect(screen.getAllByRole("row")).toHaveLength(4);
    expect(screen.getByText("bc1qA")).toBeInTheDocument();
  });

  it("sorts ascending by column when header is clicked once", async () => {
    render(<Table columns={columns} rows={rows} rowKey={(r) => r.address} />);
    await userEvent.click(screen.getByText("BTC"));
    const bodyRows = screen
      .getAllByRole("row")
      .slice(1)
      .map((r) => r.textContent);
    expect(bodyRows[0]).toContain("bc1qC");
    expect(bodyRows[2]).toContain("bc1qB");
  });

  it("sorts descending when header is clicked twice", async () => {
    render(<Table columns={columns} rows={rows} rowKey={(r) => r.address} />);
    await userEvent.click(screen.getByText("BTC"));
    await userEvent.click(screen.getByText("BTC"));
    const bodyRows = screen
      .getAllByRole("row")
      .slice(1)
      .map((r) => r.textContent);
    expect(bodyRows[0]).toContain("bc1qB");
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test Table
```

- [ ] **Step 3: Implement**

```tsx
// apps/web/src/components/ui/Table.tsx
import { useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface Column<T> {
  key: string;
  header: ReactNode;
  align?: "left" | "right";
  render?: (row: T) => ReactNode;
  sortValue?: (row: T) => number | string;
}

type SortDir = "asc" | "desc";

export function Table<T>({
  columns,
  rows,
  rowKey,
  onRowClick,
  className,
}: {
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T) => string;
  onRowClick?: (row: T) => void;
  className?: string;
}) {
  const [sortKey, setSortKey] = useState<string | null>(null);
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  const sortable = (col: Column<T>) => col.sortValue != null;

  const sorted = (() => {
    if (!sortKey) return rows;
    const col = columns.find((c) => c.key === sortKey);
    if (!col?.sortValue) return rows;
    const get = col.sortValue;
    return [...rows].sort((a, b) => {
      const av = get(a);
      const bv = get(b);
      if (av < bv) return sortDir === "asc" ? -1 : 1;
      if (av > bv) return sortDir === "asc" ? 1 : -1;
      return 0;
    });
  })();

  const toggleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
  };

  return (
    <table className={cn("w-full text-sm border-collapse", className)}>
      <thead>
        <tr>
          {columns.map((c) => (
            <th
              key={c.key}
              scope="col"
              aria-sort={
                sortKey === c.key
                  ? sortDir === "asc"
                    ? "ascending"
                    : "descending"
                  : undefined
              }
              onClick={sortable(c) ? () => toggleSort(c.key) : undefined}
              className={cn(
                "text-[10px] uppercase tracking-[0.08em] font-normal " +
                  "text-[var(--color-text-faint)] border-b border-[var(--color-border)] " +
                  "py-2 px-2",
                c.align === "right" ? "text-right" : "text-left",
                sortable(c) &&
                  "cursor-pointer select-none hover:text-[var(--color-text)]",
              )}
            >
              {c.header}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((r) => (
          <tr
            key={rowKey(r)}
            onClick={onRowClick ? () => onRowClick(r) : undefined}
            className={cn(
              "border-b border-[var(--color-border)]",
              onRowClick && "cursor-pointer hover:bg-[var(--color-surface)]",
            )}
          >
            {columns.map((c) => (
              <td
                key={c.key}
                className={cn(
                  "py-2 px-2 tabular-nums",
                  c.align === "right" ? "text-right" : "text-left",
                )}
              >
                {c.render
                  ? c.render(r)
                  : ((r as Record<string, unknown>)[c.key] as ReactNode)}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

- [ ] **Step 4: Run test, verify green**

```bash
pnpm -C apps/web run test Table
```

Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/ui/Table.tsx apps/web/src/components/ui/Table.test.tsx
git commit -m "feat(web): add Table with click-to-sort columns"
```

---

## Phase 3 — Layout & routing

### Task 3.1: Router skeleton + route placeholders

**Files:**

- Rewrite: `apps/web/src/App.tsx`
- Modify: `apps/web/src/main.tsx`
- Create: `apps/web/src/routes/ScanPage.tsx`
- Create: `apps/web/src/routes/MnemonicPage.tsx`
- Create: `apps/web/src/routes/ResultsPage.tsx`
- Create: `apps/web/src/routes/WalletPage.tsx`
- Create: `apps/web/src/routes/AddressPage.tsx`
- Create: `apps/web/src/routes/ImportPage.tsx`

- [ ] **Step 1: Create empty route stubs**

Each stub is the same shape (rename the component per file):

```tsx
// apps/web/src/routes/ScanPage.tsx
export default function ScanPage() {
  return <div>Scan — coming soon</div>;
}
```

Repeat for `MnemonicPage`, `ResultsPage`, `WalletPage`, `AddressPage`, `ImportPage`.

- [ ] **Step 2: Rewrite `apps/web/src/App.tsx`**

```tsx
import { Navigate, Route, Routes } from "react-router-dom";
import ScanPage from "@/routes/ScanPage";
import MnemonicPage from "@/routes/MnemonicPage";
import ResultsPage from "@/routes/ResultsPage";
import WalletPage from "@/routes/WalletPage";
import AddressPage from "@/routes/AddressPage";
import ImportPage from "@/routes/ImportPage";

export default function App() {
  return (
    <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)]">
      <Routes>
        <Route path="/" element={<Navigate to="/scan" replace />} />
        <Route path="/scan" element={<ScanPage />} />
        <Route path="/mnemonic" element={<MnemonicPage />} />
        <Route path="/results" element={<ResultsPage />} />
        <Route path="/results/:walletId" element={<WalletPage />} />
        <Route path="/results/:walletId/:address" element={<AddressPage />} />
        <Route path="/import/:keyId" element={<ImportPage />} />
        <Route path="*" element={<Navigate to="/scan" replace />} />
      </Routes>
    </div>
  );
}
```

- [ ] **Step 3: Wrap in `BrowserRouter` in `apps/web/src/main.tsx`**

```tsx
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import "./index.css";
import App from "./App";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </StrictMode>,
);
```

- [ ] **Step 4: Smoke test + typecheck**

```bash
pnpm -C apps/web run typecheck
mise run web:dev
```

Visit `/scan`, `/mnemonic`, `/results`. Each shows its placeholder. Stop the server.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/App.tsx apps/web/src/main.tsx apps/web/src/routes/
git commit -m "feat(web): add react-router skeleton with 6 route stubs"
```

---

### Task 3.2: Header with provider indicator

**Files:**

- Create: `apps/web/src/components/layout/Header.tsx`
- Create: `apps/web/src/components/layout/Header.test.tsx`
- Modify: `apps/web/src/App.tsx`
- Modify: `apps/web/src/routes/ScanPage.tsx`
- Modify: `apps/web/src/routes/AddressPage.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Header } from "./Header";

describe("Header", () => {
  it("renders the brand, tabs, and provider indicator", () => {
    render(
      <MemoryRouter initialEntries={["/scan"]}>
        <Header provider="blockstream" onProviderChange={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByText("Orpheus")).toBeInTheDocument();
    expect(screen.getByText("Wallet files")).toBeInTheDocument();
    expect(screen.getByText("Mnemonic")).toBeInTheDocument();
    expect(screen.getByText("Results")).toBeInTheDocument();
  });

  it("shows an amber dot for the non-default network provider", () => {
    render(
      <MemoryRouter initialEntries={["/scan"]}>
        <Header provider="blockchain" onProviderChange={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByTestId("provider-dot")).toHaveClass(
      "bg-[var(--color-warn)]",
    );
  });

  it("shows a grey 'offline' option for mock/none providers", () => {
    render(
      <MemoryRouter initialEntries={["/scan"]}>
        <Header provider="none" onProviderChange={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByText(/offline/i)).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test Header
```

- [ ] **Step 3: Implement Header**

```tsx
// apps/web/src/components/layout/Header.tsx
import { NavLink } from "react-router-dom";
import type { Provider } from "@/types";
import { cn } from "@/lib/utils";

const TABS = [
  { to: "/scan", label: "Wallet files" },
  { to: "/mnemonic", label: "Mnemonic" },
  { to: "/results", label: "Results" },
];

const PROVIDER_LABEL: Record<Provider, string> = {
  blockstream: "blockstream.info",
  blockchain: "blockchain.info",
  mock: "offline (mock)",
  none: "offline",
};

function providerTone(p: Provider): "success" | "warn" | "neutral" {
  if (p === "blockstream") return "success";
  if (p === "blockchain") return "warn";
  return "neutral";
}

const dotClass: Record<"success" | "warn" | "neutral", string> = {
  success: "bg-[var(--color-success)]",
  warn: "bg-[var(--color-warn)]",
  neutral: "bg-[var(--color-text-faint)]",
};

export function Header({
  provider,
  onProviderChange,
}: {
  provider: Provider;
  onProviderChange: (p: Provider) => void;
}) {
  const tone = providerTone(provider);
  return (
    <header className="border-b border-[var(--color-border)] bg-[var(--color-bg)]">
      <div className="max-w-[1200px] mx-auto px-6 py-3 flex items-center gap-6">
        <NavLink to="/" className="flex items-center gap-2">
          <span className="w-5 h-5 rounded bg-gradient-to-br from-[var(--color-accent)] to-[#8b5cf6]" />
          <span className="font-semibold text-[var(--color-text)]">
            Orpheus
          </span>
        </NavLink>
        <nav className="flex gap-5 ml-4">
          {TABS.map((t) => (
            <NavLink
              key={t.to}
              to={t.to}
              className={({ isActive }) =>
                cn(
                  "text-sm py-1 border-b-2 border-transparent",
                  isActive
                    ? "text-[var(--color-text)] border-[var(--color-accent)]"
                    : "text-[var(--color-text-dim)] hover:text-[var(--color-text)]",
                )
              }
            >
              {t.label}
            </NavLink>
          ))}
        </nav>
        <label className="ml-auto flex items-center gap-2 text-xs text-[var(--color-text-dim)]">
          <span
            data-testid="provider-dot"
            className={cn("w-1.5 h-1.5 rounded-full", dotClass[tone])}
          />
          <select
            value={provider}
            onChange={(e) => onProviderChange(e.target.value as Provider)}
            className="bg-transparent outline-none cursor-pointer text-[var(--color-text)]"
            aria-label="Balance provider"
          >
            <option value="blockstream">{PROVIDER_LABEL.blockstream}</option>
            <option value="blockchain">{PROVIDER_LABEL.blockchain}</option>
            <option value="mock">{PROVIDER_LABEL.mock}</option>
            <option value="none">{PROVIDER_LABEL.none}</option>
          </select>
        </label>
      </div>
    </header>
  );
}
```

- [ ] **Step 4: Wire Header into App + pass `provider` to Scan and Address pages**

Rewrite `apps/web/src/App.tsx`:

```tsx
import { useState } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { Header } from "@/components/layout/Header";
import type { Provider } from "@/types";
import ScanPage from "@/routes/ScanPage";
import MnemonicPage from "@/routes/MnemonicPage";
import ResultsPage from "@/routes/ResultsPage";
import WalletPage from "@/routes/WalletPage";
import AddressPage from "@/routes/AddressPage";
import ImportPage from "@/routes/ImportPage";

export default function App() {
  const [provider, setProvider] = useState<Provider>("blockstream");
  return (
    <div className="min-h-screen bg-[var(--color-bg)] text-[var(--color-text)]">
      <Header provider={provider} onProviderChange={setProvider} />
      <main className="max-w-[1200px] mx-auto px-6 py-6">
        <Routes>
          <Route path="/" element={<Navigate to="/scan" replace />} />
          <Route path="/scan" element={<ScanPage provider={provider} />} />
          <Route path="/mnemonic" element={<MnemonicPage />} />
          <Route path="/results" element={<ResultsPage />} />
          <Route path="/results/:walletId" element={<WalletPage />} />
          <Route
            path="/results/:walletId/:address"
            element={<AddressPage provider={provider} />}
          />
          <Route path="/import/:keyId" element={<ImportPage />} />
          <Route path="*" element={<Navigate to="/scan" replace />} />
        </Routes>
      </main>
    </div>
  );
}
```

Update `ScanPage.tsx` stub to accept `{ provider }`:

```tsx
import type { Provider } from "@/types";
export default function ScanPage({ provider }: { provider: Provider }) {
  return <div>Scan — coming soon (provider={provider})</div>;
}
```

Same for `AddressPage.tsx`:

```tsx
import type { Provider } from "@/types";
export default function AddressPage({ provider }: { provider: Provider }) {
  return <div>Address — coming soon (provider={provider})</div>;
}
```

- [ ] **Step 5: Run test + typecheck**

```bash
pnpm -C apps/web run test Header
pnpm -C apps/web run typecheck
```

- [ ] **Step 6: Commit**

```bash
git add apps/web/src/components/layout/Header.tsx \
  apps/web/src/components/layout/Header.test.tsx \
  apps/web/src/App.tsx apps/web/src/routes/
git commit -m "feat(web): add Header with active-provider indicator"
```

---

### Task 3.3: Breadcrumbs

**Files:**

- Create: `apps/web/src/components/layout/Breadcrumbs.tsx`
- Create: `apps/web/src/components/layout/Breadcrumbs.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Breadcrumbs } from "./Breadcrumbs";

describe("Breadcrumbs", () => {
  it("renders all segments", () => {
    render(
      <MemoryRouter>
        <Breadcrumbs
          segments={[
            { label: "Results", to: "/results" },
            { label: "wallet.dat", to: "/results/abc" },
            { label: "bc1q…kp8z" },
          ]}
        />
      </MemoryRouter>,
    );
    expect(screen.getByText("Results")).toBeInTheDocument();
    expect(screen.getByText("wallet.dat")).toBeInTheDocument();
    expect(screen.getByText("bc1q…kp8z")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test Breadcrumbs
```

- [ ] **Step 3: Implement**

```tsx
// apps/web/src/components/layout/Breadcrumbs.tsx
import { Fragment } from "react";
import { NavLink } from "react-router-dom";

export interface Crumb {
  label: string;
  to?: string;
}

export function Breadcrumbs({ segments }: { segments: Crumb[] }) {
  return (
    <nav
      aria-label="Breadcrumb"
      className="text-xs text-[var(--color-text-dim)] mb-4"
    >
      {segments.map((s, i) => {
        const last = i === segments.length - 1;
        return (
          <Fragment key={`${s.label}-${i}`}>
            {i > 0 && (
              <span className="mx-2 text-[var(--color-text-faint)]">/</span>
            )}
            {last || !s.to ? (
              <span className="text-[var(--color-text)]">{s.label}</span>
            ) : (
              <NavLink to={s.to} className="hover:text-[var(--color-text)]">
                {s.label}
              </NavLink>
            )}
          </Fragment>
        );
      })}
    </nav>
  );
}
```

- [ ] **Step 4: Run test, verify green + commit**

```bash
pnpm -C apps/web run test Breadcrumbs
git add apps/web/src/components/layout/Breadcrumbs.tsx \
  apps/web/src/components/layout/Breadcrumbs.test.tsx
git commit -m "feat(web): add Breadcrumbs component"
```

---

## Phase 4 — Scan page

### Task 4.1: ScanPage + FolderPicker + FileDropzone

**Files:**

- Rewrite: `apps/web/src/routes/ScanPage.tsx`
- Create: `apps/web/src/components/scan/FolderPicker.tsx`
- Create: `apps/web/src/components/scan/FolderPicker.test.tsx`
- Create: `apps/web/src/components/scan/FileDropzone.tsx`

- [ ] **Step 1: Write a failing test for FolderPicker**

```tsx
import { describe, it, expect, afterEach } from "vitest";
import { render, screen } from "@testing-library/react";
import { FolderPicker } from "./FolderPicker";

afterEach(() => {
  delete (window as unknown as { showDirectoryPicker?: unknown })
    .showDirectoryPicker;
});

describe("FolderPicker", () => {
  it("shows 'Pick a folder' when FS Access is available", () => {
    (window as unknown as { showDirectoryPicker: () => void }).showDirectoryPicker =
      () => {};
    render(<FolderPicker onFiles={() => {}} />);
    expect(
      screen.getByRole("button", { name: /pick a folder/i }),
    ).toBeInTheDocument();
  });

  it("renders nothing when FS Access is absent (and not Tauri)", () => {
    const { container } = render(<FolderPicker onFiles={() => {}} />);
    expect(container.firstChild).toBeNull();
  });
});
```

- [ ] **Step 2: Run test, verify fails**

```bash
pnpm -C apps/web run test FolderPicker
```

- [ ] **Step 3: Implement FileDropzone**

```tsx
// apps/web/src/components/scan/FileDropzone.tsx
import { useRef, useState } from "react";
import { cn, formatBytes } from "@/lib/utils";

export function FileDropzone({
  files,
  onChange,
  multiple = true,
}: {
  files: File[];
  onChange: (files: File[]) => void;
  multiple?: boolean;
}) {
  const ref = useRef<HTMLInputElement>(null);
  const [dragOver, setDragOver] = useState(false);

  return (
    <div
      onClick={() => ref.current?.click()}
      onDragOver={(e) => {
        e.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragOver(false);
        const dropped = Array.from(e.dataTransfer.files);
        onChange(multiple ? dropped : dropped.slice(0, 1));
      }}
      className={cn(
        "border border-dashed rounded-[6px] px-4 py-6 text-center cursor-pointer transition-colors",
        dragOver
          ? "border-[var(--color-accent)] bg-[color-mix(in_srgb,var(--color-accent)_8%,transparent)]"
          : "border-[var(--color-border)] hover:border-[var(--color-accent)]",
      )}
    >
      <p className="text-sm text-[var(--color-text)] m-0">
        Drop wallet files here
      </p>
      <p className="text-xs text-[var(--color-text-faint)] m-0 mt-1">
        or click to select
      </p>
      <input
        ref={ref}
        type="file"
        multiple={multiple}
        className="hidden"
        onChange={(e) => {
          const list = Array.from(e.target.files ?? []);
          onChange(multiple ? list : list.slice(0, 1));
        }}
      />
      {files.length > 0 && (
        <ul className="mt-3 text-left text-xs font-mono text-[var(--color-text-dim)]">
          {files.map((f) => (
            <li key={f.name}>
              {f.name}{" "}
              <span className="text-[var(--color-text-faint)]">
                · {formatBytes(f.size)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Implement FolderPicker**

```tsx
// apps/web/src/components/scan/FolderPicker.tsx
import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { walkDirectory } from "@/lib/fs-access";
import { isFsAccess, isTauri } from "@/lib/tauri";

export function FolderPicker({
  onFiles,
  onTauriPath,
}: {
  onFiles: (files: File[]) => void;
  onTauriPath?: (path: string) => void;
}) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const tauri = isTauri();
  const fsAccess = isFsAccess();

  if (!tauri && !fsAccess) return null;

  const pick = async () => {
    setErr(null);
    setBusy(true);
    try {
      if (tauri && onTauriPath) {
        const mod = await import("@tauri-apps/plugin-dialog");
        const chosen = await mod.open({ directory: true, multiple: false });
        if (typeof chosen === "string") onTauriPath(chosen);
      } else {
        const dir = await (
          window as unknown as {
            showDirectoryPicker: (o?: {
              startIn?: string;
            }) => Promise<FileSystemDirectoryHandle>;
          }
        ).showDirectoryPicker({ startIn: "documents" });
        const files = await walkDirectory(dir);
        onFiles(files);
      }
    } catch (e) {
      if ((e as DOMException).name !== "AbortError") {
        setErr((e as Error).message);
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div>
      <Button onClick={pick} disabled={busy}>
        {busy ? "Opening…" : tauri ? "Choose folder" : "Pick a folder"}
      </Button>
      {err && (
        <p className="text-xs text-[var(--color-danger)] mt-2">{err}</p>
      )}
    </div>
  );
}
```

- [ ] **Step 5: Install @tauri-apps/plugin-dialog** (for the import above — TS side)

```bash
pnpm -C apps/web add @tauri-apps/plugin-dialog@^2.0.0
```

- [ ] **Step 6: Rewrite ScanPage**

```tsx
// apps/web/src/routes/ScanPage.tsx
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Field, Textarea } from "@/components/ui/FormFields";
import { Button } from "@/components/ui/Button";
import { Banner } from "@/components/ui/Banner";
import { Spinner } from "@/components/ui/Spinner";
import { FolderPicker } from "@/components/scan/FolderPicker";
import { FileDropzone } from "@/components/scan/FileDropzone";
import { scan, scanDirectory } from "@/lib/api";
import { setResults } from "@/lib/storage";
import type { Provider } from "@/types";

export default function ScanPage({ provider }: { provider: Provider }) {
  const navigate = useNavigate();
  const [files, setFiles] = useState<File[]>([]);
  const [tauriPath, setTauriPath] = useState<string | null>(null);
  const [passwords, setPasswords] = useState("");
  const [showFallback, setShowFallback] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErr(null);
    if (!files.length && !tauriPath) {
      setErr("Choose a folder or select files first.");
      return;
    }
    setBusy(true);
    try {
      const reply = tauriPath
        ? await scanDirectory(tauriPath, passwords, provider)
        : await scan(files, passwords, provider);
      setResults(reply.results);
      navigate("/results");
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-6 max-w-[720px]">
      <header>
        <h1 className="text-xl font-semibold m-0">Scan wallet files</h1>
        <p className="text-sm text-[var(--color-text-dim)] mt-1">
          Point Orpheus at a folder; it dispatches each file to the matching
          extractor and resolves balances with your chosen provider.
        </p>
      </header>

      <FolderPicker onFiles={setFiles} onTauriPath={setTauriPath} />
      {tauriPath && (
        <p className="text-xs font-mono text-[var(--color-text-dim)]">
          {tauriPath}
        </p>
      )}
      {files.length > 0 && (
        <p className="text-xs text-[var(--color-text-dim)]">
          {files.length} file(s) ready to scan.
        </p>
      )}

      <button
        type="button"
        onClick={() => setShowFallback((s) => !s)}
        className="text-xs text-[var(--color-accent)] hover:underline text-left w-fit"
      >
        Or select individual files →
      </button>

      {showFallback && <FileDropzone files={files} onChange={setFiles} />}

      <Field label="Passwords (one per line, optional)">
        <Textarea
          value={passwords}
          onChange={(e) => setPasswords(e.target.value)}
          placeholder={"orpheus-demo\nmy-old-passphrase"}
        />
      </Field>

      <details className="text-xs text-[var(--color-text-faint)]">
        <summary className="cursor-pointer">Common wallet locations</summary>
        <ul className="mt-2 font-mono space-y-1">
          <li>macOS: ~/Library/Application Support/Bitcoin/wallets/</li>
          <li>Linux: ~/.bitcoin/wallets/</li>
          <li>Windows: %APPDATA%\Bitcoin\wallets\</li>
          <li>Also: ~/Documents for legacy .aes.json, .wallet files</li>
        </ul>
      </details>

      {err && <Banner variant="danger">{err}</Banner>}

      <div className="flex items-center gap-4">
        <Button type="submit" disabled={busy}>
          {busy ? "Scanning…" : "Scan"}
        </Button>
        {busy && <Spinner label="Scanning" />}
      </div>
    </form>
  );
}
```

- [ ] **Step 7: Run test + typecheck**

```bash
pnpm -C apps/web run test FolderPicker
pnpm -C apps/web run typecheck
```

- [ ] **Step 8: Commit**

```bash
git add apps/web/src/routes/ScanPage.tsx \
  apps/web/src/components/scan/FolderPicker.tsx \
  apps/web/src/components/scan/FolderPicker.test.tsx \
  apps/web/src/components/scan/FileDropzone.tsx \
  apps/web/package.json apps/web/pnpm-lock.yaml
git commit -m "feat(web): ScanPage with FolderPicker-first / dropzone fallback"
```

---

## Phase 5 — Mnemonic page

### Task 5.1: MnemonicPage

**Files:**

- Rewrite: `apps/web/src/routes/MnemonicPage.tsx`

- [ ] **Step 1: Implement MnemonicPage**

```tsx
// apps/web/src/routes/MnemonicPage.tsx
import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Field, Input, Select, Textarea } from "@/components/ui/FormFields";
import { Button } from "@/components/ui/Button";
import { Banner } from "@/components/ui/Banner";
import { Card } from "@/components/ui/Card";
import { CopyButton } from "@/components/ui/CopyButton";
import { Spinner } from "@/components/ui/Spinner";
import { mnemonic } from "@/lib/api";
import { setResults } from "@/lib/storage";
import type { DecodedMnemonic } from "@/types";

type Kind = "bip39" | "blockchain";

export default function MnemonicPage() {
  const navigate = useNavigate();
  const [phrase, setPhrase] = useState("");
  const [kind, setKind] = useState<Kind>("bip39");
  const [passphrase, setPassphrase] = useState("");
  const [gapLimit, setGapLimit] = useState(20);
  const [wordlist, setWordlist] = useState("");
  const [decoded, setDecoded] = useState<DecodedMnemonic | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const words = phrase.trim().split(/\s+/).filter(Boolean).length;
  const wordsOk =
    kind === "bip39" ? [12, 15, 18, 21, 24].includes(words) : words > 0;

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErr(null);
    setDecoded(null);
    if (!phrase.trim()) {
      setErr("Paste a phrase first.");
      return;
    }
    setBusy(true);
    try {
      const body = await mnemonic({
        phrase: phrase.trim(),
        kind,
        passphrase,
        gap_limit: gapLimit,
        wordlist: wordlist.trim() || undefined,
      });
      if (body.decoded) {
        setDecoded(body.decoded);
      } else if (body.keys) {
        setResults([
          {
            source_file: "(mnemonic)",
            source_type: "bip39",
            keys: body.keys,
          },
        ]);
        navigate("/results");
      }
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-6 max-w-[720px]">
      <header>
        <h1 className="text-xl font-semibold m-0">
          Derive keys from a mnemonic
        </h1>
        <p className="text-sm text-[var(--color-text-dim)] mt-1">
          BIP39 phrases derive BIP44 / 49 / 84 and the Breadwallet path. Legacy
          blockchain.com mnemonics decode to a wallet password.
        </p>
      </header>

      <div className="grid grid-cols-1 md:grid-cols-[1fr_140px] gap-4">
        <Field label="Type">
          <Select value={kind} onChange={(e) => setKind(e.target.value as Kind)}>
            <option value="bip39">BIP39 (12–24 words)</option>
            <option value="blockchain">blockchain.com legacy</option>
          </Select>
        </Field>
        <Field label="Gap limit">
          <Input
            type="number"
            value={gapLimit}
            onChange={(e) => setGapLimit(parseInt(e.target.value) || 20)}
            min={1}
            max={200}
          />
        </Field>
      </div>

      <Field label="Mnemonic phrase">
        <Textarea
          rows={3}
          value={phrase}
          onChange={(e) => setPhrase(e.target.value)}
          placeholder="legal winner thank year wave sausage worth useful legal winner thank yellow"
        />
      </Field>
      <p className="text-xs text-[var(--color-text-faint)] -mt-4">
        {words} {words === 1 ? "word" : "words"}
        {kind === "bip39" && !wordsOk && words > 0 && (
          <span className="text-[var(--color-warn)] ml-2">
            · BIP39 expects 12/15/18/21/24
          </span>
        )}
      </p>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Field label="BIP39 passphrase (optional)">
          <Input
            value={passphrase}
            onChange={(e) => setPassphrase(e.target.value)}
          />
        </Field>
        {kind === "blockchain" && (
          <Field label="Wordlist path">
            <Input
              value={wordlist}
              onChange={(e) => setWordlist(e.target.value)}
              placeholder="/path/to/blockchain_com_v2.txt"
            />
          </Field>
        )}
      </div>

      {err && <Banner variant="danger">{err}</Banner>}

      <div className="flex items-center gap-4">
        <Button type="submit" disabled={busy}>
          {busy ? "Deriving…" : "Derive keys"}
        </Button>
        {busy && <Spinner label="Deriving" />}
      </div>

      {decoded && (
        <Card>
          <p className="text-[10px] uppercase tracking-[0.08em] text-[var(--color-text-faint)]">
            {decoded.version} — {decoded.word_count} words
          </p>
          <p className="font-mono text-sm text-[var(--color-success)] break-all mt-2 mb-3">
            {decoded.password}
          </p>
          <CopyButton value={decoded.password} label="Copy password" />
          <p className="text-xs text-[var(--color-text-faint)] mt-3 m-0">
            This password unlocks the blockchain.com wallet.aes.json payload.
          </p>
        </Card>
      )}
    </form>
  );
}
```

- [ ] **Step 2: Typecheck**

```bash
pnpm -C apps/web run typecheck
```

- [ ] **Step 3: Commit**

```bash
git add apps/web/src/routes/MnemonicPage.tsx
git commit -m "feat(web): MnemonicPage with BIP39 + blockchain.com legacy"
```

---

## Phase 6 — Backend

### Task 6.1: Tx struct + `fetch_transactions` trait method + Mock impl

**Files:**

- Modify: `crates/orpheus-core/src/balance.rs`
- Modify: `crates/orpheus-core/Cargo.toml` (add `tempfile` to dev-dependencies if absent)

- [ ] **Step 1: Write the failing test**

Inside `crates/orpheus-core/src/balance.rs`, append at the bottom of the file:

```rust
#[cfg(test)]
mod tx_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn noop_provider_returns_empty_tx_list() {
        let p = NoopProvider;
        assert!(p.fetch_transactions("bc1qxyz", 50).is_empty());
    }

    #[test]
    fn mock_provider_returns_configured_tx_list() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mock.json");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"bc1qxyz":{{"balance_sat":0,"total_received_sat":0,"tx_count":0,"transactions":[
                {{"txid":"abc","time":1700000000,"value_sat":100000,"fee_sat":200,"confirmations":10,"block_height":800000}}
            ]}}}}"#
        )
        .unwrap();
        let p = MockProvider { path: Some(path) };
        let txs = p.fetch_transactions("bc1qxyz", 50);
        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0].txid, "abc");
        assert_eq!(txs[0].value_sat, 100_000);
    }
}
```

- [ ] **Step 2: Add `tempfile` to dev-dependencies if absent**

```bash
grep -q '^tempfile' crates/orpheus-core/Cargo.toml || {
  awk '/^\[dev-dependencies\]/{print; print "tempfile = \"3\""; next} {print}' \
    crates/orpheus-core/Cargo.toml > /tmp/orpheus-core-cargo && \
    mv /tmp/orpheus-core-cargo crates/orpheus-core/Cargo.toml
}
```

- [ ] **Step 3: Run test, confirm fails**

```bash
cargo test -p orpheus-core balance::tx_tests
```

Expected: FAIL — `fetch_transactions` does not exist.

- [ ] **Step 4: Implement the trait extension**

In `crates/orpheus-core/src/balance.rs`:

Add near the `MockEntry` definition:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Tx {
    pub txid: String,
    pub time: i64,
    pub value_sat: i64,
    pub fee_sat: Option<u64>,
    pub confirmations: Option<u32>,
    pub block_height: Option<u64>,
}
```

Extend the existing `MockEntry`:

```rust
#[derive(Debug, Clone, Deserialize)]
struct MockEntry {
    #[serde(default)]
    balance_sat: u64,
    #[serde(default)]
    total_received_sat: u64,
    #[serde(default)]
    total_sent_sat: Option<u64>,
    #[serde(default)]
    tx_count: u64,
    #[serde(default)]
    transactions: Vec<Tx>,
}
```

Extend the trait (add `fetch_transactions` with default empty impl):

```rust
pub trait BalanceProvider: Send + Sync {
    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo>;
    fn name(&self) -> &'static str;

    /// Fetch up to `limit` transactions touching `address`, newest first.
    /// Default impl returns empty — providers that can supply history override.
    fn fetch_transactions(&self, _address: &str, _limit: usize) -> Vec<Tx> {
        Vec::new()
    }
}
```

Refactor `MockProvider` to share the JSON-load path, then implement `fetch_transactions`:

```rust
impl MockProvider {
    fn load(&self) -> HashMap<String, MockEntry> {
        self.path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }
}

impl BalanceProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn fetch(&self, addresses: &[String]) -> HashMap<String, BalanceInfo> {
        let data = self.load();
        addresses
            .iter()
            .map(|addr| {
                let info = data
                    .get(addr)
                    .map(|e| BalanceInfo {
                        address: addr.clone(),
                        balance_sat: e.balance_sat,
                        total_received_sat: e.total_received_sat,
                        total_sent_sat: e
                            .total_sent_sat
                            .unwrap_or_else(|| e.total_received_sat.saturating_sub(e.balance_sat)),
                        tx_count: e.tx_count,
                    })
                    .unwrap_or_else(|| BalanceInfo::zero(addr.clone()));
                (addr.clone(), info)
            })
            .collect()
    }

    fn fetch_transactions(&self, address: &str, limit: usize) -> Vec<Tx> {
        self.load()
            .get(address)
            .map(|e| e.transactions.iter().take(limit).cloned().collect())
            .unwrap_or_default()
    }
}
```

- [ ] **Step 5: Run the new tests, verify green**

```bash
cargo test -p orpheus-core balance::tx_tests
```

Expected: 2 passed.

- [ ] **Step 6: Run the full core test suite**

```bash
cargo test -p orpheus-core --locked
```

Expected: all green. If a pre-existing test touching `MockProvider.fetch` broke because of the refactor, fix it before proceeding.

- [ ] **Step 7: Commit**

```bash
git add crates/orpheus-core/src/balance.rs crates/orpheus-core/Cargo.toml
git commit -m "feat(core): add Tx struct and fetch_transactions to BalanceProvider"
```

---

### Task 6.2: BlockstreamProvider::fetch_transactions + fixture

**Files:**

- Modify: `crates/orpheus-core/src/balance.rs`
- Create: `crates/orpheus-core/tests/fixtures/blockstream_txs.json`

- [ ] **Step 1: Create the fixture**

```bash
mkdir -p crates/orpheus-core/tests/fixtures
```

Write `crates/orpheus-core/tests/fixtures/blockstream_txs.json`:

```json
[
  {
    "txid": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "status": {
      "confirmed": true,
      "block_height": 800000,
      "block_time": 1700000000
    },
    "fee": 250,
    "vin": [{ "prevout": { "scriptpubkey_address": "bc1qother", "value": 150250 } }],
    "vout": [
      { "scriptpubkey_address": "bc1qxyz", "value": 150000 },
      { "scriptpubkey_address": "bc1qother2", "value": 0 }
    ]
  },
  {
    "txid": "aaabbbcccdddeeefffaaabbbcccdddeeefffaaabbbcccdddeeefffaaabbbcccd",
    "status": {
      "confirmed": true,
      "block_height": 799000,
      "block_time": 1699000000
    },
    "fee": 100,
    "vin": [{ "prevout": { "scriptpubkey_address": "bc1qxyz", "value": 80100 } }],
    "vout": [{ "scriptpubkey_address": "bc1qelsewhere", "value": 80000 }]
  }
]
```

- [ ] **Step 2: Write the failing test**

Append to `crates/orpheus-core/src/balance.rs`:

```rust
#[cfg(all(test, feature = "network"))]
mod blockstream_tx_tests {
    use super::*;

    #[test]
    fn parses_esplora_tx_response() {
        let raw = include_str!("../tests/fixtures/blockstream_txs.json");
        let txs = parse_blockstream_txs("bc1qxyz", raw).expect("parse");
        assert_eq!(txs.len(), 2);

        assert_eq!(
            txs[0].txid,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(txs[0].value_sat, 150_000);
        assert_eq!(txs[0].fee_sat, Some(250));
        assert_eq!(txs[0].block_height, Some(800_000));
        assert_eq!(txs[0].time, 1_700_000_000);

        assert_eq!(txs[1].value_sat, -80_100);
        assert_eq!(txs[1].block_height, Some(799_000));
    }
}
```

- [ ] **Step 3: Run the test, confirm fails**

```bash
cargo test -p orpheus-core --features network blockstream_tx_tests
```

- [ ] **Step 4: Implement parsing + the provider method**

Inside the existing `#[cfg(feature = "network")]` block in `balance.rs`, add:

```rust
#[derive(serde::Deserialize)]
struct BsVout {
    scriptpubkey_address: Option<String>,
    value: u64,
}
#[derive(serde::Deserialize)]
struct BsPrevout {
    scriptpubkey_address: Option<String>,
    value: u64,
}
#[derive(serde::Deserialize)]
struct BsVin {
    prevout: Option<BsPrevout>,
}
#[derive(serde::Deserialize)]
struct BsStatus {
    confirmed: bool,
    block_height: Option<u64>,
    block_time: Option<i64>,
}
#[derive(serde::Deserialize)]
struct BsTx {
    txid: String,
    status: BsStatus,
    fee: Option<u64>,
    vin: Vec<BsVin>,
    vout: Vec<BsVout>,
}

pub(crate) fn parse_blockstream_txs(address: &str, raw: &str) -> Result<Vec<Tx>, String> {
    let parsed: Vec<BsTx> = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    Ok(parsed
        .into_iter()
        .map(|tx| {
            let received: i64 = tx
                .vout
                .iter()
                .filter(|v| v.scriptpubkey_address.as_deref() == Some(address))
                .map(|v| v.value as i64)
                .sum();
            let sent: i64 = tx
                .vin
                .iter()
                .filter_map(|v| v.prevout.as_ref())
                .filter(|p| p.scriptpubkey_address.as_deref() == Some(address))
                .map(|p| p.value as i64)
                .sum();
            let fee_contribution = if sent > 0 { tx.fee.unwrap_or(0) as i64 } else { 0 };
            Tx {
                txid: tx.txid,
                time: tx.status.block_time.unwrap_or(0),
                value_sat: received - sent - fee_contribution,
                fee_sat: tx.fee,
                confirmations: None,
                block_height: if tx.status.confirmed {
                    tx.status.block_height
                } else {
                    None
                },
            }
        })
        .collect())
}
```

Modify the existing `impl BalanceProvider for BlockstreamProvider` block to add the method. Also add a helper on the struct:

```rust
impl BlockstreamProvider {
    fn fetch_txs_impl(&self, address: &str, limit: usize) -> Result<Vec<Tx>, String> {
        let url = format!("{}/address/{}/txs", self.base, address);
        let resp = self.client.get(&url).send().map_err(|e| e.to_string())?;
        let raw = resp.text().map_err(|e| e.to_string())?;
        let mut txs = parse_blockstream_txs(address, &raw)?;
        txs.truncate(limit);
        Ok(txs)
    }
}

impl BalanceProvider for BlockstreamProvider {
    // existing fetch + name methods unchanged — add:
    fn fetch_transactions(&self, address: &str, limit: usize) -> Vec<Tx> {
        self.fetch_txs_impl(address, limit).unwrap_or_default()
    }
}
```

Merge the new `fetch_transactions` into the existing `impl BalanceProvider for BlockstreamProvider { … }` block rather than creating a duplicate impl.

- [ ] **Step 5: Run the test, verify green**

```bash
cargo test -p orpheus-core --features network blockstream_tx_tests
```

- [ ] **Step 6: Lint + full tests**

```bash
mise run lint
mise run test
```

- [ ] **Step 7: Commit**

```bash
git add crates/orpheus-core/src/balance.rs \
  crates/orpheus-core/tests/fixtures/blockstream_txs.json
git commit -m "feat(core): BlockstreamProvider::fetch_transactions with pinned fixture"
```

---

### Task 6.3: BlockchainInfoProvider::fetch_transactions + fixture

**Files:**

- Modify: `crates/orpheus-core/src/balance.rs`
- Create: `crates/orpheus-core/tests/fixtures/blockchain_info_rawaddr.json`

- [ ] **Step 1: Fixture**

`crates/orpheus-core/tests/fixtures/blockchain_info_rawaddr.json`:

```json
{
  "address": "bc1qxyz",
  "n_tx": 2,
  "total_received": 150000,
  "total_sent": 80000,
  "final_balance": 69900,
  "txs": [
    {
      "hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "time": 1700000000,
      "fee": 250,
      "block_height": 800000,
      "result": 150000,
      "inputs": [
        { "prev_out": { "addr": "bc1qother", "value": 150250 } }
      ],
      "out": [{ "addr": "bc1qxyz", "value": 150000 }]
    },
    {
      "hash": "aaabbbcccdddeeefffaaabbbcccdddeeefffaaabbbcccdddeeefffaaabbbcccd",
      "time": 1699000000,
      "fee": 100,
      "block_height": 799000,
      "result": -80100,
      "inputs": [
        { "prev_out": { "addr": "bc1qxyz", "value": 80100 } }
      ],
      "out": [{ "addr": "bc1qelsewhere", "value": 80000 }]
    }
  ]
}
```

- [ ] **Step 2: Failing test**

Append to `balance.rs`:

```rust
#[cfg(all(test, feature = "network"))]
mod blockchain_info_tx_tests {
    use super::*;

    #[test]
    fn parses_rawaddr_response() {
        let raw = include_str!("../tests/fixtures/blockchain_info_rawaddr.json");
        let txs = parse_blockchain_info_txs(raw).expect("parse");
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].value_sat, 150_000);
        assert_eq!(txs[0].fee_sat, Some(250));
        assert_eq!(txs[1].value_sat, -80_100);
    }
}
```

- [ ] **Step 3: Run test, confirm fails**

```bash
cargo test -p orpheus-core --features network blockchain_info_tx_tests
```

- [ ] **Step 4: Implement**

Inside the `#[cfg(feature = "network")]` block:

```rust
#[derive(serde::Deserialize)]
struct BciRawAddr {
    txs: Vec<BciTx>,
}
#[derive(serde::Deserialize)]
struct BciTx {
    hash: String,
    time: i64,
    fee: Option<u64>,
    block_height: Option<u64>,
    result: i64,
}

pub(crate) fn parse_blockchain_info_txs(raw: &str) -> Result<Vec<Tx>, String> {
    let parsed: BciRawAddr = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    Ok(parsed
        .txs
        .into_iter()
        .map(|tx| Tx {
            txid: tx.hash,
            time: tx.time,
            value_sat: tx.result,
            fee_sat: tx.fee,
            confirmations: None,
            block_height: tx.block_height,
        })
        .collect())
}

impl BlockchainInfoProvider {
    fn fetch_txs_impl(&self, address: &str, limit: usize) -> Result<Vec<Tx>, String> {
        let url = format!("{}/rawaddr/{}?limit={}", self.base, address, limit);
        let resp = self.client.get(&url).send().map_err(|e| e.to_string())?;
        let raw = resp.text().map_err(|e| e.to_string())?;
        parse_blockchain_info_txs(&raw)
    }
}

impl BalanceProvider for BlockchainInfoProvider {
    // add to existing impl — do not duplicate
    fn fetch_transactions(&self, address: &str, limit: usize) -> Vec<Tx> {
        self.fetch_txs_impl(address, limit).unwrap_or_default()
    }
}
```

- [ ] **Step 5: Run test, verify green + lint + full test**

```bash
cargo test -p orpheus-core --features network blockchain_info_tx_tests
mise run lint
mise run test
```

- [ ] **Step 6: Commit**

```bash
git add crates/orpheus-core/src/balance.rs \
  crates/orpheus-core/tests/fixtures/blockchain_info_rawaddr.json
git commit -m "feat(core): BlockchainInfoProvider::fetch_transactions"
```

---

### Task 6.4: Server endpoint `GET /api/address/:address/transactions`

**Files:**

- Modify: `crates/orpheus-server/src/main.rs`

- [ ] **Step 1: Locate existing route registration**

```bash
grep -n "Router::new()\|/api/" crates/orpheus-server/src/main.rs
```

- [ ] **Step 2: Add the handler and route**

Add near the other `api_*` handlers in `crates/orpheus-server/src/main.rs`:

```rust
use orpheus_core::balance::{
    BalanceProvider, BlockchainInfoProvider, BlockstreamProvider, MockProvider, NoopProvider, Tx,
};

#[derive(serde::Deserialize)]
struct TxsQuery {
    #[serde(default)]
    provider: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

async fn api_address_transactions(
    axum::extract::Path(address): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<TxsQuery>,
) -> Json<Vec<Tx>> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let provider_name = q.provider.unwrap_or_else(|| "blockstream".into());
    let address_c = address.clone();

    let txs = tokio::task::spawn_blocking(move || -> Vec<Tx> {
        match provider_name.as_str() {
            "blockstream" => BlockstreamProvider::default().fetch_transactions(&address_c, limit),
            "blockchain" => {
                BlockchainInfoProvider::default().fetch_transactions(&address_c, limit)
            }
            "mock" => MockProvider { path: None }.fetch_transactions(&address_c, limit),
            _ => NoopProvider.fetch_transactions(&address_c, limit),
        }
    })
    .await
    .unwrap_or_default();

    Json(txs)
}
```

Add the route inside the existing `Router::new()` chain:

```rust
.route("/api/address/:address/transactions", get(api_address_transactions))
```

- [ ] **Step 3: Build + curl smoke**

```bash
cargo build -p orpheus-server --locked
mise run server:dev &
sleep 2
curl -s 'http://127.0.0.1:3000/api/address/bc1qxyz/transactions?provider=mock&limit=5'
kill %1 2>/dev/null || true
```

Expected: `[]`.

- [ ] **Step 4: Lint + tests**

```bash
mise run lint
mise run test
```

- [ ] **Step 5: Commit**

```bash
git add crates/orpheus-server/src/main.rs
git commit -m "feat(server): add GET /api/address/:address/transactions"
```

---

### Task 6.5: Server `--allow-local-paths` and `POST /api/scan-directory`

**Files:**

- Modify: `crates/orpheus-server/src/main.rs`
- Modify: `crates/orpheus-server/Cargo.toml`

- [ ] **Step 1: Add clap + dirs deps if absent**

```bash
grep -q '^clap' crates/orpheus-server/Cargo.toml || \
  awk '/^\[dependencies\]/{print; print "clap = { version = \"4\", features = [\"derive\"] }"; next} {print}' \
    crates/orpheus-server/Cargo.toml > /tmp/orpheus-server-cargo && \
  mv /tmp/orpheus-server-cargo crates/orpheus-server/Cargo.toml
grep -q '^dirs' crates/orpheus-server/Cargo.toml || \
  awk '/^\[dependencies\]/{print; print "dirs = \"5\""; next} {print}' \
    crates/orpheus-server/Cargo.toml > /tmp/orpheus-server-cargo && \
  mv /tmp/orpheus-server-cargo crates/orpheus-server/Cargo.toml
```

- [ ] **Step 2: Write the path-validator test**

Append to `crates/orpheus-server/src/main.rs`:

```rust
#[cfg(test)]
mod path_validator_tests {
    use super::*;

    #[test]
    fn rejects_relative_paths() {
        assert!(validate_scan_path("relative/path").is_err());
    }

    #[test]
    fn rejects_tilde() {
        assert!(validate_scan_path("~/wallets").is_err());
    }

    #[test]
    fn rejects_parent_traversal_outside_home() {
        let home = dirs::home_dir().unwrap();
        let trav = home.join("../etc");
        assert!(validate_scan_path(trav.to_str().unwrap()).is_err());
    }

    #[test]
    fn accepts_path_inside_home() {
        let home = dirs::home_dir().unwrap();
        let inside = home.join("some/subdir");
        assert!(validate_scan_path(inside.to_str().unwrap()).is_ok());
    }
}
```

- [ ] **Step 3: Run tests, confirm fails**

```bash
cargo test -p orpheus-server path_validator_tests
```

- [ ] **Step 4: Implement `validate_scan_path`**

Add at the top of `main.rs` (non-test):

```rust
use std::path::{Component, Path, PathBuf};

pub(crate) fn validate_scan_path(raw: &str) -> Result<PathBuf, String> {
    if raw.starts_with('~') {
        return Err("paths starting with ~ are not allowed; resolve first".into());
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err("path must be absolute".into());
    }
    let resolved = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            if path.components().any(|c| matches!(c, Component::ParentDir)) {
                return Err("path may not contain ..".into());
            }
            path.to_path_buf()
        }
    };
    let home = dirs::home_dir().ok_or_else(|| "no HOME dir".to_string())?;
    if !resolved.starts_with(&home) {
        return Err(format!("path must be inside {}", home.display()));
    }
    Ok(resolved)
}
```

- [ ] **Step 5: Run tests, verify green**

```bash
cargo test -p orpheus-server path_validator_tests
```

- [ ] **Step 6: Add CLI flag and gated route**

At the top of `main.rs`:

```rust
#[derive(clap::Parser, Debug)]
struct Args {
    /// Address to bind to.
    #[arg(long, default_value = "127.0.0.1:3000")]
    bind: String,

    /// Mount POST /api/scan-directory (reads files from server-side paths).
    #[arg(long)]
    allow_local_paths: bool,
}
```

Modify `async fn main` to parse and branch on the flag. Locate the existing `Router::new()` chain and update:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = <Args as clap::Parser>::parse();

    let mut app = Router::new()
        .route("/api/healthz", get(healthz))
        .route("/api/scan", post(api_scan))
        .route("/api/mnemonic", post(api_mnemonic))
        .route("/api/demo", post(api_demo))
        .route(
            "/api/address/:address/transactions",
            get(api_address_transactions),
        );

    if args.allow_local_paths {
        app = app.route("/api/scan-directory", post(api_scan_directory));
    }

    app = app.fallback(get(serve_static));

    let addr: std::net::SocketAddr = args.bind.parse()?;

    // (existing bind + serve logic that was at the end of main — keep unchanged)
    // …
}
```

(Replace whatever line was previously building the listener with `args.bind.parse()` or `addr`.)

Add the handler:

```rust
#[derive(serde::Deserialize)]
struct ScanDirBody {
    path: String,
    #[serde(default)]
    passwords: String,
    #[serde(default)]
    provider: Option<String>,
}

async fn api_scan_directory(
    Json(body): Json<ScanDirBody>,
) -> Result<Json<ScanReply>, (axum::http::StatusCode, String)> {
    let resolved = validate_scan_path(&body.path)
        .map_err(|msg| (axum::http::StatusCode::BAD_REQUEST, msg))?;
    let passwords_vec: Vec<String> = body
        .passwords
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    let provider_name = body.provider.unwrap_or_else(|| "blockstream".into());

    let results = tokio::task::spawn_blocking(move || {
        let provider: Option<Box<dyn orpheus_core::balance::BalanceProvider>> =
            match provider_name.as_str() {
                "blockstream" => Some(Box::new(
                    orpheus_core::balance::BlockstreamProvider::default(),
                )),
                "blockchain" => Some(Box::new(
                    orpheus_core::balance::BlockchainInfoProvider::default(),
                )),
                "mock" => Some(Box::new(orpheus_core::balance::MockProvider { path: None })),
                _ => None,
            };
        orpheus_core::scanner::scan_path(&resolved, &passwords_vec, provider.as_deref())
    })
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ScanReply {
        results,
        summary: None,
    }))
}
```

If the existing `ScanReply` in the server uses a different field layout (e.g. without `summary`), match that layout. `grep -n 'struct ScanReply' crates/orpheus-server/src/main.rs` to confirm.

- [ ] **Step 7: Build + test + lint**

```bash
cargo build -p orpheus-server --locked
mise run test
mise run lint
```

- [ ] **Step 8: Commit**

```bash
git add crates/orpheus-server/src/main.rs crates/orpheus-server/Cargo.toml
git commit -m "feat(server): add --allow-local-paths flag and /api/scan-directory"
```

---

### Task 6.6: Tauri `address_transactions` command + enable network feature

**Files:**

- Modify: `crates/orpheus-tauri/src-tauri/Cargo.toml`
- Modify: `crates/orpheus-tauri/src-tauri/src/lib.rs`

- [ ] **Step 1: Enable `network` feature on orpheus-core**

In `crates/orpheus-tauri/src-tauri/Cargo.toml`, change:

```toml
orpheus-core = { path = "../../orpheus-core" }
```

to:

```toml
orpheus-core = { path = "../../orpheus-core", features = ["network"] }
```

- [ ] **Step 2: Add the command handler**

Append to `crates/orpheus-tauri/src-tauri/src/lib.rs` (before the `pub fn run()` block):

```rust
use orpheus_core::balance::{
    BalanceProvider, BlockchainInfoProvider, BlockstreamProvider, MockProvider, NoopProvider, Tx,
};

#[tauri::command]
async fn address_transactions(
    address: String,
    provider: String,
    limit: Option<usize>,
) -> Result<Vec<Tx>, String> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let address_c = address.clone();
    tokio::task::spawn_blocking(move || {
        let p: Box<dyn BalanceProvider> = match provider.as_str() {
            "blockstream" => Box::new(BlockstreamProvider::default()),
            "blockchain" => Box::new(BlockchainInfoProvider::default()),
            "mock" => Box::new(MockProvider { path: None }),
            _ => Box::new(NoopProvider),
        };
        p.fetch_transactions(&address_c, limit)
    })
    .await
    .map_err(|e| e.to_string())
}
```

Register the command in the existing `tauri::Builder::default().invoke_handler(…)` call in `pub fn run()`. If you currently have:

```rust
.invoke_handler(tauri::generate_handler![scan_paths, mnemonic_cmd])
```

Change to:

```rust
.invoke_handler(tauri::generate_handler![
    scan_paths,
    mnemonic_cmd,
    address_transactions
])
```

(Use whatever existing command names are there as a starting set.)

- [ ] **Step 3: Build**

```bash
cargo build -p orpheus-tauri --locked
```

- [ ] **Step 4: Lint**

```bash
mise run lint
```

- [ ] **Step 5: Commit**

```bash
git add crates/orpheus-tauri/src-tauri/Cargo.toml \
  crates/orpheus-tauri/src-tauri/src/lib.rs
git commit -m "feat(tauri): add address_transactions and enable orpheus-core network feature"
```

---

## Phase 7 — Results pages

### Task 7.1: ResultsPage + WalletCard

**Files:**

- Rewrite: `apps/web/src/routes/ResultsPage.tsx`
- Create: `apps/web/src/components/results/WalletCard.tsx`

- [ ] **Step 1: Implement WalletCard**

```tsx
// apps/web/src/components/results/WalletCard.tsx
import { useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { Card } from "@/components/ui/Card";
import { Pill } from "@/components/ui/Pill";
import type { WalletScanResult } from "@/types";
import { walletId } from "@/lib/ids";
import { satToBtc, truncateMiddle } from "@/lib/utils";

export function WalletCard({ wallet }: { wallet: WalletScanResult }) {
  const [id, setId] = useState<string>("");
  useEffect(() => {
    walletId(wallet.source_file, wallet.source_type).then(setId);
  }, [wallet.source_file, wallet.source_type]);

  const withValue = wallet.keys.filter((k) => (k.balance_sat ?? 0) > 0);
  const subtotal = wallet.keys.reduce(
    (sum, k) => sum + (k.balance_sat ?? 0),
    0,
  );
  const preview = (withValue.length ? withValue : wallet.keys).slice(0, 3);

  return (
    <NavLink to={id ? `/results/${id}` : "#"} className="block no-underline">
      <Card className="hover:border-[var(--color-accent)] transition-colors">
        <div className="flex items-baseline gap-3">
          <h3 className="m-0 text-sm font-semibold text-[var(--color-text)] truncate">
            {wallet.source_file.split("/").pop() || wallet.source_file}
          </h3>
          <Pill tone="accent">{wallet.source_type}</Pill>
          <span className="ml-auto text-xs text-[var(--color-text-faint)] tabular-nums">
            {wallet.keys.length} keys ·{" "}
            <span
              className={
                withValue.length
                  ? "text-[var(--color-success)]"
                  : "text-[var(--color-text-faint)]"
              }
            >
              {withValue.length} with value
            </span>{" "}
            · {satToBtc(subtotal)} BTC
          </span>
        </div>
        {preview.length > 0 && (
          <ul className="mt-3 space-y-1 text-xs font-mono text-[var(--color-text-dim)]">
            {preview.map((k) => (
              <li key={k.address_compressed} className="tabular-nums">
                {truncateMiddle(k.address_compressed, 28)}{" "}
                <span className="text-[var(--color-text-faint)]">
                  · {satToBtc(k.balance_sat)} BTC
                </span>
              </li>
            ))}
          </ul>
        )}
      </Card>
    </NavLink>
  );
}
```

- [ ] **Step 2: Implement ResultsPage**

```tsx
// apps/web/src/routes/ResultsPage.tsx
import { useEffect, useState } from "react";
import { NavLink } from "react-router-dom";
import { Card } from "@/components/ui/Card";
import { StatBar } from "@/components/ui/StatBar";
import { WalletCard } from "@/components/results/WalletCard";
import { getResults } from "@/lib/storage";
import { satToBtc } from "@/lib/utils";
import type { WalletScanResult } from "@/types";

export default function ResultsPage() {
  const [results, setResults] = useState<WalletScanResult[] | null>(null);

  useEffect(() => {
    setResults(getResults());
  }, []);

  if (!results || !results.length) {
    return (
      <Card>
        <p className="m-0 text-[var(--color-text-dim)]">
          No scans this session. Start with{" "}
          <NavLink
            to="/scan"
            className="text-[var(--color-accent)] hover:underline"
          >
            Wallet files
          </NavLink>{" "}
          or{" "}
          <NavLink
            to="/mnemonic"
            className="text-[var(--color-accent)] hover:underline"
          >
            Mnemonic
          </NavLink>
          .
        </p>
      </Card>
    );
  }

  const totalKeys = results.reduce((s, r) => s + r.keys.length, 0);
  const withValue = results.filter((r) =>
    r.keys.some((k) => (k.balance_sat ?? 0) > 0),
  ).length;
  const recoveredSat = results.reduce(
    (s, r) => s + r.keys.reduce((a, k) => a + (k.balance_sat ?? 0), 0),
    0,
  );

  return (
    <div className="flex flex-col gap-4">
      <StatBar
        stats={[
          { label: "Wallets", value: String(results.length) },
          { label: "Keys", value: String(totalKeys) },
          { label: "With value", value: String(withValue), hit: withValue > 0 },
          {
            label: "Recovered",
            value: `${satToBtc(recoveredSat)} BTC`,
            hit: recoveredSat > 0,
          },
        ]}
      />
      <div className="flex flex-col gap-3">
        {results.map((r, i) => (
          <WalletCard key={`${r.source_file}-${i}`} wallet={r} />
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Typecheck + commit**

```bash
pnpm -C apps/web run typecheck
git add apps/web/src/routes/ResultsPage.tsx \
  apps/web/src/components/results/WalletCard.tsx
git commit -m "feat(web): ResultsPage with stat bar + wallet cards"
```

---

### Task 7.2: WalletPage + KeyTable

**Files:**

- Rewrite: `apps/web/src/routes/WalletPage.tsx`
- Create: `apps/web/src/components/results/KeyTable.tsx`

- [ ] **Step 1: Implement KeyTable**

```tsx
// apps/web/src/components/results/KeyTable.tsx
import { useNavigate } from "react-router-dom";
import { Table, type Column } from "@/components/ui/Table";
import { Pill } from "@/components/ui/Pill";
import type { ExtractedKey } from "@/types";
import { satToBtc, truncateMiddle } from "@/lib/utils";

function derivationTone(
  path: string | null | undefined,
): "accent" | "warn" | "neutral" {
  if (!path) return "neutral";
  if (path.startsWith("m/84'")) return "accent";
  if (path.startsWith("m/49'")) return "accent";
  if (path.startsWith("m/44'")) return "accent";
  if (path.startsWith("m/0'/")) return "warn";
  return "neutral";
}

function derivationLabel(path: string | null | undefined): string {
  if (!path) return "—";
  if (path.startsWith("m/84'")) return "BIP84";
  if (path.startsWith("m/49'")) return "BIP49";
  if (path.startsWith("m/44'")) return "BIP44";
  if (path.startsWith("m/0'/")) return "Bread";
  return path;
}

export function KeyTable({
  keys,
  walletId,
}: {
  keys: ExtractedKey[];
  walletId: string;
}) {
  const navigate = useNavigate();
  const columns: Column<ExtractedKey>[] = [
    {
      key: "path",
      header: "Derivation",
      sortValue: (k) => k.derivation_path ?? "",
      render: (k) => (
        <Pill tone={derivationTone(k.derivation_path)}>
          {derivationLabel(k.derivation_path)}
        </Pill>
      ),
    },
    {
      key: "address",
      header: "Address",
      sortValue: (k) => k.address_compressed,
      render: (k) => (
        <span className="font-mono text-xs">
          {truncateMiddle(k.address_compressed, 28)}
        </span>
      ),
    },
    {
      key: "btc",
      header: "BTC",
      align: "right",
      sortValue: (k) => k.balance_sat ?? 0,
      render: (k) => {
        const hit = (k.balance_sat ?? 0) > 0;
        return (
          <span
            className={
              hit
                ? "text-[var(--color-success)]"
                : "text-[var(--color-text-dim)]"
            }
          >
            {satToBtc(k.balance_sat)}
          </span>
        );
      },
    },
    {
      key: "txs",
      header: "Txs",
      align: "right",
      sortValue: (k) => k.tx_count ?? 0,
      render: (k) => <span>{k.tx_count ?? 0}</span>,
    },
  ];

  return (
    <Table
      columns={columns}
      rows={keys}
      rowKey={(k) => k.address_compressed}
      onRowClick={(k) =>
        navigate(
          `/results/${walletId}/${encodeURIComponent(k.address_compressed)}`,
        )
      }
    />
  );
}
```

- [ ] **Step 2: Implement WalletPage**

```tsx
// apps/web/src/routes/WalletPage.tsx
import { useEffect, useMemo, useState } from "react";
import { useParams } from "react-router-dom";
import { Breadcrumbs } from "@/components/layout/Breadcrumbs";
import { Card } from "@/components/ui/Card";
import { Pill } from "@/components/ui/Pill";
import { KeyTable } from "@/components/results/KeyTable";
import { getResults } from "@/lib/storage";
import { walletId as deriveWalletId } from "@/lib/ids";
import { satToBtc } from "@/lib/utils";
import type { WalletScanResult } from "@/types";

type Filter = "all" | "with-value" | "empty";

export default function WalletPage() {
  const { walletId } = useParams<{ walletId: string }>();
  const [wallet, setWallet] = useState<WalletScanResult | null>(null);
  const [filter, setFilter] = useState<Filter>("all");

  useEffect(() => {
    (async () => {
      const all = getResults();
      if (!all || !walletId) return;
      for (const w of all) {
        const id = await deriveWalletId(w.source_file, w.source_type);
        if (id === walletId) {
          setWallet(w);
          break;
        }
      }
    })();
  }, [walletId]);

  useEffect(() => {
    if (!wallet) return;
    const hasValue = wallet.keys.some((k) => (k.balance_sat ?? 0) > 0);
    if (hasValue) setFilter("with-value");
  }, [wallet]);

  const keys = useMemo(() => {
    if (!wallet) return [];
    if (filter === "with-value") {
      return wallet.keys.filter((k) => (k.balance_sat ?? 0) > 0);
    }
    if (filter === "empty") {
      return wallet.keys.filter((k) => (k.balance_sat ?? 0) === 0);
    }
    return wallet.keys;
  }, [wallet, filter]);

  if (!wallet || !walletId) {
    return <p className="text-[var(--color-text-dim)]">Wallet not found.</p>;
  }

  const withValue = wallet.keys.filter((k) => (k.balance_sat ?? 0) > 0).length;
  const subtotal = wallet.keys.reduce((s, k) => s + (k.balance_sat ?? 0), 0);
  const shortFile = wallet.source_file.split("/").pop() || wallet.source_file;

  return (
    <div className="flex flex-col gap-4">
      <Breadcrumbs
        segments={[
          { label: "Results", to: "/results" },
          { label: shortFile },
        ]}
      />
      <Card>
        <div className="flex items-baseline gap-3">
          <h1 className="text-lg font-semibold m-0">{shortFile}</h1>
          <Pill tone="accent">{wallet.source_type}</Pill>
          <span className="ml-auto text-xs text-[var(--color-text-faint)]">
            {wallet.keys.length} keys · {withValue} with value ·{" "}
            <span
              className={
                subtotal > 0 ? "text-[var(--color-success)]" : undefined
              }
            >
              {satToBtc(subtotal)} BTC
            </span>
          </span>
        </div>
      </Card>

      <div className="flex gap-2 text-xs">
        {(["all", "with-value", "empty"] as const).map((f) => (
          <button
            key={f}
            type="button"
            onClick={() => setFilter(f)}
            className={
              filter === f
                ? "px-2 py-1 rounded border border-[var(--color-accent)] text-[var(--color-accent)]"
                : "px-2 py-1 rounded border border-[var(--color-border)] text-[var(--color-text-dim)] hover:text-[var(--color-text)]"
            }
          >
            {f === "all" && `All (${wallet.keys.length})`}
            {f === "with-value" && `With value (${withValue})`}
            {f === "empty" && `Empty (${wallet.keys.length - withValue})`}
          </button>
        ))}
      </div>

      <KeyTable keys={keys} walletId={walletId} />
    </div>
  );
}
```

- [ ] **Step 3: Typecheck + commit**

```bash
pnpm -C apps/web run typecheck
git add apps/web/src/routes/WalletPage.tsx \
  apps/web/src/components/results/KeyTable.tsx
git commit -m "feat(web): WalletPage + KeyTable with filter tabs"
```

---

### Task 7.3: AddressPage + TxTable

**Files:**

- Rewrite: `apps/web/src/routes/AddressPage.tsx`
- Create: `apps/web/src/components/results/TxTable.tsx`

- [ ] **Step 1: Implement TxTable**

```tsx
// apps/web/src/components/results/TxTable.tsx
import { Table, type Column } from "@/components/ui/Table";
import type { Tx } from "@/types";
import { satToBtc, truncateMiddle } from "@/lib/utils";

export function TxTable({ txs }: { txs: Tx[] }) {
  if (!txs.length) {
    return (
      <p className="text-xs text-[var(--color-text-faint)]">
        No transactions for this address.
      </p>
    );
  }
  const columns: Column<Tx>[] = [
    {
      key: "txid",
      header: "Txid",
      render: (t) => (
        <a
          href={`https://blockstream.info/tx/${t.txid}`}
          target="_blank"
          rel="noopener noreferrer"
          className="font-mono text-xs text-[var(--color-accent)] hover:underline"
        >
          {truncateMiddle(t.txid, 20)}
        </a>
      ),
    },
    {
      key: "time",
      header: "Date",
      render: (t) => new Date(t.time * 1000).toISOString().slice(0, 10),
      sortValue: (t) => t.time,
    },
    {
      key: "value",
      header: "Value",
      align: "right",
      sortValue: (t) => t.value_sat,
      render: (t) => {
        const cls =
          t.value_sat > 0
            ? "text-[var(--color-success)]"
            : t.value_sat < 0
              ? "text-[var(--color-danger)]"
              : "text-[var(--color-text-dim)]";
        const sign = t.value_sat > 0 ? "+" : t.value_sat < 0 ? "−" : "";
        return (
          <span className={cls}>
            {sign} {satToBtc(Math.abs(t.value_sat))}
          </span>
        );
      },
    },
    {
      key: "block",
      header: "Block",
      align: "right",
      render: (t) => (t.block_height != null ? String(t.block_height) : "—"),
    },
  ];
  return <Table columns={columns} rows={txs} rowKey={(t) => t.txid} />;
}
```

- [ ] **Step 2: Implement AddressPage**

```tsx
// apps/web/src/routes/AddressPage.tsx
import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { Breadcrumbs } from "@/components/layout/Breadcrumbs";
import { Card } from "@/components/ui/Card";
import { Button } from "@/components/ui/Button";
import { StatBar } from "@/components/ui/StatBar";
import { CopyButton } from "@/components/ui/CopyButton";
import { WifReveal } from "@/components/ui/WifReveal";
import { QRCode } from "@/components/ui/QRCode";
import { TxTable } from "@/components/results/TxTable";
import { Spinner } from "@/components/ui/Spinner";
import { Banner } from "@/components/ui/Banner";
import { addressTransactions } from "@/lib/api";
import { getResults } from "@/lib/storage";
import { walletId as deriveWalletId, keyId as deriveKeyId } from "@/lib/ids";
import { satToBtc } from "@/lib/utils";
import type {
  ExtractedKey,
  Provider,
  Tx,
  WalletScanResult,
} from "@/types";

export default function AddressPage({ provider }: { provider: Provider }) {
  const { walletId, address } = useParams<{
    walletId: string;
    address: string;
  }>();
  const navigate = useNavigate();
  const decodedAddress = address ? decodeURIComponent(address) : "";

  const [wallet, setWallet] = useState<WalletScanResult | null>(null);
  const [key, setKey] = useState<ExtractedKey | null>(null);
  const [keyId, setKeyId] = useState<string>("");
  const [txs, setTxs] = useState<Tx[] | null>(null);
  const [txErr, setTxErr] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      const all = getResults();
      if (!all || !walletId || !decodedAddress) return;
      for (const w of all) {
        const id = await deriveWalletId(w.source_file, w.source_type);
        if (id === walletId) {
          setWallet(w);
          const k = w.keys.find((k) => k.address_compressed === decodedAddress);
          if (k) {
            setKey(k);
            setKeyId(await deriveKeyId(k.address_compressed));
          }
          break;
        }
      }
    })();
  }, [walletId, decodedAddress]);

  useEffect(() => {
    if (!decodedAddress) return;
    setTxs(null);
    setTxErr(null);
    addressTransactions(decodedAddress, provider, 50)
      .then(setTxs)
      .catch((e) => setTxErr((e as Error).message));
  }, [decodedAddress, provider]);

  if (!wallet || !key) {
    return <p className="text-[var(--color-text-dim)]">Address not found.</p>;
  }

  const shortFile = wallet.source_file.split("/").pop() || wallet.source_file;
  const balance = key.balance_sat ?? 0;

  return (
    <div className="flex flex-col gap-4">
      <Breadcrumbs
        segments={[
          { label: "Results", to: "/results" },
          { label: shortFile, to: `/results/${walletId}` },
          {
            label:
              decodedAddress.slice(0, 6) + "…" + decodedAddress.slice(-4),
          },
        ]}
      />
      <div className="grid grid-cols-1 md:grid-cols-[2fr_1fr] gap-4">
        <div className="flex flex-col gap-4">
          <Card>
            <p className="text-[10px] uppercase tracking-[0.08em] text-[var(--color-text-faint)] m-0">
              Address
            </p>
            <p className="font-mono text-sm m-0 mt-1 mb-3 break-all">
              {decodedAddress}
            </p>
            <p
              className={
                "text-2xl font-semibold tabular-nums m-0 " +
                (balance > 0
                  ? "text-[var(--color-success)]"
                  : "text-[var(--color-text-dim)]")
              }
            >
              {satToBtc(balance)} BTC
            </p>
          </Card>

          <StatBar
            stats={[
              { label: "Derivation", value: key.derivation_path ?? "—" },
              {
                label: "Total received",
                value: satToBtc(key.total_received_sat),
              },
              { label: "Transactions", value: String(key.tx_count ?? 0) },
            ]}
          />

          <Card>
            <h2 className="m-0 mb-3 text-sm font-semibold">Transactions</h2>
            {txErr && <Banner variant="danger">{txErr}</Banner>}
            {!txErr && txs === null && <Spinner label="Loading transactions" />}
            {txs && <TxTable txs={txs} />}
          </Card>
        </div>

        <div className="flex flex-col gap-4">
          <Card>
            <QRCode value={decodedAddress} size={140} />
          </Card>
          <WifReveal wif={key.wif} />
          <div className="flex flex-col gap-2">
            <CopyButton value={decodedAddress} label="Copy address" />
            <a
              href={`https://blockstream.info/address/${decodedAddress}`}
              target="_blank"
              rel="noopener noreferrer"
              className="block"
            >
              <Button variant="secondary" className="w-full">
                Open in Blockstream ↗
              </Button>
            </a>
            <Button
              variant="success"
              onClick={() => navigate(`/import/${keyId}`)}
            >
              Import this key →
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Typecheck + commit**

```bash
pnpm -C apps/web run typecheck
git add apps/web/src/routes/AddressPage.tsx \
  apps/web/src/components/results/TxTable.tsx
git commit -m "feat(web): AddressPage with tx history, QR, WifReveal, CTAs"
```

---

## Phase 8 — Import page

### Task 8.1: ImportPage with per-wallet tabs

**Files:**

- Rewrite: `apps/web/src/routes/ImportPage.tsx`
- Create: `apps/web/src/components/import/ElectrumSteps.tsx`
- Create: `apps/web/src/components/import/SparrowSteps.tsx`
- Create: `apps/web/src/components/import/BitcoinCoreSteps.tsx`
- Create: `apps/web/src/components/import/HardwareSteps.tsx`

- [ ] **Step 1: Implement the four step components**

```tsx
// apps/web/src/components/import/ElectrumSteps.tsx
import { WifReveal } from "@/components/ui/WifReveal";

export function ElectrumSteps({ wif }: { wif: string }) {
  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-[var(--color-text-dim)] m-0">
        Sweep this key into a new Electrum wallet so you control the funds going
        forward.
      </p>
      <ol className="text-sm leading-7 pl-5 m-0">
        <li>
          Open Electrum → <strong>File</strong> → <strong>New/Restore</strong>.
        </li>
        <li>
          Name the wallet, choose{" "}
          <strong>Import Bitcoin addresses or private keys</strong>.
        </li>
        <li>
          Paste the WIF below and click <strong>Next</strong>.
        </li>
        <li>
          Electrum shows the balance and lets you send to an address{" "}
          <em>you</em> control.
        </li>
      </ol>
      <WifReveal wif={wif} />
    </div>
  );
}
```

```tsx
// apps/web/src/components/import/SparrowSteps.tsx
import { WifReveal } from "@/components/ui/WifReveal";

export function SparrowSteps({ wif }: { wif: string }) {
  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-[var(--color-text-dim)] m-0">
        Sparrow supports sweeping a single WIF into a new wallet.
      </p>
      <ol className="text-sm leading-7 pl-5 m-0">
        <li>
          Sparrow → <strong>File</strong> → <strong>New Wallet</strong>.
        </li>
        <li>
          Select <strong>Imported Addresses/Private Keys</strong> and click{" "}
          <strong>Next</strong>.
        </li>
        <li>Paste the WIF and confirm the derived address.</li>
        <li>
          Use <strong>Send</strong> to sweep funds to an address you control.
        </li>
      </ol>
      <WifReveal wif={wif} />
    </div>
  );
}
```

```tsx
// apps/web/src/components/import/BitcoinCoreSteps.tsx
import { WifReveal } from "@/components/ui/WifReveal";
import { CopyButton } from "@/components/ui/CopyButton";

export function BitcoinCoreSteps({ wif }: { wif: string }) {
  const command = `bitcoin-cli importprivkey "${wif}" "orpheus-recovered" false`;
  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-[var(--color-text-dim)] m-0">
        Use <code>bitcoin-cli</code> against your own fully-synced node.
      </p>
      <ol className="text-sm leading-7 pl-5 m-0">
        <li>
          Make sure <code>bitcoind</code> is running and synced.
        </li>
        <li>
          Run the import command (the trailing <code>false</code> skips rescan;
          omit to rescan).
        </li>
        <li>
          Use <code>listunspent</code> to confirm balance and{" "}
          <code>sendtoaddress</code> to sweep.
        </li>
      </ol>
      <pre className="bg-[var(--color-surface)] border border-[var(--color-border)] rounded-[5px] p-3 font-mono text-xs overflow-x-auto m-0">
        {command}
      </pre>
      <div>
        <CopyButton value={command} label="Copy command" />
      </div>
      <WifReveal wif={wif} />
    </div>
  );
}
```

```tsx
// apps/web/src/components/import/HardwareSteps.tsx
export function HardwareSteps() {
  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-[var(--color-text-dim)] m-0">
        Hardware wallets hold keys in secure elements — they cannot{" "}
        <em>import</em> a WIF derived elsewhere without revealing it to the
        host.
      </p>
      <p className="text-sm m-0">The recommended flow is:</p>
      <ol className="text-sm leading-7 pl-5 m-0">
        <li>
          Set up a new account on your hardware wallet (Coldcard, Jade, Passport,
          Trezor).
        </li>
        <li>
          Using Electrum or Sparrow, sweep the recovered WIF to an address that
          account controls.
        </li>
        <li>Verify the new balance on the hardware wallet.</li>
      </ol>
      <p className="text-xs text-[var(--color-text-faint)] m-0">
        This keeps your long-term storage cold — the extracted WIF only lives in
        the sweeping host for the duration of a single transaction.
      </p>
    </div>
  );
}
```

- [ ] **Step 2: Implement ImportPage**

```tsx
// apps/web/src/routes/ImportPage.tsx
import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { Breadcrumbs } from "@/components/layout/Breadcrumbs";
import { Banner } from "@/components/ui/Banner";
import { ElectrumSteps } from "@/components/import/ElectrumSteps";
import { SparrowSteps } from "@/components/import/SparrowSteps";
import { BitcoinCoreSteps } from "@/components/import/BitcoinCoreSteps";
import { HardwareSteps } from "@/components/import/HardwareSteps";
import { getResults } from "@/lib/storage";
import { keyId as deriveKeyId } from "@/lib/ids";
import type { ExtractedKey } from "@/types";
import { cn } from "@/lib/utils";

type Tab = "electrum" | "sparrow" | "core" | "hardware";

export default function ImportPage() {
  const { keyId } = useParams<{ keyId: string }>();
  const [key, setKey] = useState<ExtractedKey | null>(null);
  const [tab, setTab] = useState<Tab>("electrum");

  useEffect(() => {
    (async () => {
      const all = getResults();
      if (!all || !keyId) return;
      for (const w of all) {
        for (const k of w.keys) {
          const id = await deriveKeyId(k.address_compressed);
          if (id === keyId) {
            setKey(k);
            return;
          }
        }
      }
    })();
  }, [keyId]);

  if (!key) {
    return <p className="text-[var(--color-text-dim)]">Key not found.</p>;
  }

  return (
    <div className="flex flex-col gap-4 max-w-[800px]">
      <Breadcrumbs segments={[{ label: "Import key" }]} />
      <Banner variant="warn">
        <strong>Handle on an air-gapped machine.</strong> Clear clipboard after
        use. Prefer sweeping funds to a new address once the key is imported.
      </Banner>
      <div className="flex gap-0 border-b border-[var(--color-border)]">
        {(
          [
            ["electrum", "Electrum"],
            ["sparrow", "Sparrow"],
            ["core", "Bitcoin Core"],
            ["hardware", "Hardware wallet"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            onClick={() => setTab(id)}
            className={cn(
              "px-4 py-2 text-sm border-b-2",
              tab === id
                ? "border-[var(--color-accent)] text-[var(--color-text)]"
                : "border-transparent text-[var(--color-text-dim)] hover:text-[var(--color-text)]",
            )}
          >
            {label}
          </button>
        ))}
      </div>
      <div>
        {tab === "electrum" && <ElectrumSteps wif={key.wif} />}
        {tab === "sparrow" && <SparrowSteps wif={key.wif} />}
        {tab === "core" && <BitcoinCoreSteps wif={key.wif} />}
        {tab === "hardware" && <HardwareSteps />}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Typecheck + commit**

```bash
pnpm -C apps/web run typecheck
git add apps/web/src/routes/ImportPage.tsx \
  apps/web/src/components/import/
git commit -m "feat(web): ImportPage with Electrum/Sparrow/Core/Hardware tabs"
```

---

## Phase 9 — Cleanup & polish

### Task 9.1: Delete old components

**Files:**

- Delete: `apps/web/src/components/Descent.tsx`
- Delete: `apps/web/src/components/Masthead.tsx`
- Delete: `apps/web/src/components/Dropzone.tsx`
- Delete: `apps/web/src/components/Field.tsx`
- Delete: `apps/web/src/components/ResultsView.tsx`

- [ ] **Step 1: Confirm no imports remain**

```bash
grep -rn 'components/Descent\|components/Masthead\|components/Dropzone"\|components/Field"\|components/ResultsView' apps/web/src/ || echo "no references"
```

Expected: "no references".

- [ ] **Step 2: Delete**

```bash
rm apps/web/src/components/Descent.tsx \
   apps/web/src/components/Masthead.tsx \
   apps/web/src/components/Dropzone.tsx \
   apps/web/src/components/Field.tsx \
   apps/web/src/components/ResultsView.tsx
```

- [ ] **Step 3: Typecheck + tests**

```bash
pnpm -C apps/web run typecheck
pnpm -C apps/web run test
```

- [ ] **Step 4: Commit**

```bash
git add -A apps/web/src/components/
git commit -m "refactor(web): delete old Greek-mythology-themed components"
```

---

### Task 9.2: Wire contrast check into `mise run ci`

**Files:**

- Create: `scripts/check-contrast.mjs`
- Modify: `mise.toml`

- [ ] **Step 1: Write the contrast script**

```js
// scripts/check-contrast.mjs
import fs from "node:fs";

function hexToRgb(h) {
  const s = h.replace("#", "");
  return [
    parseInt(s.slice(0, 2), 16),
    parseInt(s.slice(2, 4), 16),
    parseInt(s.slice(4, 6), 16),
  ];
}

function relLum([r, g, b]) {
  const lin = (c) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  const [R, G, B] = [lin(r), lin(g), lin(b)];
  return 0.2126 * R + 0.7152 * G + 0.0722 * B;
}

function ratio(a, b) {
  const l1 = Math.max(a, b);
  const l2 = Math.min(a, b);
  return (l1 + 0.05) / (l2 + 0.05);
}

const css = fs.readFileSync("apps/web/src/index.css", "utf8");

function extractBlock(source, re) {
  const m = source.match(re);
  if (!m) throw new Error("block not found: " + re);
  return Object.fromEntries(
    [...m[1].matchAll(/--([a-z0-9-]+):\s*(#[0-9a-fA-F]+);/g)].map((mm) => [
      mm[1],
      mm[2],
    ]),
  );
}

const dark = extractBlock(css, /@theme\s*\{([^}]+)\}/);
const light = extractBlock(
  css,
  /prefers-color-scheme:\s*light\s*\)\s*\{\s*:root\s*\{([^}]+)\}/,
);

const pairs = [
  ["color-text", "color-bg", 4.5],
  ["color-text-dim", "color-bg", 4.5],
  ["color-text", "color-surface", 4.5],
  ["color-accent", "color-bg", 3.0],
  ["color-success", "color-bg", 3.0],
  ["color-warn", "color-bg", 3.0],
  ["color-danger", "color-bg", 3.0],
];

let failed = false;
for (const theme of ["dark", "light"]) {
  const src = theme === "dark" ? dark : light;
  for (const [fg, bg, min] of pairs) {
    const r = ratio(relLum(hexToRgb(src[fg])), relLum(hexToRgb(src[bg])));
    const ok = r >= min;
    console.log(
      `${theme.padEnd(5)} ${fg} / ${bg}: ${r.toFixed(2)} (min ${min}) ${
        ok ? "OK" : "FAIL"
      }`,
    );
    if (!ok) failed = true;
  }
}
process.exit(failed ? 1 : 0);
```

- [ ] **Step 2: Check the mise task-dep syntax used elsewhere**

```bash
grep -n 'depends\|run =' mise.toml | head -20
```

Note whether tasks declare dependencies via `depends = […]` or `depends_on = […]`.

- [ ] **Step 3: Add `web:contrast` task + reference from `ci` task**

Append to `mise.toml`:

```toml
[tasks."web:contrast"]
description = "Fail if WCAG AA contrast drops in apps/web/src/index.css tokens"
run = "node scripts/check-contrast.mjs"
```

In the existing `[tasks.ci]` (or whatever the aggregate task is called), add `"web:contrast"` to its `depends` / `depends_on` array (match the existing syntax).

- [ ] **Step 4: Run it**

```bash
mise run web:contrast
```

Expected: every pair reports `OK` for both themes.

- [ ] **Step 5: Run full CI**

```bash
mise run ci
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add scripts/check-contrast.mjs mise.toml
git commit -m "build: add web:contrast check to mise run ci"
```

---

### Task 9.3: Manual smoke across Chrome, Firefox, Tauri

Non-automated verification. No code, no commits.

- [ ] **Step 1: Chrome (server path)**

```bash
mise run dev
```

In Chrome, visit `http://localhost:5173`:

- `/scan` renders "Pick a folder" button.
- Click it; pick `fixtures/demo-wallets/` (generated by `mise run demo:fixtures` if needed).
- Confirm scan completes and navigates to `/results`.
- Click a wallet card → `/results/:walletId` table loads.
- Click a row with balance → `/results/:walletId/:address` loads tx history.
- Click "Import this key →" → `/import/:keyId` loads Electrum tab.
- Reveal WIF; copy; confirm clipboard is populated.

- [ ] **Step 2: Firefox fallback**

In Firefox, visit the same URL.

- `/scan` shows no "Pick a folder" button; the "Or select individual files" link shows the dropzone.
- Drop fixture files; click Scan; confirm results flow works.

- [ ] **Step 3: Tauri**

```bash
mise run tauri:dev
```

In the desktop app:

- `/scan` — "Choose folder" button opens the native OS picker.
- Pick the fixtures directory; confirm scan works via Tauri command.
- Confirm address detail loads tx history through `address_transactions`.

- [ ] **Step 4: Record results in the PR description**

```markdown
## Test plan
- [x] `mise run ci` — green
- [x] Chrome: /scan → /results → /results/:id → /results/:id/:addr → /import/:id
- [x] Firefox: dropzone fallback works
- [x] Tauri: native folder picker + address tx history
```

---

## Self-review (performed while writing this plan)

**Spec coverage:**

- §1 broken CSS → Task 0.3 rewrites `index.css` with defined tokens.
- §1 remove Greek theming → Tasks 3.1 (routes), 3.2 (Header), 4.1, 5.1, 7.1–7.3, 9.1 (delete).
- §2 non-goals — no extractor/crypto changes; no sweep/sign; no fiat feed.
- §3 tokens → Task 0.3; type + spacing observed throughout.
- §4 routes → 3.1; sessionStorage → 1.2; header + indicator → 3.2.
- §5 input methods → Task 4.1 + lib/fs-access.ts (1.5) + lib/tauri.ts (1.4).
- §6 screens — every route has an implementation task.
- §7 backend — Tx + trait (6.1); Blockstream + BlockchainInfo fixtures (6.2, 6.3); server endpoint (6.4); --allow-local-paths (6.5); Tauri command (6.6); Mock extension in 6.1.
- §8 file structure matches the tasks.
- §9 testing — every primitive + lib has a colocated test task; provider tx-parsing pinned in 6.2, 6.3; path validator in 6.5.
- §10 a11y — focus ring in 0.3, aria labels in WifReveal (2.5), sort aria in Table (2.7), Spinner role=status (2.3), semantic tabs in ImportPage (8.1). Contrast check in 9.2.
- §11 punts — none implemented (per design).
- §12 ship order — tasks ordered to match.

**Placeholder scan:** no `TBD`/`TODO`/"implement later" strings. Every test step has real code. Every commit command is concrete.

**Type consistency:**

- `Provider` defined in Task 1.6 (ts) and reused in Header (3.2), ScanPage (4.1), AddressPage (7.3).
- `Tx` struct defined in Task 6.1 (Rust) and Task 1.6 (TS) with matching fields.
- `walletId(sourceFile, sourceType)` / `keyId(address)` signatures consistent across 1.1, 7.1, 7.2, 7.3, 8.1.
- `WalletScanResult.keys[].balance_sat` is `number | null | undefined` throughout.
- `Column<T>` shape in Table (2.7) matches KeyTable (7.2) and TxTable (7.3).

No inconsistencies found.

---

## Execution

Plan complete and saved to `docs/superpowers/plans/2026-04-17-web-redesign.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
