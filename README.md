<p align="center">
  <img src="app-icon.png" width="96" alt="QuickFill logo" />
</p>

<h1 align="center">QuickFill</h1>

<p align="center">Paste your saved links and snippets into any text field on Windows: with a hotkey or a short code.</p>

<p align="center">
  <a href="https://github.com/AbhijitPrajapati12/quickfill/releases"><img src="https://img.shields.io/github/downloads/AbhijitPrajapati12/quickfill/total?label=downloads&color=4c1" alt="Downloads" /></a>
  <a href="https://github.com/AbhijitPrajapati12/quickfill/releases/latest"><img src="https://img.shields.io/github/v/release/AbhijitPrajapati12/quickfill?label=release&color=0a7bbb" alt="Latest release" /></a>
  <a href="https://github.com/AbhijitPrajapati12/quickfill/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/AbhijitPrajapati12/quickfill/release.yml?label=build&logo=github" alt="Release build" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/AbhijitPrajapati12/quickfill?label=license&color=0a7bbb" alt="License: Apache 2.0" /></a>
  <a href="https://v2.tauri.app"><img src="https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white" alt="Tauri 2" /></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6" alt="Windows 10 and 11" />
  <a href="https://quickfill-app.vercel.app"><img src="https://img.shields.io/badge/website-quickfill--app.vercel.app-7c3aed" alt="Website" /></a>
</p>

---

## Why

Applying to jobs means typing the same GitHub, LinkedIn and X links into form after form. Each time: open the browser, find the profile, copy the URL, come back, paste. QuickFill removes all of that.

## Features

- **Hotkey popup.** Press your shortcut in any app and a small menu opens at the mouse pointer. Pick an item (arrow keys + Enter, type to search, or click) and it's pasted straight into the field you were typing in.
- **Short codes.** Type `:gh`, `:li` or `:x` anywhere and it's replaced with the full value, like a text expander.
- **2FA codes.** Add a site's two-factor secret key (or import its QR code from a screenshot) and QuickFill pastes the current 6-digit code, like an authenticator app.
- **Password + 2FA.** For logins such as VPNs that want your password and code typed together, one shortcut pastes the password followed by the current code (`hunter2` + `481717` → `hunter2481717`).
- **Clipboard-safe.** Whatever you had copied is restored after each paste.
- **Lives in the tray.** It can start with Windows, and short codes can be paused from the tray menu.
- **Private.** Data stays on your PC, encrypted with Windows DPAPI (tied to your Windows account).
- **Tiny.** About a 2 MB installer, built with Tauri.

## Install

Download `QuickFill_x.y.z_x64-setup.exe` from [Releases](../../releases) and run it.
The installer isn't code-signed yet, so Windows SmartScreen may warn you: click **More info → Run anyway**.

## Usage

1. Open QuickFill from the tray and add items: a label (`GitHub`), a value (your URL) and an optional short code (`:gh`). Click **Save**.
2. For 2FA: when a site shows its two-factor QR code, open the **2FA codes** card and click **+ Add 2FA**. Then either paste the secret key (the "can't scan it?" text), or snip the QR with `Win+Shift+S` and press `Ctrl+V` in the row (or click **Import QR image** to pick a file). Check that the code shown matches what the site expects, then click **Save**.
3. For a password plus a 2FA code in one field: in the **Password + 2FA** card, click **+ Add password + 2FA** and fill in the password and the 2FA secret the same way. Both are required.
4. In any text field, press the popup shortcut (default `Ctrl+Alt+Space`; change it in Settings), or type a short code.

Short-code rules: every code starts with `:` (the field adds it for you, so you type just `gh`). After the `:` come 1–15 characters with no spaces. Codes must be unique, and no code may be the start of another (e.g. `:g` and `:gh` can't both exist). The `:` keeps codes from firing inside ordinary words, so `:gh` never expands while you type "high" or "though". Codes saved without a `:` in older versions get one added automatically.

## Known limitations

- Pasting and short codes don't work inside apps running as Administrator (Windows blocks simulated input there).
- Only text clipboard contents are restored; an image on the clipboard is lost after a paste.
- Keeping a 2FA secret on the same PC as your passwords makes it less of a *second* factor: anyone who can use your Windows account can get both. Use it for convenience on accounts where that trade-off is acceptable.
- Only time-based (TOTP) codes are supported. Google Authenticator's bulk-export QR codes can't be imported.
- Short codes use a keyboard hook, which some antivirus tools may flag in unsigned apps. Typed characters are kept in memory only (the last 32) and are never logged or saved.

## Build from source

Requirements: Windows 10/11, [Node.js](https://nodejs.org) + [pnpm](https://pnpm.io), [Rust](https://rustup.rs) (MSVC toolchain) and Visual Studio Build Tools with **Desktop development with C++**.

```bash
pnpm install
pnpm tauri dev     # run in development
pnpm tauri build   # installer in src-tauri/target/release/bundle/nsis/
```

Run these from PowerShell or a Developer prompt, so the MSVC linker can be found.

## Releasing

```bash
pnpm release 0.2.0            # or: patch | minor | major
```

This bumps the version in `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `Cargo.lock`, commits `Release v0.2.0`, tags `v0.2.0` and pushes. The tag starts the [Release workflow](.github/workflows/release.yml), which builds the installer on GitHub and publishes it as a release (about 10 minutes). The website's Download button picks up the new version within an hour.

Add `--dry-run` to preview without changing anything, or `--no-push` to commit and tag locally only. The script refuses to run with uncommitted changes or a version that isn't newer than the current one.

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
| `src-tauri/src/totp.rs` | 2FA code generation (RFC 6238) |
| `src/manager/TwoFactor.tsx`, `qr.ts` | 2FA and Password + 2FA cards, QR import |

## License

[Apache License 2.0](LICENSE). See [NOTICE](NOTICE).

Releases up to and including v0.2.0 were published under the MIT License.
