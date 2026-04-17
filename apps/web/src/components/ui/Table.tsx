import { useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

export interface Column<T> {
  key: string;
  header: ReactNode;
  align?: "left" | "right";
  render?: (row: T) => ReactNode;
  sortValue?: (row: T) => number | string;
}

type SortDir = "asc" | "desc";

export function Table<T>({
  columns,
  rows,
  rowKey,
  onRowClick,
  className,
}: {
  columns: Column<T>[];
  rows: T[];
  rowKey: (row: T) => string;
  onRowClick?: (row: T) => void;
  className?: string;
}) {
  const [sortKey, setSortKey] = useState<string | null>(null);
  const [sortDir, setSortDir] = useState<SortDir>("asc");

  const sortable = (col: Column<T>) => col.sortValue != null;

  const sorted = (() => {
    if (!sortKey) return rows;
    const col = columns.find((c) => c.key === sortKey);
    if (!col?.sortValue) return rows;
    const get = col.sortValue;
    return [...rows].sort((a, b) => {
      const av = get(a);
      const bv = get(b);
      if (av < bv) return sortDir === "asc" ? -1 : 1;
      if (av > bv) return sortDir === "asc" ? 1 : -1;
      return 0;
    });
  })();

  const toggleSort = (key: string) => {
    if (sortKey === key) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortDir("asc");
    }
  };

  return (
    <table className={cn("w-full text-sm border-collapse", className)}>
      <thead>
        <tr>
          {columns.map((c) => (
            <th
              key={c.key}
              scope="col"
              aria-sort={
                sortKey === c.key
                  ? sortDir === "asc"
                    ? "ascending"
                    : "descending"
                  : undefined
              }
              onClick={sortable(c) ? () => toggleSort(c.key) : undefined}
              className={cn(
                "text-[10px] uppercase tracking-[0.08em] font-normal " +
                  "text-[var(--color-text-faint)] border-b border-[var(--color-border)] " +
                  "py-2 px-2",
                c.align === "right" ? "text-right" : "text-left",
                sortable(c) &&
                  "cursor-pointer select-none hover:text-[var(--color-text)]",
              )}
            >
              {c.header}
            </th>
          ))}
        </tr>
      </thead>
      <tbody>
        {sorted.map((r) => (
          <tr
            key={rowKey(r)}
            onClick={onRowClick ? () => onRowClick(r) : undefined}
            className={cn(
              "border-b border-[var(--color-border)]",
              onRowClick && "cursor-pointer hover:bg-[var(--color-surface)]",
            )}
          >
            {columns.map((c) => (
              <td
                key={c.key}
                className={cn(
                  "py-2 px-2 tabular-nums",
                  c.align === "right" ? "text-right" : "text-left",
                )}
              >
                {c.render
                  ? c.render(r)
                  : ((r as Record<string, unknown>)[c.key] as ReactNode)}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
