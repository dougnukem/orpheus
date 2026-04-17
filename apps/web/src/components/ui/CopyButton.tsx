import { useEffect, useRef, useState } from "react";
import { Button } from "./Button";

const COPIED_LABEL_MS = 1500;

export function CopyButton({
  value,
  label = "Copy",
  className,
}: {
  value: string;
  label?: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);
  const timerRef = useRef<number | null>(null);

  useEffect(
    () => () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    },
    [],
  );

  const onClick = async () => {
    try {
      await navigator.clipboard.writeText(value);
    } catch (err) {
      console.error("CopyButton: clipboard write failed", err);
      return;
    }
    setCopied(true);
    if (timerRef.current !== null) window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(() => {
      setCopied(false);
      timerRef.current = null;
    }, COPIED_LABEL_MS);
  };

  return (
    <Button
      variant="secondary"
      onClick={onClick}
      aria-label={copied ? "Copied" : label}
      className={className}
    >
      {copied ? "Copied" : label}
    </Button>
  );
}
