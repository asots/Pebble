import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  invalidateQueries: vi.fn(),
  uiState: {
    syncStatus: "idle" as "idle" | "syncing" | "error",
    setSyncStatus: vi.fn(),
    networkStatus: "online" as "online" | "offline",
    lastMailError: null as string | null,
    setLastMailError: vi.fn(),
    realtimeStatusByAccount: {},
    setRealtimeStatus: vi.fn(),
    notificationsEnabled: true,
    keepRunningInBackground: false,
    setKeepRunningInBackground: vi.fn(),
  },
  mailState: {
    activeAccountId: "account-1" as string | null,
  },
  pendingOpsSummary: {
    total_active_count: 0,
    failed_count: 0,
    in_progress_count: 0,
    last_error: null as string | null,
  },
  syncMutateAsync: vi.fn(() => Promise.resolve()),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const labels: Record<string, string> = {
        "status.syncNow": "Sync now",
        "status.stopSync": "Stop sync",
      };
      return labels[key] ?? fallback ?? key;
    },
  }),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn((eventName: string, handler: (event: { payload: unknown }) => void) => {
    mocks.listeners.set(eventName, handler);
    return Promise.resolve(vi.fn());
  }),
}));

vi.mock("@tanstack/react-query", () => ({
  useQueryClient: () => ({
    invalidateQueries: mocks.invalidateQueries,
  }),
}));

vi.mock("../../src/stores/ui.store", () => ({
  useUIStore: (selector: (state: typeof mocks.uiState) => unknown) => selector(mocks.uiState),
}));

vi.mock("../../src/stores/mail.store", () => ({
  useMailStore: (selector: (state: typeof mocks.mailState) => unknown) => selector(mocks.mailState),
}));

vi.mock("../../src/hooks/mutations/useSyncMutation", () => ({
  useSyncMutation: () => ({
    mutateAsync: mocks.syncMutateAsync,
  }),
}));

vi.mock("../../src/hooks/queries", () => ({
  pendingMailOpsSummaryQueryKey: (accountId: string | null) => ["pendingMailOps", accountId],
  usePendingMailOpsSummary: () => ({
    data: mocks.pendingOpsSummary,
  }),
}));

vi.mock("../../src/lib/api", () => ({
  stopSync: vi.fn(),
}));

import StatusBar from "../../src/components/StatusBar";

describe("StatusBar realtime mail events", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.listeners.clear();
    mocks.uiState.syncStatus = "idle";
    mocks.mailState.activeAccountId = "account-1";
  });

  it("invalidates message, thread, and account folder queries for new mail", async () => {
    render(<StatusBar />);

    await waitFor(() => expect(mocks.listeners.has("mail:new")).toBe(true));

    mocks.listeners.get("mail:new")?.({
      payload: {
        account_id: "account-1",
        message_id: "message-1",
        folder_ids: ["folder-inbox"],
        thread_id: "thread-1",
        subject: "Hello",
        from: "sender@example.com",
        received_at: 1_700_000_000,
      },
    });

    expect(mocks.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["messages"] });
    expect(mocks.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["threads"] });
    expect(mocks.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["folders", "account-1"] });
    expect(mocks.invalidateQueries).toHaveBeenCalledWith({ queryKey: ["folder-unread-counts", "account-1"] });
    expect(mocks.invalidateQueries).not.toHaveBeenCalledWith({ queryKey: ["folders"] });
  });

  it("shows manual mode when backend reports background sync is stopped", () => {
    mocks.uiState.realtimeStatusByAccount = {
      "account-1": {
        account_id: "account-1",
        mode: "manual",
        provider: "imap",
        last_success_at: null,
        next_retry_at: null,
        message: null,
      },
    };

    render(<StatusBar />);

    expect(document.body.textContent).toContain("Manual only");
  });

  it("returns the status to idle after a manual sync request is accepted", async () => {
    render(<StatusBar />);

    fireEvent.click(screen.getByLabelText("Sync now"));

    expect(mocks.uiState.setSyncStatus).toHaveBeenCalledWith("syncing");
    await waitFor(() => expect(mocks.syncMutateAsync).toHaveBeenCalledWith("account-1"));
    expect(mocks.uiState.setSyncStatus).toHaveBeenCalledWith("idle");
  });
});
