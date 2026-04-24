import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import SearchFilters from "../../../src/features/search/SearchFilters";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const labels: Record<string, string> = {
        "search.filters": "Filters",
        "search.from": "From",
        "search.to": "To",
        "search.subject": "Subject",
        "search.dateFrom": "From date",
        "search.dateTo": "To date",
        "search.hasAttachment": "Has attachment",
        "search.folder": "Folder",
        "search.allFolders": "All folders",
        "search.clearFilters": "Clear filters",
      };
      return labels[key] ?? fallback ?? key;
    },
  }),
}));

vi.mock("@/lib/api", () => ({
  listAccounts: vi.fn().mockResolvedValue([{ id: "account-1", email: "me@example.com" }]),
  listFolders: vi.fn().mockResolvedValue([{ id: "folder-1", account_id: "account-1", name: "Inbox" }]),
}));

describe("SearchFilters", () => {
  it("renders advanced filters as a compact panel with consistent controls", async () => {
    render(<SearchFilters filters={{}} onChange={vi.fn()} onClear={vi.fn()} />);

    const panel = screen.getByRole("region", { name: "Filters" });
    expect(panel.className).toContain("search-filters-panel");
    expect(screen.getByLabelText("From").closest(".search-filter-field")).toBeTruthy();
    expect(screen.getByLabelText("Folder").className).toContain("search-filter-control");
    expect(screen.getByRole("button", { name: "Clear filters" }).className).toContain("search-filters-clear");
    expect(await screen.findByRole("option", { name: "me@example.com / Inbox" })).toBeTruthy();
  });
});
