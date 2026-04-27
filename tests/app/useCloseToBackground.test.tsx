import { render, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  closeHandler: undefined as undefined | ((event: { preventDefault: () => void }) => void | Promise<void>),
  hide: vi.fn().mockResolvedValue(undefined),
  onCloseRequested: vi.fn((handler: (event: { preventDefault: () => void }) => void | Promise<void>) => {
    mocks.closeHandler = handler;
    return Promise.resolve(vi.fn());
  }),
  uiState: {
    keepRunningInBackground: false,
  },
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    hide: mocks.hide,
    onCloseRequested: mocks.onCloseRequested,
  }),
}));

vi.mock("../../src/stores/ui.store", () => ({
  useUIStore: {
    getState: () => mocks.uiState,
  },
}));

import { useCloseToBackground } from "../../src/app/useCloseToBackground";

function Harness() {
  useCloseToBackground();
  return null;
}

describe("useCloseToBackground", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.closeHandler = undefined;
    mocks.uiState.keepRunningInBackground = false;
  });

  it("lets the app close normally when background mode is disabled", async () => {
    render(<Harness />);

    await waitFor(() => expect(mocks.onCloseRequested).toHaveBeenCalledOnce());
    const preventDefault = vi.fn();
    await mocks.closeHandler?.({ preventDefault });

    expect(preventDefault).not.toHaveBeenCalled();
    expect(mocks.hide).not.toHaveBeenCalled();
  });

  it("hides the window to tray instead of closing when background mode is enabled", async () => {
    mocks.uiState.keepRunningInBackground = true;
    render(<Harness />);

    await waitFor(() => expect(mocks.onCloseRequested).toHaveBeenCalledOnce());
    const preventDefault = vi.fn();
    await mocks.closeHandler?.({ preventDefault });

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(mocks.hide).toHaveBeenCalledOnce();
  });
});
