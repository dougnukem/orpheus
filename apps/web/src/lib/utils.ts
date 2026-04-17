import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function satToBtc(sat: number | null | undefined): string {
  return ((sat ?? 0) / 1e8).toFixed(8);
}

export function shortenPath(p: string, max = 60): string {
  if (!p) return "";
  if (p.length <= max) return p;
  const parts = p.split("/");
  if (parts.length < 3) return p;
  return `…/${parts.slice(-2).join("/")}`;
}

export function truncateMiddle(s: string, max = 16): string {
  if (!s || s.length <= max) return s ?? "";
  const h = Math.floor(max / 2);
  return `${s.slice(0, h)}…${s.slice(-(max - h - 1))}`;
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KiB`;
  return `${(n / 1024 / 1024).toFixed(1)} MiB`;
}
