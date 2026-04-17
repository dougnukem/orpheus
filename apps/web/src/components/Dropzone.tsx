import { useRef, useState } from "react";
import { cn, formatBytes } from "@/lib/utils";

interface Props {
  files: File[];
  onChange: (files: File[]) => void;
  multiple?: boolean;
  title?: string;
}

export function Dropzone({ files, onChange, multiple = true, title = "Drop wallet files here" }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragOver, setDragOver] = useState(false);

  return (
    <div
      onClick={(e) => {
        if ((e.target as HTMLElement).tagName !== "LABEL") inputRef.current?.click();
      }}
      onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragOver(false);
        const dropped = Array.from(e.dataTransfer.files);
        onChange(multiple ? dropped : dropped.slice(0, 1));
      }}
      className={cn(
        "border border-dashed px-6 py-8 text-center cursor-pointer transition-colors",
        dragOver
          ? "border-[var(--color-bronze)] bg-[rgba(192,144,80,0.06)]"
          : "border-[var(--color-rule)] hover:border-[var(--color-bronze)]",
      )}
    >
      <p className="font-serif text-2xl text-[var(--color-fg)] m-0 mb-1">{title}</p>
      <p className="text-[var(--color-fg-faint)] text-sm m-0">
        or{" "}
        <label
          htmlFor="orpheus-file-input"
          className="text-[var(--color-bronze-lit)] border-b border-[var(--color-bronze)] cursor-pointer"
        >
          select {multiple ? "files" : "a file"}
        </label>
      </p>
      <input
        id="orpheus-file-input"
        ref={inputRef}
        type="file"
        className="hidden"
        multiple={multiple}
        onChange={(e) => {
          const list = Array.from(e.target.files ?? []);
          onChange(multiple ? list : list.slice(0, 1));
        }}
      />
      {files.length > 0 && (
        <ul className="mt-4 list-none p-0 flex flex-col gap-1 text-left text-sm tabular-nums text-[var(--color-fg-dim)]">
          {files.map((f) => (
            <li key={f.name} className="before:content-['›_'] before:text-[var(--color-bronze)]">
              {f.name}   {formatBytes(f.size)}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
