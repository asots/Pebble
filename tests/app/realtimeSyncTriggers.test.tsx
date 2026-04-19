import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useRealtimeSyncTriggers } from "../../src/app/useRealtimeSyncTriggers";
import { startSync, triggerSync } from "../../src/lib/api";

const mocks = vi.hoisted(() => ({
  activeAccountId: "account-1" as string | null,
  accounts: [{ id: "account-1" }, { id: "account-2" }],
  networkStatus: "online" as "online" | "offline",
  pollInterval: 3,
  realtimeMode: "realtime" as "realtime" | "balanced" | "battery" | "manual",
}));

vi.mock("../../src/lib/api", () => ({
  startSync: vi.fn(() => Promise.resolve("started")),
  triggerSync: vi.fn(() => Promise.resolve()),
}));

vi.mock("../../src/stores/mail.store", () => ({
  useMailStore: (selector: (s: { activeAccountId: string | null }) => unknown) =>
    selector({ activeAccountId: mocks.activeAccountId }),
}));

vi.mock("../../src/stores/ui.store", () => ({
  useUIStore: (selector: (s: {
    networkStatus: "online" | "offline";
    pollInterval: number;
    realtimeMode: "realtime" | "balanced" | "battery" | "manual";
  }) => unknown) =>
    selector({
      networkStatus: mocks.networkStatus,
      pollInterval: mocks.pollInterval,
      realtimeMode: mocks.realtimeMode,
    }),
}));

vi.mock("../../src/hooks/queries", () => ({
  useAccountsQuery: () => ({ data: mocks.accounts }),
}));

describe("useRealtimeSyncTriggers", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.activeAccountId = "account-1";
    mocks.accounts = [{ id: "account-1" }, { id: "account-2" }];
    mocks.networkStatus = "online";
    mocks.pollInterval = 3;
    mocks.realtimeMode = "realtime";
  });

  it("ensures sync is running and triggers every account when the window regains focus", async () => {
    renderHook(() => useRealtimeSyncTriggers());

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });

    expect(startSync).toHaveBeenCalledWith("account-1", 3);
    expect(startSync).toHaveBeenCalledWith("account-2", 3);
    await waitFor(() => expect(triggerSync).toHaveBeenCalledWith("account-1", "window_focus"));
    expect(triggerSync).toHaveBeenCalledWith("account-2", "window_focus");
  });

  it("notifies the backend for every account when the window loses focus", () => {
    renderHook(() => useRealtimeSyncTriggers());

    act(() => {
      window.dispatchEvent(new Event("blur"));
    });

    expect(startSync).not.toHaveBeenCalled();
    expect(triggerSync).toHaveBeenCalledWith("account-1", "window_blur");
    expect(triggerSync).toHaveBeenCalledWith("account-2", "window_blur");
  });

  it("does not trigger network recovery sync on initial online mount", () => {
    renderHook(() => useRealtimeSyncTriggers());

    expect(triggerSync).not.toHaveBeenCalledWith("account-1", "network_online");
  });

  it("ensures sync is running and triggers every account when the app transitions from offline to online", async () => {
    mocks.networkStatus = "offline";
    const { rerender } = renderHook(() => useRealtimeSyncTriggers());

    mocks.networkStatus = "online";
    rerender();

    expect(startSync).toHaveBeenCalledWith("account-1", 3);
    expect(startSync).toHaveBeenCalledWith("account-2", 3);
    await waitFor(() => expect(triggerSync).toHaveBeenCalledWith("account-1", "network_online"));
    expect(triggerSync).toHaveBeenCalledWith("account-2", "network_online");
  });

  it("does not start background sync from focus in manual mode", () => {
    mocks.realtimeMode = "manual";
    mocks.pollInterval = 0;
    renderHook(() => useRealtimeSyncTriggers());

    act(() => {
      window.dispatchEvent(new Event("focus"));
    });

    expect(startSync).not.toHaveBeenCalled();
    expect(triggerSync).toHaveBeenCalledWith("account-1", "window_focus");
    expect(triggerSync).toHaveBeenCalledWith("account-2", "window_focus");
  });
});
