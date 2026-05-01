import { describe, expect, it } from "vitest";
import {
  ACCOUNT_COLOR_PRESETS,
  assignAccountColors,
  getAccountColor,
} from "../../src/lib/accountColors";

describe("accountColors", () => {
  it("exposes a curated preset palette", () => {
    expect(ACCOUNT_COLOR_PRESETS.length).toBeGreaterThanOrEqual(10);
    expect(ACCOUNT_COLOR_PRESETS.map((preset) => preset.color)).toContain("#0ea5e9");
    expect(ACCOUNT_COLOR_PRESETS.every((preset) => /^#[0-9a-f]{6}$/.test(preset.color))).toBe(true);
  });

  it("assigns different default preset colors to accounts without saved colors", () => {
    const colorsById = assignAccountColors([
      { id: "account-1", email: "one@example.com", display_name: "One" },
      { id: "account-2", email: "two@example.com", display_name: "Two" },
      { id: "account-3", email: "three@example.com", display_name: "Three" },
    ]);

    const colors = ["account-1", "account-2", "account-3"].map((id) => colorsById.get(id));

    expect(new Set(colors).size).toBe(3);
    expect(colors).toEqual(ACCOUNT_COLOR_PRESETS.slice(0, 3).map((preset) => preset.color));
  });

  it("keeps saved colors and skips them for later default assignments", () => {
    const colorsById = assignAccountColors([
      { id: "account-1", email: "one@example.com", display_name: "One", color: "#0ea5e9" },
      { id: "account-2", email: "two@example.com", display_name: "Two" },
    ]);

    expect(colorsById.get("account-1")).toBe("#0ea5e9");
    expect(colorsById.get("account-2")).toBe(ACCOUNT_COLOR_PRESETS[1].color);
  });

  it("falls back to a stable preset for accounts not present in a list", () => {
    expect(getAccountColor(undefined, "missing-account")).toMatch(/^#[0-9a-f]{6}$/);
  });
});
