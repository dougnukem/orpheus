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
