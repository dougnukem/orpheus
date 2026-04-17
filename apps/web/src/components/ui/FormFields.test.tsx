import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Input, Textarea, Select, Field } from "./FormFields";

describe("Input", () => {
  it("forwards value and onChange", async () => {
    let captured = "";
    render(<Input value="" onChange={(e) => (captured = e.target.value)} />);
    await userEvent.type(screen.getByRole("textbox"), "hi");
    expect(captured).toBe("i");
  });
});

describe("Textarea", () => {
  it("forwards value and rows", () => {
    render(<Textarea defaultValue="hello" rows={4} />);
    const ta = screen.getByRole("textbox") as HTMLTextAreaElement;
    expect(ta.value).toBe("hello");
    expect(ta.rows).toBe(4);
  });
});

describe("Select", () => {
  it("renders children and forwards value", () => {
    render(
      <Select value="a" onChange={() => {}}>
        <option value="a">A</option>
        <option value="b">B</option>
      </Select>,
    );
    const sel = screen.getByRole("combobox") as HTMLSelectElement;
    expect(sel.value).toBe("a");
  });
});

describe("Field", () => {
  it("renders a label above children", () => {
    render(
      <Field label="Provider">
        <Input />
      </Field>,
    );
    expect(screen.getByText("Provider")).toBeInTheDocument();
    expect(screen.getByRole("textbox")).toBeInTheDocument();
  });
});
