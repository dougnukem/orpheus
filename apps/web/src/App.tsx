import { useState } from "react";

import { Masthead } from "@/components/Masthead";
import { Descent } from "@/components/Descent";
import { Dropzone } from "@/components/Dropzone";
import { Field, FieldLabel, PrimaryButton, Select, TextArea, TextInput } from "@/components/Field";
import { ResultsView } from "@/components/ResultsView";
import * as api from "@/lib/api";
import type { DecodedMnemonic, ExtractedKey, TabId, WalletScanResult } from "@/types";
import { cn } from "@/lib/utils";

const VERSION = "0.1.0";

type Status = { kind: "idle" | "busy" | "ok" | "err"; text: string };

function useStatus() {
  const [status, setStatus] = useState<Status>({ kind: "idle", text: "" });
  return { status, setStatus };
}

export default function App() {
  const [tab, setTab] = useState<TabId>("scan");
  const [results, setResults] = useState<WalletScanResult[] | null>(null);
  const [demoBusy, setDemoBusy] = useState(false);

  const runDemo = async () => {
    setDemoBusy(true);
    try {
      const r = await api.demo();
      setResults(r.results);
      setTab("results");
    } finally {
      setDemoBusy(false);
    }
  };

  return (
    <div className="min-h-full">
      <div className="grain" aria-hidden />
      <Masthead version={VERSION} />

      <main className="grid grid-cols-1 md:grid-cols-[minmax(240px,320px)_1fr] gap-8 max-w-[1400px] mx-auto px-8 py-10">
        <Descent active={tab} onSelect={setTab} onDemo={runDemo} demoBusy={demoBusy} />

        <section className="relative min-h-[60vh]">
          {tab === "scan" && <ScanPanel onResults={(r) => { setResults(r); setTab("results"); }} />}
          {tab === "extract" && (
            <ExtractPanel onResults={(r) => { setResults(r); setTab("results"); }} />
          )}
          {tab === "mnemonic" && (
            <MnemonicPanel
              onKeys={(keys) => {
                setResults([{ source_file: "(mnemonic)", source_type: "bip39", keys }]);
                setTab("results");
              }}
            />
          )}
          {tab === "results" && <ResultsPanel results={results} />}
        </section>
      </main>

      <footer className="mt-12 py-5 px-8 border-t border-[var(--color-rule)] text-center text-[0.72rem] tracking-[0.12em] text-[var(--color-fg-faint)] flex flex-wrap justify-center gap-3">
        <span>Orpheus v{VERSION}</span>
        <span className="text-[var(--color-rule)]">·</span>
        <span>audit the code before trusting it with a real wallet</span>
        <span className="text-[var(--color-rule)]">·</span>
        <a
          href="https://github.com/dougnukem/orpheus"
          className="text-[var(--color-bronze)] hover:text-[var(--color-bronze-lit)] no-underline"
        >
          github.com/dougnukem/orpheus
        </a>
      </footer>
    </div>
  );
}

function PanelHead({ num, title, lede }: { num: string; title: string; lede: React.ReactNode }) {
  return (
    <header className="mb-8 max-w-[66ch]">
      <p className="font-serif text-[0.95rem] tracking-[0.15em] text-[var(--color-bronze)] m-0">
        {num}.
      </p>
      <h2 className="font-serif font-medium text-[2.2rem] leading-tight text-[var(--color-fg)] mt-1 mb-3">
        {title}
      </h2>
      <p className="m-0 text-[var(--color-fg-dim)] text-[0.92rem] leading-relaxed">{lede}</p>
    </header>
  );
}

function StatusLine({ status }: { status: Status }) {
  return (
    <span
      className={cn(
        "text-[0.82rem]",
        status.kind === "ok" && "text-[var(--color-verdigris)]",
        status.kind === "err" && "text-[var(--color-rust)]",
        status.kind !== "ok" && status.kind !== "err" && "text-[var(--color-fg-faint)]",
      )}
    >
      {status.text}
    </span>
  );
}

function ScanPanel({ onResults }: { onResults: (r: WalletScanResult[]) => void }) {
  const [files, setFiles] = useState<File[]>([]);
  const [passwords, setPasswords] = useState("");
  const [provider, setProvider] = useState("mock");
  const { status, setStatus } = useStatus();

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!files.length) {
      setStatus({ kind: "err", text: "no files selected" });
      return;
    }
    setStatus({ kind: "busy", text: "descending…" });
    try {
      const body = await api.scan(files, passwords, provider);
      onResults(body.results);
      setStatus({ kind: "ok", text: `returned with ${body.results.length} wallet(s)` });
    } catch (err) {
      setStatus({ kind: "err", text: (err as Error).message });
    }
  };

  return (
    <article className="panel-enter">
      <PanelHead
        num="I"
        title="Scan a directory"
        lede={<>
          Upload one or more wallet files — <code>wallet.dat</code>,{" "}
          <code>.wallet</code>, <code>.aes.json</code>, JSON dumps, plain seed phrases.
          Orpheus dispatches each to the matching extractor and resolves balances via
          the chosen provider.
        </>}
      />
      <form onSubmit={onSubmit} className="flex flex-col gap-5">
        <Dropzone files={files} onChange={setFiles} />
        <div className="grid grid-cols-1 md:grid-cols-[220px_1fr] gap-4">
          <Field label="Balance provider">
            <Select value={provider} onChange={(e) => setProvider(e.target.value)}>
              <option value="mock">mock (offline)</option>
              <option value="none">none</option>
              <option value="blockchain">blockchain.info</option>
              <option value="blockstream">blockstream.info</option>
            </Select>
          </Field>
          <Field label="Passwords (one per line, optional)">
            <TextArea
              value={passwords}
              onChange={(e) => setPasswords(e.target.value)}
              placeholder={"orpheus-demo\nmy-old-passphrase"}
            />
          </Field>
        </div>
        <div className="flex items-center gap-5 pt-1">
          <PrimaryButton type="submit" disabled={status.kind === "busy"}>
            Begin descent
          </PrimaryButton>
          <StatusLine status={status} />
        </div>
      </form>
    </article>
  );
}

function ExtractPanel({ onResults }: { onResults: (r: WalletScanResult[]) => void }) {
  const [files, setFiles] = useState<File[]>([]);
  const [passwords, setPasswords] = useState("");
  const { status, setStatus } = useStatus();

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!files.length) {
      setStatus({ kind: "err", text: "select a wallet file" });
      return;
    }
    setStatus({ kind: "busy", text: "opening…" });
    try {
      const body = await api.scan(files, passwords, "mock");
      onResults(body.results);
      setStatus({ kind: "ok", text: "done" });
    } catch (err) {
      setStatus({ kind: "err", text: (err as Error).message });
    }
  };

  return (
    <article className="panel-enter">
      <PanelHead
        num="II"
        title="Extract a single wallet"
        lede="For when you know exactly which file holds the key. Same engines as Scan, scoped to one file."
      />
      <form onSubmit={onSubmit} className="flex flex-col gap-5">
        <Dropzone files={files} onChange={setFiles} multiple={false} title="One wallet file" />
        <Field label="Passwords (one per line, optional)">
          <TextArea value={passwords} onChange={(e) => setPasswords(e.target.value)} />
        </Field>
        <div className="flex items-center gap-5">
          <PrimaryButton type="submit" disabled={status.kind === "busy"}>
            Open the vault
          </PrimaryButton>
          <StatusLine status={status} />
        </div>
      </form>
    </article>
  );
}

function MnemonicPanel({ onKeys }: { onKeys: (keys: ExtractedKey[]) => void }) {
  const [phrase, setPhrase] = useState("");
  const [kind, setKind] = useState<"bip39" | "blockchain">("bip39");
  const [passphrase, setPassphrase] = useState("");
  const [gapLimit, setGapLimit] = useState(20);
  const [wordlist, setWordlist] = useState("");
  const [decoded, setDecoded] = useState<DecodedMnemonic | null>(null);
  const { status, setStatus } = useStatus();

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!phrase.trim()) {
      setStatus({ kind: "err", text: "paste a phrase" });
      return;
    }
    setDecoded(null);
    setStatus({ kind: "busy", text: "deriving…" });
    try {
      const body = await api.mnemonic({
        phrase: phrase.trim(),
        kind,
        passphrase,
        gap_limit: gapLimit,
        wordlist: wordlist.trim() || undefined,
      });
      if (body.decoded) {
        setDecoded(body.decoded);
        setStatus({ kind: "ok", text: "decoded" });
      } else if (body.keys) {
        onKeys(body.keys);
        setStatus({ kind: "ok", text: `${body.keys.length} keys derived` });
      }
    } catch (err) {
      setStatus({ kind: "err", text: (err as Error).message });
    }
  };

  return (
    <article className="panel-enter">
      <PanelHead
        num="III"
        title="Derive from a mnemonic"
        lede={<>
          BIP39 phrases derive keys across BIP44, BIP49, BIP84, and — for 2013-era iOS
          wallets — the Breadwallet path <code>m/0'/{"{0,1}"}/x</code>. Legacy
          blockchain.com mnemonics decode to a password via the published word lists.
        </>}
      />
      <form onSubmit={onSubmit} className="flex flex-col gap-5">
        <div className="grid grid-cols-1 md:grid-cols-[1fr_140px] gap-4">
          <Field label="Type">
            <Select value={kind} onChange={(e) => setKind(e.target.value as "bip39" | "blockchain")}>
              <option value="bip39">BIP39 (12–24 words)</option>
              <option value="blockchain">blockchain.com legacy</option>
            </Select>
          </Field>
          <Field label="Gap limit">
            <TextInput
              type="number"
              value={gapLimit}
              onChange={(e) => setGapLimit(parseInt(e.target.value) || 20)}
              min={1}
              max={200}
            />
          </Field>
        </div>
        <Field label="Mnemonic phrase">
          <TextArea
            rows={3}
            value={phrase}
            onChange={(e) => setPhrase(e.target.value)}
            placeholder="legal winner thank year wave sausage worth useful legal winner thank yellow"
          />
        </Field>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Field label="BIP39 passphrase (optional)">
            <TextInput value={passphrase} onChange={(e) => setPassphrase(e.target.value)} />
          </Field>
          {kind === "blockchain" && (
            <Field label="Wordlist path (blockchain.com only)">
              <TextInput
                value={wordlist}
                onChange={(e) => setWordlist(e.target.value)}
                placeholder="/path/to/blockchain_com_v2.txt"
              />
            </Field>
          )}
        </div>
        <div className="flex items-center gap-5">
          <PrimaryButton type="submit" disabled={status.kind === "busy"}>
            Cast the phrase
          </PrimaryButton>
          <StatusLine status={status} />
        </div>

        {decoded && (
          <div className="mt-4 border-l-2 border-[var(--color-bronze)] bg-[var(--color-bg-inset)] px-5 py-4">
            <FieldLabel>
              {decoded.version} — {decoded.word_count} words
            </FieldLabel>
            <p className="font-mono text-[var(--color-verdigris)] text-base break-all m-0 mt-2">
              {decoded.password}
            </p>
            <p className="text-[var(--color-fg-faint)] text-xs mt-3 m-0">
              This password unlocks the blockchain.com wallet.aes.json payload.
            </p>
          </div>
        )}
      </form>
    </article>
  );
}

function ResultsPanel({ results }: { results: WalletScanResult[] | null }) {
  return (
    <article className="panel-enter">
      <PanelHead
        num="IV"
        title="Retrieved from the underworld"
        lede={
          results?.length
            ? `${results.length} wallet file(s) sorted; keep anything showing bronze.`
            : "Nothing retrieved yet. Begin with a scan."
        }
      />
      <ResultsView results={results} />
    </article>
  );
}
