import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

describe("scrollbar CSS", () => {
  it("uses a themed stable scrollbar for scroll regions", () => {
    const css = readFileSync(join(process.cwd(), "src", "styles", "index.css"), "utf8");

    expect(css).toContain("--color-scrollbar-thumb");
    expect(css).toMatch(/\.scroll-region\s*\{[^}]*scrollbar-width\s*:\s*thin/i);
    expect(css).toMatch(/\.scroll-region\s*\{[^}]*scrollbar-gutter\s*:\s*stable/i);
    expect(css).toMatch(/\.scroll-region::-webkit-scrollbar-thumb/i);
    expect(css).toMatch(/\.scroll-region:hover::-webkit-scrollbar-thumb/i);
  });

  it("uses a compact vertical-only scrollbar for the settings panel", () => {
    const css = readFileSync(join(process.cwd(), "src", "styles", "index.css"), "utf8");

    expect(css).toMatch(/\.settings-panel-scroll\s*\{[^}]*scrollbar-gutter\s*:\s*stable/i);
    expect(css).toMatch(/\.settings-panel-scroll::-webkit-scrollbar\s*\{[^}]*width\s*:\s*8px/i);
    expect(css).toMatch(/\.settings-panel-scroll::-webkit-scrollbar\s*\{[^}]*height\s*:\s*0/i);
    expect(css).toMatch(/\.settings-panel-scroll::-webkit-scrollbar-thumb\s*\{[^}]*border\s*:\s*2px solid transparent/i);
  });
});
