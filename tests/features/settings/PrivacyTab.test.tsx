import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import PrivacyTab from "../../../src/features/settings/PrivacyTab";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

vi.mock("../../../src/stores/mail.store", () => ({
  useMailStore: (selector: (state: { activeAccountId: string | null }) => unknown) =>
    selector({ activeAccountId: null }),
}));

vi.mock("../../../src/stores/toast.store", () => ({
  useToastStore: {
    getState: () => ({ addToast: vi.fn() }),
  },
}));

vi.mock("../../../src/lib/api", () => ({
  listTrustedSenders: vi.fn(),
  removeTrustedSender: vi.fn(),
}));

describe("PrivacyTab", () => {
  it("selects relaxed as the default privacy mode when there is no stored preference", () => {
    localStorage.removeItem("pebble-privacy-mode");

    render(<PrivacyTab />);

    expect(screen.getByText("Load external images by default. Trackers are still blocked.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Relaxed" }).getAttribute("style")).toContain(
      "var(--color-accent)",
    );
  });
});
