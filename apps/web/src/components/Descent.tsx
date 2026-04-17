import type { TabId } from "@/types";
import { cn } from "@/lib/utils";

const STEPS: { id: TabId; num: string; label: string; sub: string }[] = [
  { id: "scan",     num: "I",   label: "Scan",     sub: "directory of wallets" },
  { id: "extract",  num: "II",  label: "Extract",  sub: "single wallet file" },
  { id: "mnemonic", num: "III", label: "Mnemonic", sub: "seed phrase derivation" },
  { id: "results",  num: "IV",  label: "Results",  sub: "retrieved keys & balances" },
];

interface Props {
  active: TabId;
  onSelect: (id: TabId) => void;
  onDemo: () => void | Promise<void>;
  demoBusy?: boolean;
}

export function Descent({ active, onSelect, onDemo, demoBusy }: Props) {
  return (
    <nav
      aria-label="Orpheus modes"
      className="sticky top-8 self-start flex flex-col"
    >
      {STEPS.map((step) => {
        const isActive = active === step.id;
        return (
          <button
            key={step.id}
            role="tab"
            aria-selected={isActive}
            onClick={() => onSelect(step.id)}
            className={cn(
              "relative grid grid-cols-[2.2rem_1fr] gap-x-4 py-4 pl-4 text-left border-l transition-[border-color,padding-left,color] duration-200 cursor-pointer",
              isActive
                ? "border-[var(--color-bronze-lit)] pl-5"
                : "border-[var(--color-rule)] hover:border-[var(--color-bronze)] hover:pl-5",
            )}
          >
            {isActive && (
              <span
                aria-hidden
                className="absolute left-[-3px] top-1/2 -translate-y-1/2 size-[6px] rounded-full bg-[var(--color-bronze-lit)] descent-marker"
              />
            )}
            <span
              className={cn(
                "row-span-2 font-serif text-2xl leading-none tracking-wide",
                isActive
                  ? "text-[var(--color-bronze-lit)] opacity-100"
                  : "text-[var(--color-bronze)] opacity-75",
              )}
            >
              {step.num}
            </span>
            <span className="font-serif text-[1.35rem] leading-none text-[var(--color-fg)]">
              {step.label}
            </span>
            <span className="text-[0.7rem] tracking-[0.08em] uppercase text-[var(--color-fg-faint)] mt-1">
              {step.sub}
            </span>
          </button>
        );
      })}

      <button
        onClick={onDemo}
        disabled={demoBusy}
        className={cn(
          "mt-8 mb-6 py-3 px-4 border border-[var(--color-bronze)] text-[var(--color-bronze-lit)]",
          "font-mono text-[0.8rem] uppercase tracking-[0.18em]",
          "transition-[background-color,color] hover:bg-[var(--color-bronze)] hover:text-[var(--color-bg)]",
          "disabled:opacity-50 disabled:cursor-wait",
        )}
      >
        {demoBusy ? "descending…" : "Run offline demo"}
      </button>

      <aside className="border border-dashed border-[var(--color-rule)] px-4 py-4 text-[0.78rem] leading-relaxed text-[var(--color-fg-dim)]">
        <p className="font-serif italic text-[var(--color-rust)] text-sm m-0 mb-1">
          a warning from the guide
        </p>
        <p className="m-0">
          This tool derives private keys from wallet files. Run only on
          trusted, air-gapped machines. Clear scrollback and prefer the
          ephemeral Docker image when recovering wallets with value.
        </p>
      </aside>
    </nav>
  );
}
