import type { ExtractedKey, WalletScanResult } from "@/types";
import { cn, satToBtc, shortenPath, truncateMiddle } from "@/lib/utils";

export function ResultsView({ results }: { results: WalletScanResult[] | null }) {
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

  const totalKeys = results.reduce((s, r) => s + r.keys.length, 0);
  const totalSat = results.reduce(
    (s, r) => s + r.keys.reduce((a, k) => a + (k.balance_sat ?? 0), 0),
    0,
  );
  const withValue = results.filter((r) =>
    r.keys.some((k) => (k.balance_sat ?? 0) > 0),
  ).length;

  return (
    <div>
      <div className="border border-[var(--color-rule)] px-5 py-4 mb-6 flex flex-wrap gap-x-8 gap-y-2">
        <Stat term="wallets" value={String(results.length)} />
        <Stat term="keys" value={String(totalKeys)} />
        <Stat term="with value" value={String(withValue)} hit />
        <Stat term="recovered" value={`${satToBtc(totalSat)} BTC`} hit />
      </div>

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

function KeysTable({ keys }: { keys: ExtractedKey[] }) {
  const sorted = [...keys].sort(
    (a, b) => (b.balance_sat ?? 0) - (a.balance_sat ?? 0),
  );
  const anyValue = keys.some((k) => (k.balance_sat ?? 0) > 0);
  const rows = anyValue
    ? sorted.filter((k) => (k.balance_sat ?? 0) > 0)
    : sorted.slice(0, 5);

  return (
    <div className="overflow-x-auto">
      <table className="w-full border-collapse text-[0.82rem] tabular-nums">
        <thead>
          <tr className="text-left border-b border-[var(--color-rule)]">
            {["path", "address", "BTC", "txs", "WIF"].map((h) => (
              <th
                key={h}
                className={cn(
                  "py-2 px-2 text-[0.7rem] uppercase tracking-[0.12em] font-normal text-[var(--color-fg-faint)]",
                  h === "BTC" || h === "txs" ? "text-right" : "",
                )}
              >
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((k, idx) => {
            const hit = (k.balance_sat ?? 0) > 0;
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
                    hit ? "text-[var(--color-verdigris)]" : "text-[var(--color-fg-dim)]",
                  )}
                >
                  {satToBtc(k.balance_sat)}
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
