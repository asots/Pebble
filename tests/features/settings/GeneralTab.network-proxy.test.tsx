import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import GeneralTab from "../../../src/features/settings/GeneralTab";
import { getGlobalProxy, updateGlobalProxy } from "../../../src/lib/api";
import { useUIStore } from "../../../src/stores/ui.store";

vi.mock("react-i18next", () => ({
  initReactI18next: {
    type: "3rdParty",
    init: vi.fn(),
  },
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const labels: Record<string, string> = {
        "common.save": "Save",
        "common.saving": "Saving...",
        "settings.globalProxy": "Global Proxy",
        "settings.globalProxyDesc": "Used by inherited network requests.",
        "settings.globalProxyHost": "SOCKS5 Proxy",
        "settings.globalProxyPort": "Port",
        "settings.globalProxySaved": "Global proxy saved",
      };
      return labels[key] ?? fallback ?? key;
    },
  }),
}));

vi.mock("../../../src/lib/api", () => ({
  getGlobalProxy: vi.fn(),
  updateGlobalProxy: vi.fn(),
}));

describe("GeneralTab global proxy", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUIStore.setState({
      pollInterval: 15,
      realtimeMode: "realtime",
      showFolderUnreadCount: false,
      notificationsEnabled: true,
    });
    vi.mocked(getGlobalProxy).mockResolvedValue({ host: "127.0.0.1", port: 7890 });
    vi.mocked(updateGlobalProxy).mockResolvedValue(undefined);
  });

  it("loads and saves the encrypted global proxy setting", async () => {
    render(<GeneralTab />);

    await waitFor(() => {
      expect(getGlobalProxy).toHaveBeenCalled();
    });
    expect((screen.getByLabelText("SOCKS5 Proxy") as HTMLInputElement).value).toBe("127.0.0.1");
    expect((screen.getByLabelText("Port") as HTMLInputElement).value).toBe("7890");

    fireEvent.change(screen.getByLabelText("SOCKS5 Proxy"), {
      target: { value: "10.0.0.2" },
    });
    fireEvent.change(screen.getByLabelText("Port"), {
      target: { value: "1080" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(updateGlobalProxy).toHaveBeenCalledWith("10.0.0.2", 1080);
    });
  });

  it("clears the global proxy when both fields are blank", async () => {
    vi.mocked(getGlobalProxy).mockResolvedValue(null);

    render(<GeneralTab />);

    await waitFor(() => {
      expect(getGlobalProxy).toHaveBeenCalled();
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(updateGlobalProxy).toHaveBeenCalledWith(undefined, undefined);
    });
  });
});
