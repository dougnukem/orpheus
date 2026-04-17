import type { ComponentProps, PropsWithChildren } from "react";
import { cn } from "@/lib/utils";

const inputBase =
  "bg-[var(--color-bg-inset)] border border-[var(--color-rule)] text-[var(--color-fg)] " +
  "px-3 py-2.5 font-mono text-sm transition-colors " +
  "focus:outline-none focus:border-[var(--color-bronze)] focus:bg-[#100703]";

export function FieldLabel({ children }: PropsWithChildren) {
  return (
    <span className="text-[0.72rem] tracking-[0.15em] uppercase text-[var(--color-fg-faint)] mb-1.5 block">
      {children}
    </span>
  );
}

export function Field({
  label,
  className,
  children,
}: PropsWithChildren<{ label: string; className?: string }>) {
  return (
    <label className={cn("flex flex-col", className)}>
      <FieldLabel>{label}</FieldLabel>
      {children}
    </label>
  );
}

export function TextInput(props: ComponentProps<"input">) {
  return <input {...props} className={cn(inputBase, "resize-none", props.className)} />;
}

export function TextArea(props: ComponentProps<"textarea">) {
  return (
    <textarea
      rows={3}
      {...props}
      className={cn(inputBase, "min-h-[3.2rem] resize-y", props.className)}
    />
  );
}

export function Select(props: ComponentProps<"select">) {
  return (
    <select
      {...props}
      className={cn(inputBase, "appearance-none pr-8 cursor-pointer", props.className)}
    />
  );
}

export function PrimaryButton(props: ComponentProps<"button">) {
  return (
    <button
      {...props}
      className={cn(
        "bg-[var(--color-bronze)] text-[var(--color-bg)] px-5 py-2.5",
        "font-mono text-[0.85rem] uppercase tracking-[0.15em]",
        "transition-colors hover:bg-[var(--color-bronze-lit)] active:translate-y-px",
        "disabled:opacity-50 disabled:cursor-wait",
        props.className,
      )}
    />
  );
}
