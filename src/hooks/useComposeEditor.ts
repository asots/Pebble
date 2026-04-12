import { useState, useEffect, useRef, useMemo } from "react";
import { useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { Markdown as MarkdownExtension } from "tiptap-markdown";
import TurndownService from "turndown";
import { getSignature } from "@/lib/signatures";
import type { Message } from "@/lib/ipc-types";
import type { TFunction } from "i18next";

export type EditorMode = "rich" | "markdown" | "html";

const turndown = new TurndownService({ headingStyle: "atx", codeBlockStyle: "fenced" });

interface UseComposeEditorArgs {
  fromAccountId: string;
  composeMode: string | null;
  composeReplyTo: Message | null;
  isReply: boolean;
  t: TFunction;
  restoredDraft: {
    editorMode?: EditorMode;
    rawSource?: string;
  } | null;
}

export function useComposeEditor({
  fromAccountId,
  composeMode,
  composeReplyTo,
  isReply,
  t,
  restoredDraft,
}: UseComposeEditorArgs) {
  const [editorMode, setEditorMode] = useState<EditorMode>(restoredDraft?.editorMode ?? "rich");
  const [rawSource, setRawSource] = useState(restoredDraft?.rawSource ?? "");
  const [richTextHtml, setRichTextHtml] = useState("");
  const [htmlPreview, setHtmlPreview] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Build signature HTML block
  const signatureHtml = useMemo(() => {
    const sig = getSignature(fromAccountId);
    if (!sig) return "";
    const esc = (s: string) =>
      s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/\n/g, "<br/>");
    return `<br/><br/><div style="color:var(--color-text-secondary);font-size:13px">--<br/>${esc(sig)}</div>`;
  }, [fromAccountId]);

  const editorContent = useMemo(() => {
    try {
      const esc = (s: string) =>
        s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

      const extractBody = (html: string) => {
        try {
          const doc = new DOMParser().parseFromString(html, "text/html");
          return doc.body.innerHTML;
        } catch {
          return `<p>${esc(html)}</p>`;
        }
      };

      if (isReply && composeReplyTo) {
        const sender = esc(composeReplyTo.from_name || composeReplyTo.from_address || "");
        const dateStr = esc(new Date((composeReplyTo.date || 0) * 1000).toLocaleString());
        const body = composeReplyTo.body_html_raw
          ? extractBody(composeReplyTo.body_html_raw)
          : `<p>${esc(composeReplyTo.body_text || "")}</p>`;
        const attribution = t("compose.quoteAttribution", { date: dateStr, sender });
        return `${signatureHtml}<br/><br/><blockquote><p>${esc(attribution)}</p>${body}</blockquote>`;
      }
      if (composeMode === "forward" && composeReplyTo) {
        const sender = esc(composeReplyTo.from_name || composeReplyTo.from_address || "");
        const fwdSubject = esc(composeReplyTo.subject || "");
        const body = composeReplyTo.body_html_raw
          ? extractBody(composeReplyTo.body_html_raw)
          : `<p>${esc(composeReplyTo.body_text || "")}</p>`;
        return `${signatureHtml}<br/><br/><p>${esc(t("compose.forwardedHeader"))}</p><p>${esc(t("compose.forwardedFrom", { sender }))}</p><p>${esc(t("compose.forwardedSubject", { subject: fwdSubject }))}</p>${body}`;
      }
      return signatureHtml;
    } catch (err) {
      console.error("[ComposeView] Failed to build editor content:", err);
      return "";
    }
  }, [composeMode, composeReplyTo, isReply, t, signatureHtml]);

  const editor = useEditor({
    immediatelyRender: false,
    extensions: [
      StarterKit,
      Placeholder.configure({ placeholder: t("compose.editorPlaceholder", "Write your message...") }),
      MarkdownExtension.configure({ html: true, transformPastedText: true }),
    ],
    content: "",
  });

  // Set editor content after creation to avoid initialization crashes
  useEffect(() => {
    if (editor && editorContent) {
      editor.commands.setContent(editorContent);
    }
  }, [editor, editorContent]);

  useEffect(() => {
    if (!editor) {
      return;
    }

    const syncRichTextHtml = () => {
      setRichTextHtml(editor.getHTML());
    };

    syncRichTextHtml();
    editor.on("update", syncRichTextHtml);

    return () => {
      editor.off("update", syncRichTextHtml);
    };
  }, [editor]);

  // Switch between modes, syncing content
  function switchMode(newMode: EditorMode) {
    if (newMode === editorMode || !editor) return;

    if (editorMode === "rich") {
      if (newMode === "markdown") {
        setRawSource(turndown.turndown(editor.getHTML()));
      } else {
        setRawSource(editor.getHTML());
      }
    } else if (editorMode === "markdown") {
      if (newMode === "rich") {
        editor.commands.setContent(rawSource);
      } else {
        editor.commands.setContent(rawSource);
        setRawSource(editor.getHTML());
      }
    } else {
      if (newMode === "rich") {
        editor.commands.setContent(rawSource);
      } else {
        setRawSource(turndown.turndown(rawSource));
      }
    }

    setEditorMode(newMode);
  }

  return {
    editor,
    editorMode,
    rawSource,
    setRawSource,
    richTextHtml,
    htmlPreview,
    setHtmlPreview,
    switchMode,
    textareaRef,
  };
}
