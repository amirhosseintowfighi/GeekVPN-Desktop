import { describe, expect, it } from "vitest";

describe("Home page", () => {
  it("is importable as a lazy chunk", async () => {
    const mod = await import("../Home");
    expect(mod.Home).toBeDefined();
  });
});
