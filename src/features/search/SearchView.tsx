import { useState, useCallback, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Search, SlidersHorizontal, Loader } from "lucide-react";
import type { AdvancedSearchQuery, SearchHit, Message } from "@/lib/api";
import { advancedSearch, searchMessages, getMessage } from "@/lib/api";
import { useUIStore } from "@/stores/ui.store";
import SearchFilters from "./SearchFilters";
import SearchResultItem from "./SearchResultItem";
import MessageDetail from "@/components/MessageDetail";

const emptyFilters: AdvancedSearchQuery = {};

function hasActiveFilters(filters: AdvancedSearchQuery): boolean {
  return !!(
    filters.from ||
    filters.to ||
    filters.subject ||
    filters.dateFrom ||
    filters.dateTo ||
    filters.hasAttachment ||
    filters.folderId
  );
}

export default function SearchView() {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [filters, setFilters] = useState<AdvancedSearchQuery>(emptyFilters);
  const [showFilters, setShowFilters] = useState(false);
  const [results, setResults] = useState<SearchHit[]>([]);
  const [messages, setMessages] = useState<Record<string, Message | null>>({});
  const [loading, setLoading] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);
  const [autoSearchDone, setAutoSearchDone] = useState(false);

  const doSearch = useCallback(async () => {
    const trimmed = query.trim();
    const filtersActive = hasActiveFilters(filters);

    if (!trimmed && !filtersActive) {
      setResults([]);
      setHasSearched(false);
      return;
    }

    setLoading(true);
    setHasSearched(true);
    try {
      let hits: SearchHit[];
      if (filtersActive) {
        hits = await advancedSearch({ ...filters, text: trimmed || undefined });
      } else {
        hits = await searchMessages(trimmed);
      }
      setResults(hits);
      setSelectedId(null);
    } finally {
      setLoading(false);
    }
  }, [query, filters]);

  // Pick up query from store when navigating from inbox search bar
  useEffect(() => {
    const storeQuery = useUIStore.getState().searchQuery;
    if (storeQuery) {
      setQuery(storeQuery);
      useUIStore.getState().setSearchQuery(""); // clear from store
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-search when query is populated from store
  useEffect(() => {
    if (query && !autoSearchDone && !hasSearched) {
      setAutoSearchDone(true);
      doSearch();
    }
  }, [query, autoSearchDone, hasSearched, doSearch]);

  // Fetch message details for results
  useEffect(() => {
    const idsToFetch = results
      .map((h) => h.message_id)
      .filter((id) => !(id in messages));

    if (idsToFetch.length === 0) return;

    let cancelled = false;
    (async () => {
      const fetched: Record<string, Message | null> = {};
      for (const id of idsToFetch) {
        if (cancelled) return;
        try {
          fetched[id] = await getMessage(id);
        } catch {
          fetched[id] = null;
        }
      }
      if (!cancelled) {
        setMessages((prev) => ({ ...prev, ...fetched }));
      }
    })();
    return () => { cancelled = true; };
  }, [results]); // eslint-disable-line react-hooks/exhaustive-deps

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    doSearch();
  }

  function handleClearFilters() {
    setFilters(emptyFilters);
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      {/* Search header */}
      <form
        onSubmit={handleSubmit}
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          padding: "10px 14px",
          borderBottom: "1px solid var(--color-border)",
          backgroundColor: "var(--color-bg)",
        }}
      >
        <Search size={16} color="var(--color-text-secondary)" style={{ flexShrink: 0 }} />
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("inbox.searchPlaceholder")}
          autoFocus
          style={{
            flex: 1,
            border: "none",
            outline: "none",
            backgroundColor: "transparent",
            fontSize: "14px",
            color: "var(--color-text-primary)",
          }}
        />
        <button
          type="button"
          onClick={() => setShowFilters(!showFilters)}
          title={t("search.filters")}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "4px",
            color: showFilters || hasActiveFilters(filters)
              ? "var(--color-accent)"
              : "var(--color-text-secondary)",
            display: "flex",
            alignItems: "center",
          }}
        >
          <SlidersHorizontal size={16} />
        </button>
      </form>

      {/* Filters panel */}
      {showFilters && (
        <SearchFilters
          filters={filters}
          onChange={setFilters}
          onClear={handleClearFilters}
        />
      )}

      {/* Results + Detail split layout */}
      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        {/* Results list */}
        <div
          style={{
            width: selectedId ? "360px" : "100%",
            flexShrink: 0,
            overflow: "auto",
            borderRight: selectedId ? "1px solid var(--color-border)" : "none",
            transition: "width 0.15s ease",
          }}
        >
          {loading && (
            <div
              className="fade-in"
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                padding: "40px",
                color: "var(--color-text-secondary)",
                fontSize: "13px",
                gap: "10px",
              }}
            >
              <Loader size={20} className="spinner" />
              <span>{t("common.loading")}</span>
            </div>
          )}

          {!loading && hasSearched && results.length === 0 && (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                padding: "40px",
                color: "var(--color-text-secondary)",
                fontSize: "14px",
              }}
            >
              {t("search.noResults")}
            </div>
          )}

          {!loading && !hasSearched && (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                padding: "60px 20px",
                color: "var(--color-text-tertiary)",
                fontSize: "14px",
                gap: "8px",
              }}
            >
              <Search size={32} />
              <span>{t("search.title")}</span>
            </div>
          )}

          {!loading &&
            results.map((hit) => (
              <SearchResultItem
                key={hit.message_id}
                hit={hit}
                message={messages[hit.message_id] ?? null}
                isSelected={hit.message_id === selectedId}
                onClick={() => setSelectedId(hit.message_id)}
              />
            ))}
        </div>

        {/* Detail panel */}
        {selectedId && (
          <div style={{ flex: 1, overflow: "hidden" }}>
            <MessageDetail
              messageId={selectedId}
              onBack={() => setSelectedId(null)}
            />
          </div>
        )}
      </div>
    </div>
  );
}
