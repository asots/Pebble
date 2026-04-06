import { describe, expect, it } from "vitest";
import { hasComposeDraft } from "../../src/features/compose/compose-draft";

describe("hasComposeDraft", () => {
  it("treats rich-text body content as a draft even when raw source is empty", () => {
    expect(
      hasComposeDraft({
        to: [],
        cc: [],
        bcc: [],
        subject: "",
        rawSource: "",
        richTextHtml: "<p>Hello team</p>",
      }),
    ).toBe(true);
  });

  it("counts cc and bcc recipients as draft content", () => {
    expect(
      hasComposeDraft({
        to: [],
        cc: ["cc@example.com"],
        bcc: ["bcc@example.com"],
        subject: "",
        rawSource: "",
        richTextHtml: "<p></p>",
      }),
    ).toBe(true);
  });

  it("ignores empty markup and whitespace-only fields", () => {
    expect(
      hasComposeDraft({
        to: ["   "],
        cc: [],
        bcc: [],
        subject: " ",
        rawSource: "   ",
        richTextHtml: "<p><br></p>",
      }),
    ).toBe(false);
  });
});
