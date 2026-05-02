import type { ComposePrefill } from "@/stores/compose.store";

function decodeAddressPart(value: string) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function parseAddressList(value: string | null) {
  if (!value) return [];
  return decodeAddressPart(value)
    .split(/[;,]/)
    .map((address) => address.trim())
    .filter(Boolean);
}

export function parseMailtoUrl(url: string): ComposePrefill | null {
  if (!url.toLowerCase().startsWith("mailto:")) return null;

  const withoutScheme = url.slice("mailto:".length);
  const queryStart = withoutScheme.indexOf("?");
  const path = queryStart >= 0 ? withoutScheme.slice(0, queryStart) : withoutScheme;
  const query = queryStart >= 0 ? withoutScheme.slice(queryStart + 1) : "";
  const params = new URLSearchParams(query);

  return {
    to: [...parseAddressList(path), ...parseAddressList(params.get("to"))],
    cc: parseAddressList(params.get("cc")),
    bcc: parseAddressList(params.get("bcc")),
    subject: params.get("subject") ?? "",
    body: params.get("body") ?? "",
  };
}
