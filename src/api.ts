import { invoke } from "@tauri-apps/api/core";

/** "text" is a one-line value; "template" is multi-line (emails, notes). */
export type ItemKind = "text" | "template";

export interface Item {
  id: string;
  label: string;
  value: string;
  code?: string | null;
  kind?: ItemKind;
}

export interface Settings {
  hotkey: string;
  autostart: boolean;
  codesEnabled: boolean;
}

export interface Snapshot {
  items: Item[];
  settings: Settings;
  hotkeyError: string | null;
}

export const api = {
  getData: () => invoke<Snapshot>("get_data"),
  saveItems: (items: Item[]) => invoke<Snapshot>("save_items", { items }),
  setHotkey: (accel: string) => invoke<Snapshot>("set_hotkey", { accel }),
  setAutostart: (enabled: boolean) => invoke<Snapshot>("set_autostart", { enabled }),
  setCodesEnabled: (enabled: boolean) => invoke<Snapshot>("set_codes_enabled", { enabled }),
  pasteItem: (id: string) => invoke<void>("paste_item", { id }),
  hidePicker: () => invoke<void>("hide_picker"),
};

/** Tauri rejects with the Rust error string; normalize anything else. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
