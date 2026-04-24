import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import AccountSetup from "../../src/components/AccountSetup";
import { accountsQueryKey } from "../../src/hooks/queries";
import {
  completeOAuthFlow,
  startSync,
} from "../../src/lib/api";

vi.mock("../../src/lib/i18n", () => ({
  default: {
    t: (_key: string, fallback?: string) => fallback ?? _key,
  },
}));

vi.mock("react-i18next", () => ({
  initReactI18next: {
    type: "3rdParty",
    init: vi.fn(),
  },
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

vi.mock("../../src/lib/api", () => ({
  addAccount: vi.fn(),
  completeOAuthFlow: vi.fn(),
  startSync: vi.fn(),
  testImapConnection: vi.fn(),
}));

describe("AccountSetup OAuth", () => {
  beforeEach(() => {
    vi.mocked(completeOAuthFlow).mockResolvedValue({
      id: "account-1",
      email: "user@example.com",
      display_name: "User",
      provider: "gmail",
      created_at: 1,
      updated_at: 1,
    });
    vi.mocked(startSync).mockResolvedValue("started");
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("refreshes account folders after OAuth sign-in starts sync", async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
      },
    });
    const invalidateSpy = vi.spyOn(queryClient, "invalidateQueries");
    const onClose = vi.fn();

    render(
      <QueryClientProvider client={queryClient}>
        <AccountSetup onClose={onClose} />
      </QueryClientProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Sign in with Google" }));

    await waitFor(() => {
      expect(completeOAuthFlow).toHaveBeenCalledWith("gmail", "", "");
    });
    await waitFor(() => {
      expect(startSync).toHaveBeenCalledWith("account-1", 3);
    });
    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: accountsQueryKey });
    });

    expect(invalidateSpy).toHaveBeenCalledWith({
      queryKey: ["folders", "account-1"],
    });

    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ["folders"] });
  });
});
