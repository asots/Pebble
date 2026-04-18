import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

function readLocale(locale: string) {
  const localePath = resolve(process.cwd(), "src", "locales", `${locale}.json`);
  return JSON.parse(readFileSync(localePath, "utf8"));
}

describe("locale files", () => {
  it("translates folder count settings in English and Chinese", () => {
    const en = readLocale("en");
    const zh = readLocale("zh");

    expect(en.settings.folderCounts).toBe("Folder Counts");
    expect(en.settings.showUnreadCount).toBe("Show unread count badges in sidebar");
    expect(zh.settings.folderCounts).toBe("文件夹计数");
    expect(zh.settings.showUnreadCount).toBe("在侧边栏显示未读数徽章");
  });

  it("translates selected-text actions in English and Chinese", () => {
    const en = readLocale("en");
    const zh = readLocale("zh");

    expect(en.kanban.note).toBe("Kanban note");
    expect(en.kanban.contextNoteAdded).toBe("Added selected text to Kanban note");
    expect(en.kanban.contextNoteFailed).toBe("Failed to add Kanban note");
    expect(en.rules.contextRuleName).toBe("Selected text rule");
    expect(en.selection.actions).toBe("Selected text actions");
    expect(en.selection.copySelectedText).toBe("Copy selected text");
    expect(en.selection.copiedSelectedText).toBe("Copied selected text");
    expect(en.selection.moreActions).toBe("More selected-text actions");
    expect(en.selection.translate).toBe("Translate");
    expect(en.selection.translateSelectedText).toBe("Translate selected text");
    expect(en.selection.search).toBe("Search");
    expect(en.selection.searchSelectedText).toBe("Search selected text");
    expect(en.selection.createRule).toBe("Create rule");
    expect(en.selection.createRuleFromSelection).toBe("Create rule from selected text");
    expect(en.selection.addToKanbanNoteLabel).toBe("Add to Kanban note");
    expect(en.selection.addToKanbanNote).toBe("Add selected text as kanban note");
    expect(zh.kanban.note).toBe("看板备注");
    expect(zh.kanban.contextNoteAdded).toBe("已把选中文本加入看板备注");
    expect(zh.kanban.contextNoteFailed).toBe("加入看板备注失败");
    expect(zh.rules.contextRuleName).toBe("选中文本规则");
    expect(zh.selection.actions).toBe("选中文本操作");
    expect(zh.selection.copySelectedText).toBe("复制选中文本");
    expect(zh.selection.copiedSelectedText).toBe("已复制选中文本");
    expect(zh.selection.moreActions).toBe("更多选中文本操作");
    expect(zh.selection.translate).toBe("翻译");
    expect(zh.selection.translateSelectedText).toBe("翻译选中文本");
    expect(zh.selection.search).toBe("搜索");
    expect(zh.selection.searchSelectedText).toBe("搜索选中文本");
    expect(zh.selection.createRule).toBe("创建规则");
    expect(zh.selection.createRuleFromSelection).toBe("用选中文本创建规则");
    expect(zh.selection.addToKanbanNoteLabel).toBe("加入看板备注");
    expect(zh.selection.addToKanbanNote).toBe("加入看板备注");
  });
});
