import { describe, it, expect, vi, beforeEach } from "vitest";
import { useMailStore } from "../../src/stores/mail.store";

// Mock Tauri invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockInvoke = vi.mocked(invoke);

const defaultState = {
  accounts: [],
  folders: [],
  messages: [],
  selectedMessageId: null,
  activeAccountId: null,
  activeFolderId: null,
  loadingMessages: false,
  loadingFolders: false,
};

describe("MailStore", () => {
  beforeEach(() => {
    useMailStore.setState(defaultState);
    vi.clearAllMocks();
  });

  it("should have correct initial state", () => {
    const state = useMailStore.getState();
    expect(state.accounts).toEqual([]);
    expect(state.folders).toEqual([]);
    expect(state.messages).toEqual([]);
    expect(state.selectedMessageId).toBeNull();
    expect(state.activeAccountId).toBeNull();
    expect(state.activeFolderId).toBeNull();
    expect(state.loadingMessages).toBe(false);
    expect(state.loadingFolders).toBe(false);
  });

  it("should fetch and set accounts", async () => {
    const mockAccounts = [
      {
        id: "a1",
        email: "test@example.com",
        display_name: "Test User",
        provider: "imap" as const,
        created_at: 1000,
        updated_at: 1000,
      },
    ];
    mockInvoke.mockResolvedValueOnce(mockAccounts);

    await useMailStore.getState().fetchAccounts();

    expect(useMailStore.getState().accounts).toEqual(mockAccounts);
    expect(mockInvoke).toHaveBeenCalledWith("list_accounts");
  });

  it("should set selected message", () => {
    useMailStore.getState().setSelectedMessage("msg-1");
    expect(useMailStore.getState().selectedMessageId).toBe("msg-1");

    useMailStore.getState().setSelectedMessage(null);
    expect(useMailStore.getState().selectedMessageId).toBeNull();
  });

  it("should fetch folders sorted by sort_order", async () => {
    const unsortedFolders = [
      {
        id: "f3",
        account_id: "a1",
        remote_id: "r3",
        name: "Trash",
        folder_type: "folder" as const,
        role: "trash" as const,
        parent_id: null,
        color: null,
        is_system: true,
        sort_order: 3,
      },
      {
        id: "f1",
        account_id: "a1",
        remote_id: "r1",
        name: "Inbox",
        folder_type: "folder" as const,
        role: "inbox" as const,
        parent_id: null,
        color: null,
        is_system: true,
        sort_order: 1,
      },
      {
        id: "f2",
        account_id: "a1",
        remote_id: "r2",
        name: "Sent",
        folder_type: "folder" as const,
        role: "sent" as const,
        parent_id: null,
        color: null,
        is_system: true,
        sort_order: 2,
      },
    ];
    mockInvoke.mockResolvedValueOnce(unsortedFolders);

    await useMailStore.getState().fetchFolders("a1");

    const folders = useMailStore.getState().folders;
    expect(folders[0].id).toBe("f1");
    expect(folders[1].id).toBe("f2");
    expect(folders[2].id).toBe("f3");
    expect(mockInvoke).toHaveBeenCalledWith("list_folders", {
      accountId: "a1",
    });
  });
});
