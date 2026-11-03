import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { RuleRow, Segmented, Switch } from "./controls";

describe("Switch", () => {
  it("reports its state to assistive tech and flips on click", () => {
    function Host() {
      const [on, setOn] = useState(false);
      return <Switch checked={on} onChange={setOn} label="Kill Switch" />;
    }
    render(<Host />);
    const sw = screen.getByRole("switch", { name: "Kill Switch" });
    expect(sw.getAttribute("aria-checked")).toBe("false");
    fireEvent.click(sw);
    expect(sw.getAttribute("aria-checked")).toBe("true");
  });
});

describe("Segmented", () => {
  it("is a radio group with exactly one checked option", () => {
    function Host() {
      const [v, setV] = useState<"a" | "b">("a");
      return <Segmented label="مسیر" value={v} onChange={setV} options={[{ value: "a", label: "هوشمند" }, { value: "b", label: "سراسری" }]} />;
    }
    render(<Host />);
    fireEvent.click(screen.getByRole("radio", { name: "سراسری" }));
    expect(screen.getByRole("radio", { name: "سراسری" }).getAttribute("aria-checked")).toBe("true");
    expect(screen.getByRole("radio", { name: "هوشمند" }).getAttribute("aria-checked")).toBe("false");
  });
});

describe("RuleRow", () => {
  it("is a pressable button when it can be picked, and says when it is", () => {
    const picked: string[] = [];
    render(<RuleRow code="DE" title="آلمان" end="121 ms" selected onClick={() => picked.push("de")} />);
    const row = screen.getByRole("button", { name: /آلمان/ });
    expect(row.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(row);
    expect(picked).toEqual(["de"]);
  });
});
