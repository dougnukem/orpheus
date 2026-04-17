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
