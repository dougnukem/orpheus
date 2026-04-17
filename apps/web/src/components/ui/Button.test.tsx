import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Button } from "./Button";

describe("Button", () => {
  it("renders children and fires onClick", async () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Scan</Button>);
    await userEvent.click(screen.getByRole("button", { name: "Scan" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("does not fire onClick when disabled", async () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Scan
      </Button>,
    );
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).not.toHaveBeenCalled();
  });

  it("applies the primary variant class by default", () => {
    const { container } = render(<Button>Scan</Button>);
    expect(container.firstChild).toHaveClass("bg-[var(--color-accent)]");
  });

  it("applies the secondary variant class", () => {
    const { container } = render(<Button variant="secondary">Cancel</Button>);
    expect(container.firstChild).toHaveClass("border-[var(--color-border)]");
  });

  it("applies the success variant class", () => {
    const { container } = render(<Button variant="success">Import</Button>);
    expect(container.firstChild).toHaveClass("bg-[var(--color-success)]");
  });

  it("applies focus-visible outline classes", () => {
    const { container } = render(<Button>Scan</Button>);
    // Note: tailwind-merge collapses the bare `focus-visible:outline` into
    // the more specific `focus-visible:outline-2`/`outline-[color]`. Those
    // three together still render the 2px accent outline on keyboard focus.
    expect(container.firstChild).toHaveClass(
      "focus-visible:outline-2",
      "focus-visible:outline-offset-2",
      "focus-visible:outline-[var(--color-accent)]",
    );
  });
});
