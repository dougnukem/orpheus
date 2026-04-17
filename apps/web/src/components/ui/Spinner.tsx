import { cn } from "@/lib/utils";

export function Spinner({
  label = "Loading",
  className,
}: {
  label?: string;
  className?: string;
}) {
  return (
    <span
      role="status"
      aria-label={label}
      className={cn(
        "inline-block h-4 w-4 border-2 border-[var(--color-border)] " +
          "border-t-[var(--color-accent)] rounded-full animate-spin",
        className,
      )}
    />
  );
}
