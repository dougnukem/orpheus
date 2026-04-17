import { cn } from "@/lib/utils";

export interface Stat {
  label: string;
  value: string;
  hit?: boolean;
}

export function StatBar({
  stats,
  className,
}: {
  stats: Stat[];
  className?: string;
}) {
  return (
    <div
      className={cn(
        "grid grid-cols-2 md:grid-cols-4 gap-3 " +
          "border border-[var(--color-border)] bg-[var(--color-surface)] " +
          "rounded-[6px] px-4 py-3",
        className,
      )}
    >
      {stats.map((s) => (
        <div key={s.label} className="flex flex-col">
          <span className="text-[10px] uppercase tracking-[0.08em] text-[var(--color-text-faint)]">
            {s.label}
          </span>
          <span
            className={cn(
              "text-base font-semibold tabular-nums",
              s.hit
                ? "text-[var(--color-success)]"
                : "text-[var(--color-text)]",
            )}
          >
            {s.value}
          </span>
        </div>
      ))}
    </div>
  );
}
