import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { setTrayMenuLabels } from "@/lib/api";
import { useUIStore } from "@/stores/ui.store";

export function useTrayI18n() {
  const language = useUIStore((s) => s.language);
  const { t } = useTranslation();

  useEffect(() => {
    setTrayMenuLabels(
      t("tray.show", "Show Window"),
      t("tray.hide", "Hide Window"),
      t("tray.quit", "Quit Pebble"),
    ).catch((err) => console.warn("Failed to sync tray menu labels", err));
  }, [language, t]);
}
