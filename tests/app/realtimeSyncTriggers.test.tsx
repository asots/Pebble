import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useRealtimeSyncTriggers } from "../../src/app/useRealtimeSyncTriggers";
import { triggerSync } from "../../src/lib/api";

const mocks = vi.hoisted(() => ({
  activeAccountId: "account-1" as string | null,
  networkStatus: "online" as "online" | "offline",
}));

vi.mock("../../src/lib/api", () => ({
  triggerSync: vi.fn(() => Promise.resolve()),
}));

vi.mock("../../src/stores/mail.store", () => ({
  useMailStore: (selector: (s: { activeAccountId: string | null }) => unknown) =>
    selector({ activeAccountId: mocks.activeAccountId }),
}));

vi.mock("../../src/stores/ui.store", () => ({
  useUIStore: (selector: (s: { networkStatus: "online" | "offline" }) => unknown) =>
    selector({ networkStatus: mocks.networkStatus }),
}));

describe("useRealtimeSyncTriggers", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.activeAccountId = "account-1";
    mocks.networkStatus = "online";
  });

  it("triggers sync when the window regains focus", () => {
    renderHook(() => useRealtimeSyncTriggers());

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });

    expect(triggerSync).toHaveBeenCalledWith("account-1", "window_focus");
  });

  it("notifies the backend when the window loses focus", () => {
    renderHook(() => useRealtimeSyncTriggers());

    act(() => {
      window.dispatchEvent(new Event("blur"));
    });

    expect(triggerSync).toHaveBeenCalledWith("account-1", "window_blur");
  });

  it("does not trigger network recovery sync on initial online mount", () => {
    renderHook(() => useRealtimeSyncTriggers());

    expect(triggerSync).not.toHaveBeenCalledWith("account-1", "network_online");
  });

  it("triggers sync when the app transitions from offline to online", () => {
    mocks.networkStatus = "offline";
    const { rerender } = renderHook(() => useRealtimeSyncTriggers());

    mocks.networkStatus = "online";
    rerender();

    expect(triggerSync).toHaveBeenCalledWith("account-1", "network_online");
  });
});
