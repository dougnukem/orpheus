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
  mock: "mock fixtures",
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
