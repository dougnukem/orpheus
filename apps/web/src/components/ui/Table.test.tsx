import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Table, type Column } from "./Table";

interface Row {
  address: string;
  btc: number;
}

const columns: Column<Row>[] = [
  { key: "address", header: "Address" },
  {
    key: "btc",
    header: "BTC",
    align: "right",
    sortValue: (r) => r.btc,
    render: (r) => r.btc.toFixed(8),
  },
];

const rows: Row[] = [
  { address: "bc1qA", btc: 0.5 },
  { address: "bc1qB", btc: 1.2 },
  { address: "bc1qC", btc: 0.01 },
];

describe("Table", () => {
  it("renders rows and columns", () => {
    render(<Table columns={columns} rows={rows} rowKey={(r) => r.address} />);
    expect(screen.getAllByRole("row")).toHaveLength(4);
    expect(screen.getByText("bc1qA")).toBeInTheDocument();
  });

  it("sorts ascending by column when header is clicked once", async () => {
    render(<Table columns={columns} rows={rows} rowKey={(r) => r.address} />);
    await userEvent.click(screen.getByText("BTC"));
    const bodyRows = screen
      .getAllByRole("row")
      .slice(1)
      .map((r) => r.textContent);
    expect(bodyRows[0]).toContain("bc1qC");
    expect(bodyRows[2]).toContain("bc1qB");
  });

  it("sorts descending when header is clicked twice", async () => {
    render(<Table columns={columns} rows={rows} rowKey={(r) => r.address} />);
    await userEvent.click(screen.getByText("BTC"));
    await userEvent.click(screen.getByText("BTC"));
    const bodyRows = screen
      .getAllByRole("row")
      .slice(1)
      .map((r) => r.textContent);
    expect(bodyRows[0]).toContain("bc1qB");
  });
});
