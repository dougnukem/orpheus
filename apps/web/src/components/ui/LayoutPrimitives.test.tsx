import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Card } from "./Card";
import { Pill } from "./Pill";
import { StatBar } from "./StatBar";
import { Spinner } from "./Spinner";
import { Banner } from "./Banner";

describe("Card", () => {
  it("renders children inside a bordered surface", () => {
    const { container } = render(<Card>body</Card>);
    expect(container.firstChild).toHaveClass("border");
    expect(screen.getByText("body")).toBeInTheDocument();
  });
});

describe("Pill", () => {
  it("renders text and applies the tone color class", () => {
    render(<Pill tone="accent">BIP84</Pill>);
    expect(screen.getByText("BIP84")).toHaveClass("text-[var(--color-accent)]");
  });

  it("defaults to neutral tone", () => {
    render(<Pill>x</Pill>);
    expect(screen.getByText("x")).toHaveClass("text-[var(--color-text-dim)]");
  });
});

describe("StatBar", () => {
  it("renders each stat with a label and value", () => {
    render(
      <StatBar
        stats={[
          { label: "Wallets", value: "4" },
          { label: "Recovered", value: "0.038 BTC", hit: true },
        ]}
      />,
    );
    expect(screen.getByText("Wallets")).toBeInTheDocument();
    expect(screen.getByText("0.038 BTC")).toHaveClass(
      "text-[var(--color-success)]",
    );
  });
});

describe("Spinner", () => {
  it("renders a role=status element with an aria label", () => {
    render(<Spinner label="Scanning" />);
    expect(screen.getByRole("status")).toHaveAccessibleName("Scanning");
  });
});

describe("Banner", () => {
  it("renders the warn variant", () => {
    const { container } = render(<Banner variant="warn">heads up</Banner>);
    expect(container.firstChild).toHaveClass("border-[var(--color-warn)]");
  });

  it("renders the danger variant", () => {
    const { container } = render(<Banner variant="danger">oh no</Banner>);
    expect(container.firstChild).toHaveClass("border-[var(--color-danger)]");
  });

  it("uses role=status for info variant", () => {
    render(<Banner variant="info">fyi</Banner>);
    expect(screen.getByRole("status")).toHaveTextContent("fyi");
  });

  it("uses role=alert for warn and danger variants", () => {
    const { unmount } = render(<Banner variant="warn">heads up</Banner>);
    expect(screen.getByRole("alert")).toHaveTextContent("heads up");
    unmount();
    render(<Banner variant="danger">oh no</Banner>);
    expect(screen.getByRole("alert")).toHaveTextContent("oh no");
  });
});
