export function Masthead({ version }: { version: string }) {
  return (
    <header className="grid grid-cols-[auto_1fr_auto] items-end gap-8 px-8 pt-10 pb-6 border-b border-[var(--color-rule)]">
      <div
        className="font-serif text-[clamp(4rem,8vw,7rem)] leading-[0.8] text-[var(--color-bronze)] glyph-enter"
        aria-hidden
        style={{ textShadow: "0 0 40px rgba(192, 144, 80, 0.25)" }}
      >
        Ω
      </div>

      <div>
        <p className="text-[0.72rem] uppercase tracking-[0.22em] text-[var(--color-fg-faint)] m-0">
          tool for the recovery of forgotten coin
        </p>
        <h1 className="font-serif font-medium text-[clamp(2.5rem,5vw,4rem)] leading-none tracking-tight text-[var(--color-fg)] mt-1">
          Orpheus
        </h1>
        <p className="font-serif italic text-[var(--color-bronze)] mt-2 text-lg">
          καταβαίνω
          <span className="font-mono not-italic text-[0.8rem] uppercase tracking-[0.12em] text-[var(--color-fg-faint)] ml-3">
            to descend
          </span>
        </p>
      </div>

      <div className="text-[0.72rem] tracking-[0.15em] text-[var(--color-fg-faint)] pb-1">
        v{version}
      </div>
    </header>
  );
}
