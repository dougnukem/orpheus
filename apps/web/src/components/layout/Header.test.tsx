import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Header } from "./Header";

describe("Header", () => {
  it("renders the brand, tabs, and provider indicator", () => {
    render(
      <MemoryRouter initialEntries={["/scan"]}>
        <Header provider="blockstream" onProviderChange={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByText("Orpheus")).toBeInTheDocument();
    expect(screen.getByText("Wallet files")).toBeInTheDocument();
    expect(screen.getByText("Mnemonic")).toBeInTheDocument();
    expect(screen.getByText("Results")).toBeInTheDocument();
  });

  it("shows an amber dot for the non-default network provider", () => {
    render(
      <MemoryRouter initialEntries={["/scan"]}>
        <Header provider="blockchain" onProviderChange={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByTestId("provider-dot")).toHaveClass(
      "bg-[var(--color-warn)]",
    );
  });

  it("shows a grey dot for mock/none providers", () => {
    render(
      <MemoryRouter initialEntries={["/scan"]}>
        <Header provider="none" onProviderChange={() => {}} />
      </MemoryRouter>,
    );
    expect(screen.getByTestId("provider-dot")).toHaveClass(
      "bg-[var(--color-text-faint)]",
    );
  });

  it("renders 'offline' labels for both mock and none options", () => {
    render(
      <MemoryRouter initialEntries={["/scan"]}>
        <Header provider="none" onProviderChange={() => {}} />
      </MemoryRouter>,
    );
    const offlineLabels = screen.getAllByText(/offline/i);
    expect(offlineLabels.length).toBeGreaterThanOrEqual(2);
  });
});
