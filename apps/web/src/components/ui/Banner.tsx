import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

type Variant = "info" | "warn" | "danger";

const variants: Record<Variant, string> = {
  info: "border-[var(--color-accent)] bg-[color-mix(in_srgb,var(--color-accent)_8%,transparent)]",
  warn: "border-[var(--color-warn)] bg-[color-mix(in_srgb,var(--color-warn)_8%,transparent)]",
  danger:
    "border-[var(--color-danger)] bg-[color-mix(in_srgb,var(--color-danger)_8%,transparent)]",
};

const roles: Record<Variant, "status" | "alert"> = {
  info: "status",
  warn: "alert",
  danger: "alert",
};

export function Banner({
  variant = "info",
  className,
  children,
}: PropsWithChildren<{ variant?: Variant; className?: string }>) {
  return (
    <div
      role={roles[variant]}
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
