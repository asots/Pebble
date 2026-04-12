import { useEffect, useRef, useCallback } from "react";
import { useUIStore } from "@/stores/ui.store";
import { hasComposeDraft } from "@/features/compose/compose-draft";

import type { EditorMode } from "./useComposeEditor";

const DRAFT_STORAGE_KEY = "pebble-compose-draft";

interface DraftData {
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  rawSource: string;
  richTextHtml: string;
  editorMode: EditorMode;
  savedAt: number;
}

function saveDraftToStorage(draft: Omit<DraftData, "savedAt">) {
  try {
    localStorage.setItem(DRAFT_STORAGE_KEY, JSON.stringify({ ...draft, savedAt: Date.now() }));
  } catch { /* quota exceeded — silently skip */ }
}

export function loadDraftFromStorage(): DraftData | null {
  try {
    const raw = localStorage.getItem(DRAFT_STORAGE_KEY);
    if (!raw) return null;
    const draft = JSON.parse(raw) as DraftData;
    // Discard drafts older than 24 hours
    if (Date.now() - draft.savedAt > 24 * 60 * 60 * 1000) {
      localStorage.removeItem(DRAFT_STORAGE_KEY);
      return null;
    }
    return draft;
  } catch { return null; }
}

export function clearDraftStorage() {
  localStorage.removeItem(DRAFT_STORAGE_KEY);
}

interface UseComposeDraftArgs {
  to: string[];
  cc: string[];
  bcc: string[];
  subject: string;
  rawSource: string;
  richTextHtml: string;
  editorMode: EditorMode;
  composeMode: string | null;
}

export function useComposeDraft({
  to, cc, bcc, subject, rawSource, richTextHtml, editorMode, composeMode,
}: UseComposeDraftArgs) {
  // Snapshot the initial compose state so pre-populated reply/forward
  // fields don't immediately trigger the "unsaved draft" guard.
  const initialSnapshot = useRef<{
    to: string[]; cc: string[]; bcc: string[]; subject: string;
  } | null>(null);
  if (!initialSnapshot.current) {
    initialSnapshot.current = { to: [...to], cc: [...cc], bcc: [...bcc], subject };
  }

  const arraysEqual = useCallback(
    (a: string[], b: string[]) => a.length === b.length && a.every((v, i) => v === b[i]),
    [],
  );

  // Track dirty state for leave-protection
  useEffect(() => {
    const init = initialSnapshot.current!;
    const userChanged =
      !arraysEqual(to, init.to) ||
      !arraysEqual(cc, init.cc) ||
      !arraysEqual(bcc, init.bcc) ||
      subject !== init.subject ||
      rawSource.trim().length > 0 ||
      hasComposeDraft({ to: [], cc: [], bcc: [], subject: "", rawSource, richTextHtml });
    useUIStore.getState().setComposeDirty(userChanged);
  }, [arraysEqual, bcc, cc, rawSource, richTextHtml, subject, to]);

  // Auto-save draft to localStorage (debounced 1s)
  useEffect(() => {
    if (!composeMode || composeMode !== "new") return;
    const timer = setTimeout(() => {
      const hasDraft = to.length > 0 || cc.length > 0 || bcc.length > 0 || subject.trim() || rawSource.trim() || richTextHtml.trim();
      if (hasDraft) {
        saveDraftToStorage({ to, cc, bcc, subject, rawSource, richTextHtml, editorMode });
      }
    }, 1000);
    return () => clearTimeout(timer);
  }, [to, cc, bcc, subject, rawSource, richTextHtml, editorMode, composeMode]);
}
