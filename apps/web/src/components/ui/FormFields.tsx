import type { ComponentProps, PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

const control =
  "w-full rounded-[5px] border border-[var(--color-border)] " +
  "bg-[var(--color-surface)] text-[var(--color-text)] " +
  "px-3 py-2 text-sm font-mono transition-colors " +
  "focus:outline-none focus:border-[var(--color-accent)] " +
  "placeholder:text-[var(--color-text-faint)]";

export function Input(props: ComponentProps<"input">) {
  return <input {...props} className={cn(control, props.className)} />;
}

export function Textarea(props: ComponentProps<"textarea">) {
  return (
    <textarea
      rows={3}
      {...props}
      className={cn(control, "resize-y min-h-[3rem]", props.className)}
    />
  );
}

export function Select(props: ComponentProps<"select">) {
  return (
    <select
      {...props}
      className={cn(
        control,
        "appearance-none pr-8 cursor-pointer font-sans",
        props.className,
      )}
    />
  );
}

export function FieldLabel({ children }: PropsWithChildren) {
  return (
    <span className="text-[10px] tracking-[0.08em] uppercase text-[var(--color-text-faint)] mb-1 block font-sans">
      {children}
    </span>
  );
}

export function Field({
  label,
  children,
  className,
}: PropsWithChildren<{ label: string; className?: string }>) {
  return (
    <label className={cn("flex flex-col", className)}>
      <FieldLabel>{label}</FieldLabel>
      {children}
    </label>
  );
}
