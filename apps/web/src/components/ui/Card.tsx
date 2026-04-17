import type { PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

export function Card({
  children,
  className,
}: PropsWithChildren<{ className?: string }>) {
  return (
    <div
      className={cn(
        "border border-[var(--color-border)] bg-[var(--color-surface)] " +
          "rounded-[6px] p-4",
        className,
      )}
    >
      {children}
    </div>
  );
}
