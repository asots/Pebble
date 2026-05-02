import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import ConfirmDialog from "../../src/components/ConfirmDialog";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback?: string) => fallback ?? _key,
  }),
}));

describe("ConfirmDialog", () => {
  it("keeps focus inside the dialog and restores the previous focus on close", () => {
    const opener = document.createElement("button");
    document.body.appendChild(opener);
    opener.focus();

    const { unmount } = render(
      <ConfirmDialog
        title="Delete rule"
        message="This cannot be undone."
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    const dialog = screen.getByRole("dialog");
    const cancelButton = screen.getByRole("button", { name: "Cancel" });
    const confirmButton = screen.getByRole("button", { name: "Confirm" });

    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(document.activeElement).toBe(confirmButton);

    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(cancelButton);

    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(confirmButton);

    unmount();

    expect(document.activeElement).toBe(opener);
    opener.remove();
  });

  it("uses the themed dialog surface instead of a plain white panel", () => {
    render(
      <ConfirmDialog
        title="Discard draft?"
        message="You have unsaved changes."
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    );

    const panel = screen.getByRole("dialog").firstElementChild as HTMLElement;

    expect(panel.style.backgroundColor).toBe("var(--color-sidebar-bg)");
    expect(panel.style.color).toBe("var(--color-text-primary)");
    expect(panel.style.border).toBe("1px solid var(--color-border)");
  });
});
