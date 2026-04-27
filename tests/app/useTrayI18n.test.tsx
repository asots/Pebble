import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  language: "zh",
  setTrayMenuLabels: vi.fn().mockResolvedValue(undefined),
  translations: {
    "tray.show": "显示窗口",
    "tray.hide": "隐藏窗口",
    "tray.quit": "退出 Pebble",
  } as Record<string, string>,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback: string) => mocks.translations[key] ?? fallback,
  }),
}));

vi.mock("../../src/stores/ui.store", () => ({
  useUIStore: (selector: (state: { language: string }) => unknown) =>
    selector({ language: mocks.language }),
}));

vi.mock("../../src/lib/api", () => ({
  setTrayMenuLabels: mocks.setTrayMenuLabels,
}));

import { useTrayI18n } from "../../src/app/useTrayI18n";

function Harness() {
  useTrayI18n();
  return null;
}

describe("useTrayI18n", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.language = "zh";
    mocks.translations = {
      "tray.show": "显示窗口",
      "tray.hide": "隐藏窗口",
      "tray.quit": "退出 Pebble",
    };
  });

  it("syncs localized tray menu labels to the native app", async () => {
    render(<Harness />);

    await waitFor(() =>
      expect(mocks.setTrayMenuLabels).toHaveBeenCalledWith("显示窗口", "隐藏窗口", "退出 Pebble"),
    );
  });

  it("resyncs labels when the app language changes", async () => {
    const view = render(<Harness />);

    await waitFor(() => expect(mocks.setTrayMenuLabels).toHaveBeenCalledTimes(1));

    mocks.language = "en";
    mocks.translations = {
      "tray.show": "Show Window",
      "tray.hide": "Hide Window",
      "tray.quit": "Quit Pebble",
    };
    view.rerender(<Harness />);

    await waitFor(() =>
      expect(mocks.setTrayMenuLabels).toHaveBeenLastCalledWith("Show Window", "Hide Window", "Quit Pebble"),
    );
  });
});
