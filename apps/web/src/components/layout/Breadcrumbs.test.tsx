import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Breadcrumbs } from "./Breadcrumbs";

describe("Breadcrumbs", () => {
  it("renders all segments", () => {
    render(
      <MemoryRouter>
        <Breadcrumbs
          segments={[
            { label: "Results", to: "/results" },
            { label: "wallet.dat", to: "/results/abc" },
            { label: "bc1q…kp8z" },
          ]}
        />
      </MemoryRouter>,
    );
    expect(screen.getByText("Results")).toBeInTheDocument();
    expect(screen.getByText("wallet.dat")).toBeInTheDocument();
    expect(screen.getByText("bc1q…kp8z")).toBeInTheDocument();
  });
});
