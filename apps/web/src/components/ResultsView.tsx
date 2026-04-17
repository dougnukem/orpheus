import type { ExtractedKey, ScanSummary, WalletScanResult } from "@/types";
import { cn, satToBtc, shortenPath, truncateMiddle } from "@/lib/utils";

interface Props {
  results: WalletScanResult[] | null;
  summary?: ScanSummary | null;
  provider?: string | null;
}

export function ResultsView({ results, summary, provider }: Props) {
  if (results === null) {
    return (
      <p className="font-serif italic text-[var(--color-fg-faint)] text-center text-lg py-16">
        Nothing retrieved yet. Begin with a scan.
      </p>
    );
  }
  if (!results.length) {
    return (
      <p className="font-serif italic text-[var(--color-fg-faint)] text-center text-lg py-12">
        silence in the hall
      </p>
    );
  }

  const computed = summary ?? summariseClientSide(results, provider ?? null);

  return (
    <div>
      <SummaryPanel summary={computed} />

      {results.map((r, i) => (
        <section
          key={`${r.source_file}-${i}`}
          className={cn(
            "py-5 border-t border-[var(--color-rule)] first:border-t-0 first:pt-0",
          )}
        >
          <header className="flex justify-between items-baseline gap-4 mb-3">
            <h3
              className="font-serif text-lg font-medium text-[var(--color-fg)] m-0 truncate"
              title={r.source_file}
            >
              {shortenPath(r.source_file)}
            </h3>
            <div className="text-[0.72rem] tracking-[0.12em] uppercase text-[var(--color-fg-faint)] whitespace-nowrap">
              {r.source_type} · {r.keys.length} keys ·{" "}
              {satToBtc(r.keys.reduce((a, k) => a + (k.balance_sat ?? 0), 0))} BTC
            </div>
          </header>
          {r.error ? (
            <p className="text-[var(--color-rust)] text-sm m-0">error: {r.error}</p>
          ) : r.keys.length ? (
            <KeysTable keys={r.keys} />
          ) : null}
        </section>
      ))}
    </div>
  );
}

function SummaryPanel({ summary }: { summary: ScanSummary }) {
  return (
    <div className="border border-[var(--color-rule)] p-5 mb-6 grid gap-6 md:grid-cols-[auto_1fr]">
      <div className="grid grid-cols-2 gap-x-6 gap-y-2 content-start">
        <Stat term="wallets" value={String(summary.wallets)} />
        <Stat term="keys" value={String(summary.total_keys)} />
        <Stat term="addresses" value={String(summary.unique_addresses)} />
        <Stat term="funded" value={String(summary.funded_addresses)} hit={summary.funded_addresses > 0} />
        <Stat term="spent (empty)" value={String(summary.spent_addresses)} />
        <Stat term="unfunded" value={String(summary.unfunded_addresses)} />
      </div>

      <div className="grid gap-3 content-start border-l border-[var(--color-rule)] pl-6">
        <LedgerLine label="total received" sat={summary.total_received_sat} />
        <LedgerLine label="total sent" sat={summary.total_sent_sat} />
        <LedgerLine label="current balance" sat={summary.total_balance_sat} hit />
        <div className="mt-2 text-[0.7rem] uppercase tracking-[0.12em] text-[var(--color-fg-faint)]">
          balance provider:{" "}
          <span className="text-[var(--color-fg-dim)]">
            {summary.provider ?? "none (skipped)"}
          </span>
        </div>
      </div>

      {summary.by_source_type.length > 0 && (
        <div className="md:col-span-2 border-t border-[var(--color-rule)] pt-4">
          <div className="text-[0.7rem] uppercase tracking-[0.12em] text-[var(--color-fg-faint)] mb-2">
            by source type
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-x-6 gap-y-1 text-[0.82rem] tabular-nums">
            {summary.by_source_type.map((s) => (
              <div key={s.source_type} className="flex justify-between gap-3">
                <span className="text-[var(--color-fg-dim)]">{s.source_type}</span>
                <span className="text-[var(--color-fg-faint)]">
                  {s.wallets}w · {s.keys}k ·{" "}
                  <span
                    className={cn(
                      s.balance_sat > 0
                        ? "text-[var(--color-bronze-lit)]"
                        : "text-[var(--color-fg-dim)]",
                    )}
                  >
                    {satToBtc(s.balance_sat)} BTC
                  </span>
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function Stat({ term, value, hit }: { term: string; value: string; hit?: boolean }) {
  return (
    <dl className="m-0 flex items-baseline gap-2">
      <dt className="text-[0.7rem] uppercase tracking-[0.12em] text-[var(--color-fg-faint)]">
        {term}
      </dt>
      <dd
        className={cn(
          "m-0 font-serif text-[1.35rem]",
          hit ? "text-[var(--color-bronze-lit)]" : "text-[var(--color-fg)]",
        )}
      >
        {value}
      </dd>
    </dl>
  );
}

function LedgerLine({ label, sat, hit }: { label: string; sat: number; hit?: boolean }) {
  return (
    <div className="flex justify-between items-baseline gap-4">
      <span className="text-[0.72rem] uppercase tracking-[0.15em] text-[var(--color-fg-faint)]">
        {label}
      </span>
      <span
        className={cn(
          "font-serif text-[1.4rem] tabular-nums",
          hit ? "text-[var(--color-bronze-lit)]" : "text-[var(--color-fg)]",
        )}
      >
        {satToBtc(sat)} BTC
      </span>
    </div>
  );
}

function KeysTable({ keys }: { keys: ExtractedKey[] }) {
  const sorted = [...keys].sort((a, b) => {
    const bBal = (b.balance_sat ?? 0) - (a.balance_sat ?? 0);
    if (bBal !== 0) return bBal;
    return (b.total_received_sat ?? 0) - (a.total_received_sat ?? 0);
  });
  const anyActivity = keys.some(
    (k) => (k.balance_sat ?? 0) > 0 || (k.total_received_sat ?? 0) > 0,
  );
  const rows = anyActivity
    ? sorted.filter(
        (k) => (k.balance_sat ?? 0) > 0 || (k.total_received_sat ?? 0) > 0,
      )
    : sorted.slice(0, 5);

  const headers: [string, boolean][] = [
    ["path", false],
    ["address", false],
    ["received", true],
    ["sent", true],
    ["balance", true],
    ["txs", true],
    ["WIF", false],
  ];

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[0.82rem] tabular-nums">
        <thead>
          <tr className="text-left border-b border-[var(--color-rule)]">
            {headers.map(([h, right]) => (
              <th
                key={h}
                className={cn(
                  "py-2 px-2 text-[0.7rem] uppercase tracking-[0.12em] font-normal text-[var(--color-fg-faint)]",
                  right && "text-right",
                )}
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((k, idx) => {
            const balance = k.balance_sat ?? 0;
            const received = k.total_received_sat ?? 0;
            const hit = balance > 0;
            const hadActivity = received > 0;
            return (
              <tr
                key={`${k.wif || k.address_compressed}-${idx}`}
                className={cn(
                  "border-b border-[#20150e]",
                  hit && "bg-gradient-to-r from-[rgba(145,168,123,0.08)] to-transparent",
                )}
                style={hit ? { boxShadow: "inset 3px 0 0 var(--color-verdigris)" } : undefined}
              >
                <td className="py-2 px-2 text-[var(--color-bronze)] whitespace-nowrap">
                  {k.derivation_path ?? "—"}
                </td>
                <td
                  className={cn(
                    "py-2 px-2 break-all",
                    hit ? "text-[var(--color-fg)]" : "text-[var(--color-fg-dim)]",
                  )}
                >
                  {k.address_compressed}
                </td>
                <td
                  className={cn(
                    "py-2 px-2 text-right",
                    hadActivity ? "text-[var(--color-fg-dim)]" : "text-[var(--color-fg-faint)]",
                  )}
                >
                  {satToBtc(received)}
                </td>
                <td
                  className={cn(
                    "py-2 px-2 text-right",
                    (k.total_sent_sat ?? 0) > 0
                      ? "text-[var(--color-rust)]"
                      : "text-[var(--color-fg-faint)]",
                  )}
                >
                  {satToBtc(k.total_sent_sat)}
                </td>
                <td
                  className={cn(
                    "py-2 px-2 text-right",
                    hit ? "text-[var(--color-verdigris)]" : "text-[var(--color-fg-dim)]",
                  )}
                >
                  {satToBtc(balance)}
                </td>
                <td className="py-2 px-2 text-right text-[var(--color-fg-dim)]">
                  {k.tx_count ?? 0}
                </td>
                <td className="py-2 px-2 text-[var(--color-fg-faint)] text-[0.78rem]">
                  {truncateMiddle(k.wif, 20)}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function summariseClientSide(
  results: WalletScanResult[],
  provider: string | null,
): ScanSummary {
  const byType = new Map<string, { wallets: number; keys: number; balance_sat: number }>();
  const unique = new Set<string>();
  let funded = 0;
  let spent = 0;
  let unfunded = 0;
  let received = 0;
  let sent = 0;
  let balance = 0;
  let totalKeys = 0;

  for (const r of results) {
    const agg = byType.get(r.source_type) ?? { wallets: 0, keys: 0, balance_sat: 0 };
    agg.wallets += 1;
    agg.keys += r.keys.length;
    totalKeys += r.keys.length;
    for (const k of r.keys) {
      const kBalance = k.balance_sat ?? 0;
      const kReceived = k.total_received_sat ?? 0;
      const kSent = k.total_sent_sat ?? 0;
      agg.balance_sat += kBalance;
      balance += kBalance;
      received += kReceived;
      sent += kSent;
      if (!unique.has(k.address_compressed)) {
        unique.add(k.address_compressed);
        if (kBalance > 0) funded += 1;
        else if (kReceived > 0) spent += 1;
        else unfunded += 1;
      }
    }
    byType.set(r.source_type, agg);
  }

  return {
    wallets: results.length,
    total_keys: totalKeys,
    unique_addresses: unique.size,
    funded_addresses: funded,
    spent_addresses: spent,
    unfunded_addresses: unfunded,
    total_received_sat: received,
    total_sent_sat: sent,
    total_balance_sat: balance,
    by_source_type: Array.from(byType.entries()).map(([source_type, a]) => ({
      source_type: source_type as ScanSummary["by_source_type"][number]["source_type"],
      wallets: a.wallets,
      keys: a.keys,
      balance_sat: a.balance_sat,
    })),
    provider,
  };
}
