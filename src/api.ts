import { invoke } from "@tauri-apps/api/core";

/**
 * "text" is a one-line value; "template" is multi-line (emails, notes);
 * "totp" holds a 2FA secret and pastes the current one-time code;
 * "passwordTotp" holds a password (value) and a 2FA secret (secret) and
 * pastes the password followed by the code.
 */
export type ItemKind = "text" | "template" | "totp" | "passwordTotp";

/** Kinds that hold secrets: shown in their own cards, never searched by value. */
export const isSecretKind = (kind?: ItemKind) => kind === "totp" || kind === "passwordTotp";

export interface TotpPreview {
  code: string;
  /** Seconds until the code changes. */
  remaining: number;
  /** Seconds each code lasts (usually 30). */
  period: number;
}

export interface Item {
  id: string;
  label: string;
  value: string;
  code?: string | null;
  kind?: ItemKind;
  /** The 2FA secret of a "passwordTotp" item. */
  secret?: string | null;
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
  totpPreview: (secret: string) => invoke<TotpPreview>("totp_preview", { secret }),
};

/** Tauri rejects with the Rust error string; normalize anything else. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
