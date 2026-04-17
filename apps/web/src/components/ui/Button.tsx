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
