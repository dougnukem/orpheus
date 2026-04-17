import { useState } from "react";
import { Button } from "./Button";

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

  const onClick = async () => {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
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
