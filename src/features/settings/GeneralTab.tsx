import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useUIStore } from "@/stores/ui.store";

const NOTIFICATIONS_KEY = "pebble-notifications-enabled";
const POLL_OPTIONS = [5, 10, 15, 30, 60, 120, 300];

export default function GeneralTab() {
  const { t } = useTranslation();
  const pollInterval = useUIStore((s) => s.pollInterval);
  const setPollInterval = useUIStore((s) => s.setPollInterval);

  const [notificationsEnabled, setNotificationsEnabled] = useState(() => {
    return localStorage.getItem(NOTIFICATIONS_KEY) === "true";
  });

  const toggleNotifications = useCallback(() => {
    setNotificationsEnabled((prev) => {
      const next = !prev;
      localStorage.setItem(NOTIFICATIONS_KEY, String(next));
      invoke("set_notifications_enabled", { enabled: next }).catch((err) => console.warn("Failed to update notifications setting in backend", err));
      return next;
    });
  }, []);

  const showUnreadCount = useUIStore((s) => s.showFolderUnreadCount);
  const setShowUnreadCount = useUIStore((s) => s.setShowFolderUnreadCount);

  const toggleUnreadCount = useCallback(() => {
    setShowUnreadCount(!showUnreadCount);
  }, [showUnreadCount, setShowUnreadCount]);

  return (
    <div>
      <h3 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "8px" }}>
        {t("settings.syncInterval")}
      </h3>
      <p style={{ fontSize: "12px", color: "var(--color-text-secondary)", marginBottom: "12px", marginTop: 0 }}>
        {t("settings.syncIntervalDesc")}
      </p>
      <div style={{ display: "flex", gap: "8px", flexWrap: "wrap" }}>
        {POLL_OPTIONS.map((secs) => {
          const label = secs >= 60 ? `${secs / 60}m` : `${secs}s`;
          return (
            <button
              key={secs}
              onClick={() => setPollInterval(secs)}
              style={{
                padding: "8px 16px",
                borderRadius: "6px",
                border: pollInterval === secs ? "2px solid var(--color-accent)" : "1px solid var(--color-border)",
                backgroundColor: pollInterval === secs ? "var(--color-bg-hover)" : "transparent",
                cursor: "pointer",
                fontSize: "13px",
                fontWeight: pollInterval === secs ? 600 : 400,
                color: "var(--color-text-primary)",
              }}
            >
              {label}
            </button>
          );
        })}
      </div>

      <h3 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "16px", marginTop: "32px" }}>
        {t("settings.notifications")}
      </h3>
      <label
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          cursor: "pointer",
          fontSize: "13px",
          color: "var(--color-text-primary)",
        }}
      >
        <input type="checkbox" checked={notificationsEnabled} onChange={toggleNotifications} />
        <span>{t("settings.enableNotifications")}</span>
      </label>

      <h3 style={{ fontSize: "14px", fontWeight: 600, marginBottom: "16px", marginTop: "32px" }}>
        {t("settings.folderCounts", "Folder Counts")}
      </h3>
      <label
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          cursor: "pointer",
          fontSize: "13px",
          color: "var(--color-text-primary)",
        }}
      >
        <input
          type="checkbox"
          checked={showUnreadCount}
          onChange={toggleUnreadCount}
        />
        <span>{t("settings.showUnreadCount", "Show unread count badges in sidebar")}</span>
      </label>
    </div>
  );
}
