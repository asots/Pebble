import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { useTranslation } from "react-i18next";
import type { Message } from "@/lib/api";

interface Props {
  id: string;
  note?: string;
  message: Message | null;
  onRemove: (id: string) => void;
  onOpen: (messageId: string) => void;
}

export default function KanbanCard({ id, note, message, onRemove, onOpen }: Props) {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id });

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
    padding: "10px 12px",
    marginBottom: "6px",
    backgroundColor: "var(--color-bg)",
    border: "1px solid var(--color-border)",
    borderRadius: "8px",
    cursor: "grab",
    fontSize: "13px",
    textAlign: "left",
    width: "100%",
    font: "inherit",
    color: "inherit",
  };

  // Enter opens the message; Space is reserved for dnd-kit drag activation.
  function handleKeyDown(e: React.KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      onOpen(id);
    }
  }

  if (!message) {
    return (
      <div
        ref={setNodeRef}
        style={style}
        {...attributes}
        {...listeners}
        aria-label={t("common.loading")}
      >
        <span style={{ color: "var(--color-text-secondary)" }}>{t("common.loading")}</span>
      </div>
    );
  }

  const subject = message.subject || t("common.noSubject");
  const sender = message.from_name || message.from_address;

  return (
    <div
      ref={setNodeRef}
      style={style}
      {...attributes}
      {...listeners}
      onKeyDown={handleKeyDown}
      aria-label={t("kanban.cardAriaLabel", "{{subject}} from {{sender}}. Enter to open, Space to drag.", { subject, sender })}
    >
      <div
        onClick={() => onOpen(id)}
        style={{ cursor: "pointer" }}
        title={t("kanban.clickToOpen", "Click to open message")}
      >
        <div style={{ fontWeight: 600, color: "var(--color-text-primary)", marginBottom: "4px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {subject}
        </div>
      </div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ color: "var(--color-text-secondary)", fontSize: "12px" }}>
          {sender}
        </span>
        <span style={{ color: "var(--color-text-secondary)", fontSize: "11px", marginRight: "auto", marginLeft: "8px" }}>
          {new Date(message.date * 1000).toLocaleDateString()}
        </span>
        <button
          onClick={(e) => { e.stopPropagation(); onRemove(id); }}
          onPointerDown={(e) => e.stopPropagation()}
          onKeyDown={(e) => e.stopPropagation()}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            color: "var(--color-text-secondary)",
            fontSize: "14px",
            padding: "0 4px",
          }}
          aria-label={t("kanban.removeFromBoard")}
          title={t("kanban.removeFromBoard")}
        >
          ×
        </button>
      </div>
      {note && (
        <div
          style={{
            marginTop: "8px",
            padding: "6px 8px",
            borderRadius: "6px",
            backgroundColor: "var(--color-bg-secondary, rgba(0,0,0,0.04))",
            color: "var(--color-text-secondary)",
            fontSize: "12px",
            lineHeight: 1.4,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {note}
        </div>
      )}
    </div>
  );
}
