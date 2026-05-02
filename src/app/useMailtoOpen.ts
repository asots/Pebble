import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { parseMailtoUrl } from "@/features/compose/mailto";
import { useComposeStore } from "@/stores/compose.store";

const OPEN_MAILTO_EVENT = "app:open-mailto";

interface OpenMailtoPayload {
  urls: string[];
}

export function openMailtoUrl(url: string) {
  const prefill = parseMailtoUrl(url);
  if (!prefill) return false;
  useComposeStore.getState().openCompose("new", null, prefill);
  return true;
}

function openMailtoUrls(urls: string[]) {
  for (const url of urls) {
    openMailtoUrl(url);
  }
}

export function useMailtoOpen() {
  useEffect(() => {
    invoke<string[]>("take_pending_mailto_urls")
      .then(openMailtoUrls)
      .catch((err) => console.warn("Failed to read pending mailto URLs:", err));

    const unlisten = listen<OpenMailtoPayload>(OPEN_MAILTO_EVENT, (event) => {
      openMailtoUrls(event.payload.urls);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
