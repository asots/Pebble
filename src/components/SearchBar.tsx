import { useState } from "react";
import { Search, SlidersHorizontal, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useUIStore } from "@/stores/ui.store";

interface Props {
  onSearch: (query: string) => void;
  onClear: () => void;
}

export default function SearchBar({ onSearch, onClear }: Props) {
  const { t } = useTranslation();
  const [value, setValue] = useState("");
  const setActiveView = useUIStore((s) => s.setActiveView);

  function handleSubmit(e: React.SyntheticEvent<HTMLFormElement>) {
    e.preventDefault();
    if (value.trim()) {
      onSearch(value.trim());
      useUIStore.getState().setSearchQuery(value.trim());
      useUIStore.getState().setActiveView("search");
    }
  }

  function handleClear() {
    setValue("");
    onClear();
  }

  return (
    <form
      onSubmit={handleSubmit}
      style={{
        display: "flex",
        alignItems: "center",
        gap: "8px",
        padding: "8px 12px",
        borderBottom: "1px solid var(--color-border)",
        backgroundColor: "var(--color-bg)",
      }}
    >
      <Search size={15} color="var(--color-text-secondary)" style={{ flexShrink: 0 }} />
      <input
        type="text"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={t("inbox.searchPlaceholder", "Search messages...")}
        aria-label={t("search.title", "Search")}
        style={{
          flex: 1,
          border: "none",
          backgroundColor: "transparent",
          fontSize: "13px",
          color: "var(--color-text-primary)",
        }}
      />
      {value && (
        <button
          type="button"
          onClick={handleClear}
          aria-label={t("search.clearFilters", "Clear filters")}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "2px",
            color: "var(--color-text-secondary)",
            display: "flex",
            alignItems: "center",
          }}
        >
          <X size={14} />
        </button>
      )}
      <button
        type="button"
        onClick={() => setActiveView("search")}
        title={t("search.advanced", "Advanced search")}
        aria-label={t("search.advanced", "Advanced search")}
        style={{
          background: "none",
          border: "none",
          cursor: "pointer",
          padding: "2px",
          color: "var(--color-text-secondary)",
          display: "flex",
          alignItems: "center",
        }}
      >
        <SlidersHorizontal size={14} />
      </button>
    </form>
  );
}
