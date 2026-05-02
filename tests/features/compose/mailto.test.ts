import { describe, expect, it } from "vitest";
import { parseMailtoUrl } from "../../../src/features/compose/mailto";

describe("parseMailtoUrl", () => {
  it("parses a simple mailto recipient", () => {
    expect(parseMailtoUrl("mailto:zd423@qq.com")).toEqual({
      to: ["zd423@qq.com"],
      cc: [],
      bcc: [],
      subject: "",
      body: "",
    });
  });

  it("parses decoded recipients and compose fields", () => {
    expect(parseMailtoUrl(
      "mailto:alice@example.com,bob@example.com?cc=carol%40example.com&bcc=dave%40example.com&subject=Hello%20there&body=Line%201%0ALine%202",
    )).toEqual({
      to: ["alice@example.com", "bob@example.com"],
      cc: ["carol@example.com"],
      bcc: ["dave@example.com"],
      subject: "Hello there",
      body: "Line 1\nLine 2",
    });
  });

  it("ignores non-mailto urls", () => {
    expect(parseMailtoUrl("https://example.com")).toBeNull();
  });
});
