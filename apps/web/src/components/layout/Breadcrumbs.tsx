import { Fragment } from "react";
import { NavLink } from "react-router-dom";

export interface Crumb {
  label: string;
  to?: string;
}

export function Breadcrumbs({ segments }: { segments: Crumb[] }) {
  return (
    <nav
      aria-label="Breadcrumb"
      className="text-xs text-[var(--color-text-dim)] mb-4"
    >
      {segments.map((s, i) => {
        const last = i === segments.length - 1;
        return (
          <Fragment key={`${s.label}-${i}`}>
            {i > 0 && (
              <span className="mx-2 text-[var(--color-text-faint)]">/</span>
            )}
            {last || !s.to ? (
              <span className="text-[var(--color-text)]">{s.label}</span>
            ) : (
              <NavLink to={s.to} className="hover:text-[var(--color-text)]">
                {s.label}
              </NavLink>
            )}
          </Fragment>
        );
      })}
    </nav>
  );
}
