import { create } from "zustand";
import type { KanbanCard, KanbanColumnType } from "@/lib/api";
import { listKanbanCards, moveToKanban, removeFromKanban } from "@/lib/api";

interface KanbanState {
  cards: KanbanCard[];
  cardIdSet: Set<string>;
  contextNotes: Record<string, string>;
  loading: boolean;
  fetchCards: () => Promise<void>;
  moveCard: (messageId: string, column: KanbanColumnType, position: number) => Promise<void>;
  addCard: (messageId: string, column: KanbanColumnType) => Promise<void>;
  removeCard: (messageId: string) => Promise<void>;
  reorderInColumn: (column: KanbanColumnType, orderedIds: string[]) => void;
  setContextNote: (messageId: string, note: string) => void;
}

const CONTEXT_NOTES_STORAGE_KEY = "pebble-kanban-context-notes";

function loadContextNotes(): Record<string, string> {
  try {
    const parsed = JSON.parse(localStorage.getItem(CONTEXT_NOTES_STORAGE_KEY) || "{}");
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
}

function saveContextNotes(notes: Record<string, string>) {
  localStorage.setItem(CONTEXT_NOTES_STORAGE_KEY, JSON.stringify(notes));
}

function buildIdSet(cards: KanbanCard[]): Set<string> {
  return new Set(cards.map((c) => c.message_id));
}

export const useKanbanStore = create<KanbanState>((set, get) => ({
  cards: [],
  cardIdSet: new Set<string>(),
  contextNotes: loadContextNotes(),
  loading: false,

  fetchCards: async () => {
    set({ loading: true });
    try {
      const cards = await listKanbanCards();
      set({ cards, cardIdSet: buildIdSet(cards) });
    } finally {
      set({ loading: false });
    }
  },

  moveCard: async (messageId: string, column: KanbanColumnType, position: number) => {
    // Optimistic update
    const prev = get().cards;
    const updated = prev.map((c) =>
      c.message_id === messageId ? { ...c, column, position } : c,
    );
    set({ cards: updated, cardIdSet: buildIdSet(updated) });
    try {
      await moveToKanban(messageId, column, position);
    } catch {
      // Rollback on error
      set({ cards: prev, cardIdSet: buildIdSet(prev) });
    }
  },

  addCard: async (messageId: string, column: KanbanColumnType) => {
    await moveToKanban(messageId, column);
    await get().fetchCards();
  },

  reorderInColumn: (column, orderedIds) => {
    const prev = get().cards;
    const others = prev.filter((c) => c.column !== column);
    const reordered = orderedIds
      .map((id, i) => {
        const card = prev.find((c) => c.message_id === id);
        return card ? { ...card, position: i } : null;
      })
      .filter((c): c is KanbanCard => c !== null);
    const merged = [...others, ...reordered];
    set({ cards: merged, cardIdSet: buildIdSet(merged) });
    // Persist all position changes and rollback entirely on any failure
    Promise.all(
      reordered.map((card) => moveToKanban(card.message_id, card.column, card.position)),
    ).catch(() => {
      set({ cards: prev, cardIdSet: buildIdSet(prev) });
    });
  },

  removeCard: async (messageId: string) => {
    const prev = get().cards;
    const filtered = prev.filter((c) => c.message_id !== messageId);
    set({ cards: filtered, cardIdSet: buildIdSet(filtered) });
    try {
      await removeFromKanban(messageId);
    } catch {
      set({ cards: prev, cardIdSet: buildIdSet(prev) });
    }
  },

  setContextNote: (messageId, note) => {
    const next = { ...get().contextNotes, [messageId]: note };
    saveContextNotes(next);
    set({ contextNotes: next });
  },
}));
