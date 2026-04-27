import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import TitleBar from "../../src/components/TitleBar";

const windowMock = {
  close: vi.fn(),
  minimize: vi.fn(),
  toggleMaximize: vi.fn(),
};

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => windowMock,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

vi.mock("../../src/stores/compose.store", () => ({
  isComposeDirty: () => false,
}));

vi.mock("../../src/stores/confirm.store", () => ({
  useConfirmStore: {
    getState: () => ({
      confirm: vi.fn(),
    }),
  },
}));

vi.mock("../../src/lib/i18n", () => ({
  default: {
    t: (_key: string, fallback?: string) => fallback ?? _key,
  },
}));

describe("TitleBar", () => {
  it("renders the app logo as a transparent custom titlebar image", () => {
    const { container } = render(<TitleBar />);

    expect(screen.getByText("Pebble")).toBeTruthy();

    const logo = container.querySelector("img[aria-hidden='true']");
    expect(logo).not.toBeNull();
    expect(logo?.className).toContain("bg-transparent");
    expect(logo?.getAttribute("draggable")).toBe("false");
  });
});
