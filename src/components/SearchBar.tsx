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
    <form onSubmit={handleSubmit} className="search-toolbar">
      <div className="search-input-shell">
        <Search size={15} aria-hidden="true" />
        <input
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={t("inbox.searchPlaceholder", "Search messages...")}
          aria-label={t("search.title", "Search")}
          className="search-input"
        />
      </div>
      {value && (
        <button
          type="button"
          onClick={handleClear}
          aria-label={t("search.clearFilters", "Clear filters")}
          className="search-toolbar-button search-toolbar-icon-button"
        >
          <X size={14} />
        </button>
      )}
      <button
        type="button"
        onClick={() => setActiveView("search")}
        title={t("search.advanced", "Advanced search")}
        aria-label={t("search.advanced", "Advanced search")}
        className="search-toolbar-button search-toolbar-icon-button"
      >
        <SlidersHorizontal size={14} />
      </button>
    </form>
  );
}
