import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
const mockInvoke = vi.mocked(invoke);

// Import after mocking
import { accountsQueryKey } from "../../src/hooks/queries/useAccountsQuery";
import {
  getGlobalProxy,
  getOAuthAccountProxy,
  listAccounts,
  updateGlobalProxy,
  updateOAuthAccountProxy,
  updateAccount,
} from "../../src/lib/api";

describe("useAccountsQuery", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should have correct query key", () => {
    expect(accountsQueryKey).toEqual(["accounts"]);
  });

  it("listAccounts should call the correct Tauri command", async () => {
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

    const result = await listAccounts();

    expect(result).toEqual(mockAccounts);
    expect(mockInvoke).toHaveBeenCalledWith("list_accounts");
  });

  it("getOAuthAccountProxy should call the correct Tauri command", async () => {
    mockInvoke.mockResolvedValueOnce({ host: "127.0.0.1", port: 7890 });

    const result = await getOAuthAccountProxy("account-1");

    expect(result).toEqual({ host: "127.0.0.1", port: 7890 });
    expect(mockInvoke).toHaveBeenCalledWith("get_oauth_account_proxy", {
      accountId: "account-1",
    });
  });

  it("updateOAuthAccountProxy should call the correct Tauri command", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await updateOAuthAccountProxy("account-1", "127.0.0.1", 7890);

    expect(mockInvoke).toHaveBeenCalledWith("update_oauth_account_proxy", {
      accountId: "account-1",
      proxyHost: "127.0.0.1",
      proxyPort: 7890,
    });
  });

  it("getGlobalProxy should call the correct Tauri command", async () => {
    mockInvoke.mockResolvedValueOnce({ host: "127.0.0.1", port: 7890 });

    const result = await getGlobalProxy();

    expect(result).toEqual({ host: "127.0.0.1", port: 7890 });
    expect(mockInvoke).toHaveBeenCalledWith("get_global_proxy");
  });

  it("updateGlobalProxy should call the correct Tauri command", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await updateGlobalProxy("127.0.0.1", 7890);

    expect(mockInvoke).toHaveBeenCalledWith("update_global_proxy", {
      proxyHost: "127.0.0.1",
      proxyPort: 7890,
    });
  });

  it("updateAccount should include accountColor when saving an account color", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await updateAccount(
      "account-1",
      "user@example.com",
      "User",
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      undefined,
      "#22c55e",
    );

    expect(mockInvoke).toHaveBeenCalledWith("update_account", {
      accountId: "account-1",
      email: "user@example.com",
      displayName: "User",
      password: undefined,
      imapHost: undefined,
      imapPort: undefined,
      smtpHost: undefined,
      smtpPort: undefined,
      imapSecurity: undefined,
      smtpSecurity: undefined,
      proxyHost: undefined,
      proxyPort: undefined,
      accountColor: "#22c55e",
    });
  });
});
