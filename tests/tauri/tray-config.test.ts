import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("Tauri tray config", () => {
  it("creates a native tray icon with restore, hide, and quit actions", () => {
    const libPath = resolve(process.cwd(), "src-tauri", "src", "lib.rs");
    const source = readFileSync(libPath, "utf8");

    expect(source).toContain("TrayIconBuilder::with_id(\"main\")");
    expect(source).toContain(".default_window_icon()");
    expect(source).toContain("TRAY_SHOW_ID");
    expect(source).toContain("TRAY_HIDE_ID");
    expect(source).toContain("TRAY_QUIT_ID");
    expect(source).toContain("restore_main_window");
    expect(source).toContain("hide_main_window");
  });

  it("allows the frontend to hide the window when closing to tray", () => {
    const capabilityPath = resolve(process.cwd(), "src-tauri", "capabilities", "default.json");
    const capability = JSON.parse(readFileSync(capabilityPath, "utf8"));

    expect(capability.permissions).toContain("core:window:allow-hide");
  });

  it("exposes a command for localized tray menu labels", () => {
    const libPath = resolve(process.cwd(), "src-tauri", "src", "lib.rs");
    const source = readFileSync(libPath, "utf8");

    expect(source).toContain("set_tray_menu_labels");
    expect(source).toContain("tray_by_id(\"main\")");
    expect(source).toContain(".set_menu(Some(menu))");
    expect(source).toContain("hide_label");
    expect(source).toContain("set_tray_menu_labels,");
  });
});
