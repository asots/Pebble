import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useMailtoOpen } from "../../src/app/useMailtoOpen";
import { useComposeStore } from "../../src/stores/compose.store";
import { useUIStore } from "../../src/stores/ui.store";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mocks.invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mocks.listen,
}));

describe("useMailtoOpen", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.listen.mockReset();
    mocks.unlisten.mockReset();
    mocks.invoke.mockResolvedValue([]);
    mocks.listen.mockResolvedValue(mocks.unlisten);
    useUIStore.setState({ activeView: "inbox", previousView: "inbox" });
    useComposeStore.setState({
      composeMode: null,
      composeReplyTo: null,
      composePrefill: null,
      composeKey: 0,
      composeDirty: false,
      showComposeLeaveConfirm: false,
      pendingView: null,
    });
  });

  it("opens pending mailto urls from the backend queue", async () => {
    mocks.invoke.mockResolvedValue(["mailto:alice@example.com?subject=Hi"]);

    renderHook(() => useMailtoOpen());

    await waitFor(() => {
      expect(useComposeStore.getState().composePrefill).toMatchObject({
        to: ["alice@example.com"],
        subject: "Hi",
      });
    });
    expect(useUIStore.getState().activeView).toBe("compose");
    expect(mocks.invoke).toHaveBeenCalledWith("take_pending_mailto_urls");
  });

  it("opens mailto urls emitted while the app is running", async () => {
    let handler: ((event: { payload: { urls: string[] } }) => void) | undefined;
    mocks.listen.mockImplementation((_eventName, nextHandler) => {
      handler = nextHandler;
      return Promise.resolve(mocks.unlisten);
    });

    renderHook(() => useMailtoOpen());
    handler?.({ payload: { urls: ["mailto:bob@example.com?body=Hello"] } });

    await waitFor(() => {
      expect(useComposeStore.getState().composePrefill).toMatchObject({
        to: ["bob@example.com"],
        body: "Hello",
      });
    });
  });
});
