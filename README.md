<p align="center">
  <img src="app-icon.png" width="96" alt="QuickFill logo" />
</p>

<h1 align="center">QuickFill</h1>

<p align="center">Paste your saved links and snippets into any text field on Windows: with a hotkey or a short code.</p>

---

## Why

Applying to jobs means typing the same GitHub, LinkedIn and X links into form after form. Each time: open the browser, find the profile, copy the URL, come back, paste. QuickFill removes all of that.

## Features

- **Hotkey popup.** Press your shortcut in any app and a small menu opens at the mouse pointer. Pick an item (arrow keys + Enter, type to search, or click) and it's pasted straight into the field you were typing in.
- **Short codes.** Type `:gh`, `:li` or `:x` anywhere and it's replaced with the full value, like a text expander.
- **Clipboard-safe.** Whatever you had copied is restored after each paste.
- **Lives in the tray.** It can start with Windows, and short codes can be paused from the tray menu.
- **Private.** Data stays on your PC, encrypted with Windows DPAPI (tied to your Windows account).
- **Tiny.** About a 2 MB installer, built with Tauri.

## Install

Download `QuickFill_x.y.z_x64-setup.exe` from [Releases](../../releases) and run it.
The installer isn't code-signed yet, so Windows SmartScreen may warn you: click **More info → Run anyway**.

## Usage

1. Open QuickFill from the tray and add items: a label (`GitHub`), a value (your URL) and an optional short code (`:gh`). Click **Save**.
2. In any text field, press the popup shortcut (default `Ctrl+Alt+Space`; change it in Settings), or type a short code.

Short-code rules: 2–16 characters, no spaces, unique, and no code may be the start of another (e.g. `:g` and `:gh` can't both exist).

## Known limitations

- Pasting and short codes don't work inside apps running as Administrator (Windows blocks simulated input there).
- Only text clipboard contents are restored; an image on the clipboard is lost after a paste.
- Short codes use a keyboard hook, which some antivirus tools may flag in unsigned apps. Typed characters are kept in memory only (the last 32) and are never logged or saved.

## Build from source

Requirements: Windows 10/11, [Node.js](https://nodejs.org) + [pnpm](https://pnpm.io), [Rust](https://rustup.rs) (MSVC toolchain) and Visual Studio Build Tools with **Desktop development with C++**.

```bash
pnpm install
pnpm tauri dev     # run in development
pnpm tauri build   # installer in src-tauri/target/release/bundle/nsis/
```

Run these from PowerShell or a Developer prompt, so the MSVC linker can be found.

## Tech

Tauri v2 (Rust) + React + TypeScript. The Win32 APIs (via the `windows` crate) handle the popup placement, focus restore, `SendInput` paste, the low-level keyboard hook and DPAPI encryption.

| Path | Purpose |
| --- | --- |
| `src/manager/` | Main window: items and settings |
| `src/picker/` | Hotkey popup |
| `src-tauri/src/picker.rs` | Places the popup at the mouse pointer |
| `src-tauri/src/paste.rs` | Clipboard swap, focus restore, Ctrl+V |
| `src-tauri/src/expander.rs` | Short codes (keyboard hook) |
| `src-tauri/src/store.rs`, `crypto.rs` | Encrypted storage and validation |

## License

MIT
