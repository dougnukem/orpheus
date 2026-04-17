import { describe, it, expect } from "vitest";
import { walkDirectory, WALLET_FILE_PATTERNS } from "./fs-access";

interface FakeFileHandle {
  kind: "file";
  name: string;
  getFile(): Promise<File>;
}

interface FakeDirHandle {
  kind: "directory";
  name: string;
  values(): AsyncGenerator<FakeFileHandle | FakeDirHandle>;
}

function fakeFile(name: string, size = 1000): File {
  return new File([new Uint8Array(size)], name);
}

function fakeFileHandle(name: string, size = 1000): FakeFileHandle {
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
  entries: Array<FakeFileHandle | FakeDirHandle>,
): FakeDirHandle {
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
