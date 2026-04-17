import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { QRCode } from "./QRCode";

describe("QRCode", () => {
  it("renders an SVG with width and aria-label", () => {
    const { container, getByLabelText } = render(
      <QRCode value="bc1qxyz" size={128} />,
    );
    const svg = container.querySelector("svg");
    expect(svg).toBeInTheDocument();
    expect(svg).toHaveAttribute("width", "128");
    expect(getByLabelText(/bc1qxyz/)).toBeInTheDocument();
  });

  it("renders one <rect> per dark module", () => {
    const { container } = render(<QRCode value="x" size={100} />);
    const rects = container.querySelectorAll("rect");
    expect(rects.length).toBeGreaterThan(10);
  });
});
