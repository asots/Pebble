export interface ComposeDraftInput {
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  rawSource: string;
  richTextHtml: string;
}

function hasNonEmptyAddress(addresses: string[]): boolean {
  return addresses.some((address) => address.trim().length > 0);
}

function hasVisibleText(html: string): boolean {
  return html
    .replace(/<style[\s\S]*?<\/style>/gi, "")
    .replace(/<script[\s\S]*?<\/script>/gi, "")
    .replace(/<br\s*\/?>/gi, " ")
    .replace(/&nbsp;/gi, " ")
    .replace(/<[^>]+>/g, "")
    .trim().length > 0;
}

export function hasComposeDraft(input: ComposeDraftInput): boolean {
  return (
    hasNonEmptyAddress(input.to) ||
    hasNonEmptyAddress(input.cc) ||
    hasNonEmptyAddress(input.bcc) ||
    input.subject.trim().length > 0 ||
    input.rawSource.trim().length > 0 ||
    hasVisibleText(input.richTextHtml)
  );
}
