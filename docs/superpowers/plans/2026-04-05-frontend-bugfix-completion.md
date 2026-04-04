# Frontend Bugfix & Feature Completion Plan (Round 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all broken features, wire stub code to real APIs, and add missing UI entry points so every implemented backend command has a functional frontend.

**Architecture:** All changes are frontend-only (React components, stores, hooks). No new backend Tauri commands. Archive/Delete are excluded because no `move_to_folder` command exists yet (IMAP provider stub only).

**Tech Stack:** React 19, TypeScript, Zustand, TanStack Query, lucide-react, Tauri v2 IPC

---

## File Map

| File | Action | Purpose |
|---|---|---|
| `src/stores/mail.store.ts` | Modify | Add `currentMessages` getter that syncs React Query data for keyboard shortcuts |
| `src/hooks/useKeyboard.ts` | Modify | Fix J/K/S shortcuts to use React Query data; add compose-new/reply/open-message actions |
| `src/features/inbox/InboxView.tsx` | Modify | Pass messages to mail store for keyboard access; remove unused useSearch |
| `src/features/search/SearchView.tsx` | Modify | Add message detail panel on result click |
| `src/features/inbox/SnoozePopover.tsx` | Modify | Wire onClose prop; add error feedback |
| `src/features/command-palette/commands.ts` | Modify | Fix notifications toggle logic; fix backup/restore commands |
| `src/features/settings/ShortcutsTab.tsx` | Modify | Add missing shortcut entries to ACTION_I18N_MAP and SHORTCUT_GROUPS |
| `src/components/StatusBar.tsx` | Modify | Add manual sync button |
| `src/components/Sidebar.tsx` | Modify | Add account switcher dropdown |
| `src/features/kanban/KanbanCard.tsx` | Modify | Add click-to-open-message navigation |
| `src/components/AttachmentList.tsx` | Modify | Show download feedback toast |
| `src/components/MessageDetail.tsx` | Modify | Remove broken archive/delete until backend supports it; add disabled state with tooltip |
| `src/components/MessageItem.tsx` | Modify | Remove broken archive button; keep star only |
| `src/locales/en.json` | Modify | Add missing i18n keys |
| `src/locales/zh.json` | Modify | Add missing i18n keys |

---

## Wave 1: Fix Broken Core Features

### Task 1: Fix keyboard shortcuts (J/K/S) to use React Query data

The root cause: `useKeyboard.ts` reads `useMailStore.getState().messages` which is the legacy Zustand cache (always empty when React Query is used). InboxView uses `useMessagesQuery` but never syncs data back to the store.

**Files:**
- Modify: `src/stores/mail.store.ts`
- Modify: `src/features/inbox/InboxView.tsx`
- Modify: `src/hooks/useKeyboard.ts`

- [ ] **Step 1: Add `setMessages` to mail store for syncing React Query data**

In `src/stores/mail.store.ts`, add to the `MailState` interface:

```tsx
setMessages: (messages: Message[]) => void;
```

Add to the store creation:

```tsx
setMessages: (messages) => set({ messages }),
```

- [ ] **Step 2: Sync React Query messages into the store from InboxView**

In `src/features/inbox/InboxView.tsx`, after the `useMessagesQuery` call:

```tsx
const { data: messages = [], isLoading: messagesLoading } = useMessagesQuery(
  threadView ? null : activeFolderId
);
const setMessages = useMailStore((s) => s.setMessages);

// Keep legacy store in sync for keyboard shortcuts
useEffect(() => {
  setMessages(messages);
}, [messages, setMessages]);
```

- [ ] **Step 3: Remove the unused `useSearch` import from InboxView**

Remove the line:
```tsx
const { search, clear } = useSearch();
```

And update the SearchBar:
```tsx
<SearchBar onSearch={() => {}} onClear={() => {}} />
```

Since SearchBar already navigates to SearchView on submit (from Task 5 earlier), the `onSearch` callback is no longer needed for data. We keep the prop interface but pass no-ops.

- [ ] **Step 4: Verify keyboard shortcuts work**

Run `pnpm dev`. Select a message. Press `J` to go to next, `K` to go to previous, `S` to toggle star. All should work now that the store has real data.

- [ ] **Step 5: Commit**

```bash
git add src/stores/mail.store.ts src/features/inbox/InboxView.tsx src/hooks/useKeyboard.ts
git commit -m "fix: sync React Query messages to store for keyboard shortcuts"
```

---

### Task 2: Fix SearchView — clicking a result opens message detail

**Files:**
- Modify: `src/features/search/SearchView.tsx`

- [ ] **Step 1: Add MessageDetail panel to SearchView**

Import MessageDetail:
```tsx
import MessageDetail from "@/components/MessageDetail";
```

Replace the current results rendering section. After the `<div style={{ flex: 1, overflow: "auto" }}>` results list, add a conditional detail panel:

```tsx
{/* Split layout: results list + detail panel */}
<div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
  {/* Results list */}
  <div style={{
    width: selectedId ? "360px" : "100%",
    flexShrink: 0,
    overflow: "auto",
    borderRight: selectedId ? "1px solid var(--color-border)" : "none",
    transition: "width 0.15s ease",
  }}>
    {/* ...existing loading/empty/results rendering... */}
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
```

This mirrors InboxView's split-panel pattern.

- [ ] **Step 2: Verify search result click opens detail**

Run `pnpm dev`. Navigate to Search, enter a query. Click a result — the detail panel should appear on the right showing the full message.

- [ ] **Step 3: Commit**

```bash
git add src/features/search/SearchView.tsx
git commit -m "fix: add message detail panel to search results"
```

---

### Task 3: Fix SnoozePopover — wire onClose and add error feedback

**Files:**
- Modify: `src/features/inbox/SnoozePopover.tsx`

- [ ] **Step 1: Use onClose instead of _onClose and add error state**

In `src/features/inbox/SnoozePopover.tsx`:

1. Change the destructuring from `onClose: _onClose` to just `onClose`:
```tsx
export default function SnoozePopover({ messageId, onClose, onSnoozed }: Props) {
```

2. Add error state:
```tsx
const [error, setError] = useState(false);
```

3. Update the catch branch to set error and call onClose after a delay:
```tsx
} catch (err) {
  console.error("Snooze failed:", err);
  setError(true);
  setTimeout(() => {
    setError(false);
    onClose();
  }, 2000);
}
```

4. Show error feedback in the UI — add after the preset buttons list:
```tsx
{error && (
  <div style={{
    padding: "6px 10px",
    fontSize: "12px",
    color: "#ef4444",
    textAlign: "center",
  }}>
    Snooze failed
  </div>
)}
```

- [ ] **Step 2: Commit**

```bash
git add src/features/inbox/SnoozePopover.tsx
git commit -m "fix: wire SnoozePopover onClose and add error feedback"
```

---

### Task 4: Fix command palette — notifications toggle + backup/restore

**Files:**
- Modify: `src/features/command-palette/commands.ts`

- [ ] **Step 1: Fix notifications toggle to match AppearanceTab's logic**

The current code reads `=== "true"` but AppearanceTab stores `"false"` for disabled (default is enabled when key is absent). Fix:

```tsx
{
  id: "settings:toggle-notifications",
  name: "Toggle Notifications",
  category: "Settings",
  execute: () => {
    const current = localStorage.getItem(NOTIFICATIONS_KEY) !== "false";
    localStorage.setItem(NOTIFICATIONS_KEY, String(!current));
  },
},
```

- [ ] **Step 2: Wire backup/restore commands to actual APIs**

Import the cloud sync APIs:
```tsx
import { updateMessageFlags, backupToWebdav, restoreFromWebdav } from "@/lib/api";
```

Replace the stub backup/restore commands:

```tsx
{
  id: "sync:backup",
  name: "Backup to Cloud",
  shortcut: "Ctrl+Shift+B",
  category: "Cloud Sync",
  execute: async () => {
    const url = localStorage.getItem("pebble-webdav-url") || "";
    const user = localStorage.getItem("pebble-webdav-user") || "";
    const pass = localStorage.getItem("pebble-webdav-pass") || "";
    if (!url) {
      useUIStore.getState().setActiveView("settings");
      return;
    }
    try {
      await backupToWebdav(url, user, pass);
    } catch (err) {
      console.error("Backup failed:", err);
    }
  },
},
{
  id: "sync:restore",
  name: "Restore from Cloud",
  category: "Cloud Sync",
  execute: async () => {
    const url = localStorage.getItem("pebble-webdav-url") || "";
    const user = localStorage.getItem("pebble-webdav-user") || "";
    const pass = localStorage.getItem("pebble-webdav-pass") || "";
    if (!url) {
      useUIStore.getState().setActiveView("settings");
      return;
    }
    try {
      await restoreFromWebdav(url, user, pass);
    } catch (err) {
      console.error("Restore failed:", err);
    }
  },
},
```

Note: Check `CloudSyncTab.tsx` to verify the localStorage keys used for WebDAV credentials. If the keys differ, use the same keys.

- [ ] **Step 3: Commit**

```bash
git add src/features/command-palette/commands.ts
git commit -m "fix: wire notifications toggle and backup/restore commands to real APIs"
```

---

### Task 5: Remove broken archive/delete buttons (no backend support)

There is no `move_to_folder` or `delete_message` Tauri command. The archive and delete buttons currently lie to users by appearing functional. Mark them as disabled with tooltips until backend support is added.

**Files:**
- Modify: `src/components/MessageDetail.tsx`
- Modify: `src/components/MessageItem.tsx`

- [ ] **Step 1: Disable archive/delete in MessageDetail toolbar**

In `src/components/MessageDetail.tsx`, find the archive and delete action objects in the toolbar array. Change them to disabled with a tooltip hint:

```tsx
{
  icon: Archive,
  label: t("messageActions.archive"),
  action: async () => {},
  disabled: true,
},
{
  icon: Trash2,
  label: t("messageActions.delete"),
  action: async () => {},
  disabled: true,
},
```

Update the `.map()` rendering to respect `disabled`:
```tsx
.map(({ icon: Icon, label, action, active, disabled }: ..., i) => (
  <button
    key={i}
    onClick={disabled ? undefined : action}
    title={disabled ? `${label} (coming soon)` : label}
    style={{
      ...existing styles...,
      opacity: disabled ? 0.35 : 1,
      cursor: disabled ? "default" : "pointer",
    }}
    onMouseEnter={disabled ? undefined : (e) => { ... }}
    onMouseLeave={disabled ? undefined : (e) => { ... }}
  >
    <Icon size={16} />
  </button>
))
```

- [ ] **Step 2: Remove archive button from MessageItem hover overlay**

In `src/components/MessageItem.tsx`, remove the Archive button from the hover overlay entirely (keep only the Star toggle). Delete the `<button>` block for Archive (lines ~150-166).

Also remove `Archive` from the lucide import if it becomes unused.

- [ ] **Step 3: Commit**

```bash
git add src/components/MessageDetail.tsx src/components/MessageItem.tsx
git commit -m "fix: disable archive/delete buttons (no backend move_to_folder command yet)"
```

---

### Task 6: Add missing shortcuts to ShortcutsTab

**Files:**
- Modify: `src/features/settings/ShortcutsTab.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/zh.json`

- [ ] **Step 1: Add i18n keys for new shortcuts**

In `en.json`, add to `"shortcuts"`:
```json
"composeNew": "Compose new message",
"reply": "Reply to message",
"openSearch": "Open search",
"backupToCloud": "Backup to cloud",
"toggleNotifications": "Toggle notifications"
```

In `zh.json`, add to `"shortcuts"`:
```json
"composeNew": "撰写新邮件",
"reply": "回复邮件",
"openSearch": "打开搜索",
"backupToCloud": "备份到云端",
"toggleNotifications": "切换通知"
```

- [ ] **Step 2: Update ACTION_I18N_MAP and SHORTCUT_GROUPS**

In `src/features/settings/ShortcutsTab.tsx`:

Add to `ACTION_I18N_MAP`:
```tsx
"compose-new":          "shortcuts.composeNew",
"reply":                "shortcuts.reply",
"open-search":          "shortcuts.openSearch",
"backup-to-cloud":      "shortcuts.backupToCloud",
"toggle-notifications": "shortcuts.toggleNotifications",
```

Update `SHORTCUT_GROUPS` to include the new actions:
```tsx
const SHORTCUT_GROUPS = [
  { categoryKey: "shortcuts.general", actions: ["command-palette", "close-modal"] },
  { categoryKey: "shortcuts.navigation", actions: ["next-message", "prev-message", "open-message", "open-search"] },
  { categoryKey: "shortcuts.mailActions", actions: ["compose-new", "reply", "toggle-star", "toggle-view-inbox", "toggle-view-kanban"] },
];
```

Note: `backup-to-cloud` and `toggle-notifications` are not in a category yet. Add a new group or append to general:
```tsx
{ categoryKey: "shortcuts.general", actions: ["command-palette", "close-modal", "backup-to-cloud", "toggle-notifications"] },
```

- [ ] **Step 3: Commit**

```bash
git add src/features/settings/ShortcutsTab.tsx src/locales/en.json src/locales/zh.json
git commit -m "feat: add all registered shortcuts to settings UI"
```

---

## Wave 2: Missing UI Essentials

### Task 7: Add account switcher to sidebar

**Files:**
- Modify: `src/components/Sidebar.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/zh.json`

- [ ] **Step 1: Add i18n key**

In `en.json`, add to `"sidebar"`:
```json
"allAccounts": "All Accounts"
```

In `zh.json`:
```json
"allAccounts": "所有账户"
```

- [ ] **Step 2: Add account selector between Mail label and folders**

In `src/components/Sidebar.tsx`, after the section label `{t("sidebar.mail", "Mail")}` div and before the folders `<nav>`, add an account dropdown when not collapsed and multiple accounts exist:

```tsx
{!sidebarCollapsed && accounts.length > 1 && (
  <div style={{ padding: "0 6px 4px" }}>
    <select
      value={activeAccountId || ""}
      onChange={(e) => {
        setActiveAccountId(e.target.value);
        setActiveFolderId(null); // reset folder on account switch
      }}
      style={{
        width: "100%",
        padding: "4px 6px",
        fontSize: "12px",
        borderRadius: "4px",
        border: "1px solid var(--color-border)",
        backgroundColor: "var(--color-bg)",
        color: "var(--color-text-primary)",
        cursor: "pointer",
        outline: "none",
      }}
    >
      {accounts.map((acc) => (
        <option key={acc.id} value={acc.id}>
          {acc.email}
        </option>
      ))}
    </select>
  </div>
)}
```

When collapsed, show a small icon hint with a title tooltip showing the active account email.

- [ ] **Step 3: Commit**

```bash
git add src/components/Sidebar.tsx src/locales/en.json src/locales/zh.json
git commit -m "feat: add account switcher dropdown to sidebar"
```

---

### Task 8: Add sync button to StatusBar

**Files:**
- Modify: `src/components/StatusBar.tsx`
- Modify: `src/stores/ui.store.ts` (if needed for sync status updates)

- [ ] **Step 1: Add manual sync trigger to StatusBar**

Import `startSync` and `stopSync` from api:
```tsx
import { startSync, stopSync } from "@/lib/api";
import { useMailStore } from "@/stores/mail.store";
import { RefreshCw } from "lucide-react";
```

Add a sync button next to the status text:
```tsx
const activeAccountId = useMailStore((s) => s.activeAccountId);

async function handleSync() {
  if (!activeAccountId) return;
  if (syncStatus === "syncing") {
    await stopSync(activeAccountId);
    setSyncStatus("idle");
  } else {
    setSyncStatus("syncing");
    try {
      await startSync(activeAccountId);
      setSyncStatus("idle");
    } catch {
      setSyncStatus("error");
    }
  }
}
```

Render as an icon button to the left of the status text:
```tsx
<button
  onClick={handleSync}
  disabled={!activeAccountId}
  title={syncStatus === "syncing" ? "Stop sync" : "Sync now"}
  style={{
    background: "none",
    border: "none",
    cursor: activeAccountId ? "pointer" : "default",
    padding: "2px",
    color: "var(--color-text-secondary)",
    display: "flex",
    alignItems: "center",
    opacity: activeAccountId ? 1 : 0.4,
  }}
>
  <RefreshCw
    size={13}
    style={{
      animation: syncStatus === "syncing" ? "spin 1s linear infinite" : "none",
    }}
  />
</button>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/StatusBar.tsx
git commit -m "feat: add manual sync button to status bar"
```

---

### Task 9: Add click-to-open on KanbanCard

**Files:**
- Modify: `src/features/kanban/KanbanCard.tsx`
- Modify: `src/features/kanban/KanbanView.tsx`

- [ ] **Step 1: Add onOpen callback to KanbanCard**

In `src/features/kanban/KanbanCard.tsx`, add `onOpen` to props:

```tsx
interface Props {
  id: string;
  message: Message | null;
  onRemove: (id: string) => void;
  onOpen: (messageId: string) => void;
}
```

Add a double-click handler on the card container:
```tsx
onDoubleClick={() => {
  if (message) onOpen(id);
}}
```

- [ ] **Step 2: Wire onOpen in KanbanView to navigate to message**

In `src/features/kanban/KanbanView.tsx`, import navigation:
```tsx
import { useUIStore } from "@/stores/ui.store";
import { useMailStore } from "@/stores/mail.store";
```

Add handler:
```tsx
function handleOpenMessage(messageId: string) {
  useMailStore.getState().setSelectedMessage(messageId);
  useUIStore.getState().setActiveView("inbox");
}
```

Pass to `KanbanCard`:
```tsx
<KanbanCard
  key={card.message_id}
  id={card.message_id}
  message={messageMap.get(card.message_id) ?? null}
  onRemove={handleRemove}
  onOpen={handleOpenMessage}
/>
```

- [ ] **Step 3: Commit**

```bash
git add src/features/kanban/KanbanCard.tsx src/features/kanban/KanbanView.tsx
git commit -m "feat: double-click kanban card to open message in inbox"
```

---

### Task 10: Show download feedback in AttachmentList

**Files:**
- Modify: `src/components/AttachmentList.tsx`

- [ ] **Step 1: Add downloaded state with path feedback**

Add state for downloaded paths:
```tsx
const [downloadedPaths, setDownloadedPaths] = useState<Record<string, string>>({});
```

Update `handleDownload` to store the path:
```tsx
async function handleDownload(attachment: Attachment) {
  setDownloadingId(attachment.id);
  try {
    await downloadAttachment(attachment.id, attachment.filename);
    const path = await getAttachmentPath(attachment.id);
    if (path) {
      setDownloadedPaths((prev) => ({ ...prev, [attachment.id]: path }));
    }
  } catch (err) {
    console.error("Failed to download attachment:", err);
  } finally {
    setDownloadingId(null);
  }
}
```

In the attachment row, after the download button, show a check icon + truncated path when downloaded:
```tsx
{downloadedPaths[attachment.id] && (
  <span style={{
    fontSize: "11px",
    color: "var(--color-accent)",
    maxWidth: "120px",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  }} title={downloadedPaths[attachment.id]}>
    ✓ Saved
  </span>
)}
```

Change the download button icon to a check when downloaded:
```tsx
{isDownloading ? <Loader size={14} /> : downloadedPaths[attachment.id] ? <Check size={14} /> : <Download size={14} />}
```

Import `Check` from lucide-react.

- [ ] **Step 2: Commit**

```bash
git add src/components/AttachmentList.tsx
git commit -m "feat: show download success feedback in attachment list"
```

---

## Wave 3: Completeness & Polish

### Task 11: Wire open-message (Enter) to show message detail

**Files:**
- Modify: `src/hooks/useKeyboard.ts`

- [ ] **Step 1: Make Enter key set selectedMessageId if not already set**

The current `open-message` case is a no-op. Change it to ensure the message detail actually opens:

```tsx
case "open-message": {
  const { selectedMessageId } = useMailStore.getState();
  if (selectedMessageId) {
    // Already selected — the detail panel is shown by InboxView automatically.
    // This is intentionally a confirmation action.
  } else {
    // Select the first message if none selected
    const { messages } = useMailStore.getState();
    if (messages.length > 0) {
      useMailStore.getState().setSelectedMessage(messages[0].id);
    }
  }
  break;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/hooks/useKeyboard.ts
git commit -m "fix: Enter key selects first message if none selected"
```

---

### Task 12: Wire remaining keyboard shortcut actions in useKeyboard

The shortcut store has `open-search`, `backup-to-cloud`, `toggle-notifications` registered but `useKeyboard.ts` has no case for them.

**Files:**
- Modify: `src/hooks/useKeyboard.ts`

- [ ] **Step 1: Add cases for remaining shortcuts**

```tsx
case "open-search":
  useUIStore.getState().setActiveView("search");
  break;

case "backup-to-cloud": {
  // Trigger backup command — reuse command palette logic
  const url = localStorage.getItem("pebble-webdav-url") || "";
  const user = localStorage.getItem("pebble-webdav-user") || "";
  const pass = localStorage.getItem("pebble-webdav-pass") || "";
  if (url) {
    import("@/lib/api").then(({ backupToWebdav }) => {
      backupToWebdav(url, user, pass).catch(console.error);
    });
  } else {
    useUIStore.getState().setActiveView("settings");
  }
  break;
}

case "toggle-notifications": {
  const key = "pebble-notifications-enabled";
  const current = localStorage.getItem(key) !== "false";
  localStorage.setItem(key, String(!current));
  break;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/hooks/useKeyboard.ts
git commit -m "feat: wire open-search, backup, and notifications keyboard shortcuts"
```

---

### Task 13: Final i18n audit and cleanup

**Files:**
- Modify: `src/locales/en.json`
- Modify: `src/locales/zh.json`

- [ ] **Step 1: Add any remaining missing keys**

Verify all hardcoded English strings across components have i18n keys. Specifically check:
- `MessageItem.tsx` — "Unstar" / "Star" / "Archive" tooltips
- `AttachmentList.tsx` — "✓ Saved" text
- `StatusBar.tsx` — "Sync now" / "Stop sync" tooltips
- `KanbanCard.tsx` — "Loading..." text
- `SnoozePopover.tsx` — "Snooze failed" text
- `SearchView.tsx` — any new strings from Task 2

Add keys to both `en.json` and `zh.json`:

```json
// en.json additions
"attachments": {
  "saved": "Saved"
},
"status": {
  "syncNow": "Sync now",
  "stopSync": "Stop sync"
},
"snooze": {
  "failed": "Snooze failed"
},
"kanban": {
  "loading": "Loading..."
}
```

```json
// zh.json additions
"attachments": {
  "saved": "已保存"
},
"status": {
  "syncNow": "立即同步",
  "stopSync": "停止同步"
},
"snooze": {
  "failed": "稍后提醒失败"
},
"kanban": {
  "loading": "加载中..."
}
```

- [ ] **Step 2: Commit**

```bash
git add src/locales/en.json src/locales/zh.json
git commit -m "feat(i18n): add remaining missing translation keys"
```

---

## Out of Scope (Requires Backend Work)

These items need new Tauri commands or significant backend changes and are **not** included in this plan:

| Feature | Why |
|---|---|
| Archive/Delete messages | No `move_to_folder` Tauri command (IMAP provider is a stub) |
| OAuth login (Gmail/Outlook) | Requires OAuth client ID configuration |
| Desktop notifications | No push event from backend; Tauri notification plugin not configured |
| Infinite scroll / pagination | Works with current `limit`/`offset` params but needs UX design for load-more trigger |
| Snoozed messages list view | New view + sidebar entry; feature-level scope |
| Thread message actions | Requires refactoring MessageDetail toolbar into reusable component |
| BCC field in compose | Requires backend `sendEmail` API change |
| Unread count badges on folders | `Folder` type does not include unread count |
| Contact directory view | New feature, not a bug fix |

---

## Verification Checklist

After all tasks are complete:

- [ ] `J`/`K` keys navigate between messages
- [ ] `S` key toggles star on selected message
- [ ] `C` key opens compose, `R` opens reply
- [ ] `Enter` selects first message or confirms selection
- [ ] Search result click opens message detail panel
- [ ] Snooze popover closes on error with feedback
- [ ] Command palette "Toggle Notifications" works correctly
- [ ] Command palette "Backup" calls WebDAV API (or navigates to settings if unconfigured)
- [ ] All keyboard shortcuts appear in Settings > Shortcuts
- [ ] Account switcher dropdown appears when multiple accounts exist
- [ ] Sync button in status bar triggers sync with spinning animation
- [ ] Double-click Kanban card opens message in inbox
- [ ] Attachment download shows "✓ Saved" feedback
- [ ] Archive/Delete buttons are visually disabled with "(coming soon)" tooltip
- [ ] All UI text has en/zh translations
