import { useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import iconUrl from "/icon.png?url";

const REPO = "QingJ01/Pebble";
const RELEASES_URL = `https://github.com/${REPO}/releases`;

function openUrl(url: string) {
  invoke("open_external_url", { url }).catch((err) => console.warn("Failed to open external URL", err));
}

interface UpdateState {
  status: "idle" | "checking" | "latest" | "available" | "error";
  latestVersion?: string;
  releaseUrl?: string;
  error?: string;
}

export default function AboutTab() {
  const { t } = useTranslation();
  const [appVersion, setAppVersion] = useState<string>("");
  const [update, setUpdate] = useState<UpdateState>({ status: "idle" });

  // Load version on first render
  if (!appVersion) {
    getVersion().then(setAppVersion).catch(() => setAppVersion("0.1.0"));
  }

  async function handleCheckUpdate() {
    setUpdate({ status: "checking" });
    try {
      const info = await invoke<{
        latest_version: string;
        release_url: string;
        is_newer: boolean;
      }>("check_for_update", { currentVersion: appVersion });
      if (info.is_newer) {
        setUpdate({
          status: "available",
          latestVersion: info.latest_version,
          releaseUrl: info.release_url,
        });
      } else {
        setUpdate({ status: "latest", latestVersion: info.latest_version });
      }
    } catch (err) {
      setUpdate({
        status: "error",
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  return (
    <div>
      <h2
        style={{
          fontSize: "18px",
          fontWeight: 600,
          color: "var(--color-text-primary)",
          marginTop: 0,
          marginBottom: "24px",
        }}
      >
        {t("about.title", "About")}
      </h2>

      {/* App info */}
      <div style={{ marginBottom: "24px" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "14px", marginBottom: "16px" }}>
          <img
            src={iconUrl}
            alt="Pebble"
            style={{ width: "56px", height: "56px", borderRadius: "12px" }}
          />
          <div>
            <div style={{ fontSize: "17px", fontWeight: 600, color: "var(--color-text-primary)" }}>
              Pebble
            </div>
            <div style={{ fontSize: "13px", color: "var(--color-text-secondary)", marginTop: "2px" }}>
              {t("about.version", "Version")} {appVersion || "..."}
            </div>
          </div>
        </div>

        <p style={{ fontSize: "13px", color: "var(--color-text-secondary)", lineHeight: 1.7, margin: "0 0 8px" }}>
          {t(
            "about.description",
            "A privacy-first desktop email client built with Rust and React. Mail, search index, and attachments stay on your device. No telemetry. Optional features like translation send only the selected text to the provider you configure.",
          )}
        </p>
        <p style={{ fontSize: "13px", color: "var(--color-text-secondary)", lineHeight: 1.7, margin: "0 0 8px" }}>
          {t(
            "about.features",
            "Supports Gmail, Outlook, and IMAP accounts. Includes Kanban board, full-text search, snooze, rules engine, built-in translation, and optional WebDAV settings backup.",
          )}
        </p>
        <p style={{ fontSize: "12px", color: "var(--color-text-tertiary, var(--color-text-secondary))", lineHeight: 1.5, margin: 0 }}>
          {t("about.license", "Open source under AGPL-3.0 license.")}
        </p>
      </div>

      {/* Check for updates */}
      <div style={{ marginBottom: "24px" }}>
        <h3
          style={{
            fontSize: "14px",
            fontWeight: 600,
            color: "var(--color-text-primary)",
            marginTop: 0,
            marginBottom: "12px",
          }}
        >
          {t("about.updates", "Updates")}
        </h3>

        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <button
            onClick={handleCheckUpdate}
            disabled={update.status === "checking"}
            style={{
              padding: "8px 18px",
              fontSize: "13px",
              fontWeight: 500,
              border: "1px solid var(--color-border)",
              borderRadius: "6px",
              backgroundColor: "var(--color-bg-hover)",
              color: "var(--color-text-primary)",
              cursor: update.status === "checking" ? "wait" : "pointer",
              opacity: update.status === "checking" ? 0.6 : 1,
            }}
          >
            {update.status === "checking"
              ? t("about.checking", "Checking...")
              : t("about.checkUpdate", "Check for updates")}
          </button>

          {update.status === "latest" && (
            <span style={{ fontSize: "13px", color: "#22c55e" }}>
              {t("about.upToDate", "You're on the latest version")}
            </span>
          )}
        </div>

        {update.status === "available" && (
          <div
            style={{
              marginTop: "12px",
              padding: "12px 14px",
              borderRadius: "6px",
              backgroundColor: "rgba(59, 130, 246, 0.08)",
              border: "1px solid rgba(59, 130, 246, 0.2)",
              fontSize: "13px",
              color: "var(--color-text-primary)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <span>
              {t("about.newVersion", "New version available: {{version}}", {
                version: update.latestVersion,
              })}
            </span>
            <button
              onClick={() => openUrl(update.releaseUrl || RELEASES_URL)}
              style={{
                padding: "6px 14px",
                fontSize: "12px",
                fontWeight: 500,
                border: "none",
                borderRadius: "6px",
                backgroundColor: "var(--color-accent, #3b82f6)",
                color: "#fff",
                cursor: "pointer",
              }}
            >
              {t("about.download", "Download")}
            </button>
          </div>
        )}

        {update.status === "error" && (
          <div
            style={{
              marginTop: "12px",
              padding: "10px 14px",
              borderRadius: "6px",
              backgroundColor: "rgba(220, 53, 69, 0.1)",
              border: "1px solid rgba(220, 53, 69, 0.3)",
              fontSize: "13px",
              color: "#dc3545",
            }}
          >
            {t("about.checkFailed", "Failed to check for updates")}: {update.error}
          </div>
        )}
      </div>

      {/* Links */}
      <div>
        <h3
          style={{
            fontSize: "14px",
            fontWeight: 600,
            color: "var(--color-text-primary)",
            marginTop: 0,
            marginBottom: "12px",
          }}
        >
          {t("about.links", "Links")}
        </h3>
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          {[
            { label: "GitHub Releases", url: RELEASES_URL },
            { label: t("about.sourceCode", "Source Code"), url: `https://github.com/${REPO}` },
            { label: t("about.reportIssue", "Report an Issue"), url: `https://github.com/${REPO}/issues` },
          ].map((link) => (
            <a
              key={link.url}
              href={link.url}
              onClick={(e) => { e.preventDefault(); openUrl(link.url); }}
              style={{
                fontSize: "13px",
                color: "var(--color-accent, #3b82f6)",
                textDecoration: "none",
                cursor: "pointer",
              }}
            >
              {link.label}
            </a>
          ))}
        </div>
      </div>
    </div>
  );
}
