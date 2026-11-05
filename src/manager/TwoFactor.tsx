import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorText, type Item, type Snapshot, type TotpPreview } from "../api";
import { decodeQrImage, labelFromLink, toTotpLink } from "./qr";

interface Props {
  snapshot: Snapshot;
  onUpdate: (s: Snapshot) => void;
}

const isTotp = (i: Item) => i.kind === "totp";

/** 2FA secrets: saved separately from the links/templates list above. */
export default function TwoFactor({ snapshot, onUpdate }: Props) {
  const [entries, setEntries] = useState<Item[]>(() => snapshot.items.filter(isTotp));
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [previews, setPreviews] = useState<Record<string, TotpPreview | string>>({});
  const [revealed, setRevealed] = useState<Set<string>>(new Set());
  // Entries whose secret-key panel is open. New entries open it automatically.
  const [openKeys, setOpenKeys] = useState<Set<string>>(new Set());
  const [copied, setCopied] = useState<string | null>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const importTarget = useRef<string | null>(null);

  const edit = (next: Item[]) => {
    setEntries(next);
    setDirty(true);
    setSaved(false);
  };

  const update = (id: string, patch: Partial<Item>) =>
    edit(entries.map((e) => (e.id === id ? { ...e, ...patch } : e)));

  const add = () => {
    const id = crypto.randomUUID();
    edit([...entries, { id, label: "", value: "", code: "", kind: "totp" }]);
    setOpenKeys((prev) => new Set(prev).add(id));
  };

  // Refresh the live codes every second so they can be checked against a phone app.
  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      const next: Record<string, TotpPreview | string> = {};
      await Promise.all(
        entries
          .filter((e) => e.value.trim())
          .map(async (e) => {
            try {
              next[e.id] = await api.totpPreview(e.value);
            } catch (err) {
              next[e.id] = errorText(err);
            }
          }),
      );
      if (!cancelled) setPreviews(next);
    };
    tick();
    const timer = window.setInterval(tick, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [entries]);

  const save = useCallback(async () => {
    try {
      // Links and templates are saved as they were last saved; their unsaved
      // edits stay in the list above.
      const others = snapshot.items.filter((i) => !isTotp(i));
      const s = await api.saveItems([...others, ...entries]);
      onUpdate(s);
      setEntries(s.items.filter(isTotp));
      setDirty(false);
      setError(null);
      setSaved(true);
      setOpenKeys(new Set());
      setRevealed(new Set());
    } catch (e) {
      setError(errorText(e));
    }
  }, [snapshot, entries, onUpdate]);

  const importQr = async (id: string, blob: Blob) => {
    try {
      const link = toTotpLink(await decodeQrImage(blob));
      const entry = entries.find((e) => e.id === id);
      const label = entry?.label.trim() ? entry.label : labelFromLink(link);
      update(id, { value: link, label });
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  };

  const pickFile = (id: string) => {
    importTarget.current = id;
    fileInput.current?.click();
  };

  const onFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    const id = importTarget.current;
    e.target.value = "";
    if (file && id) importQr(id, file);
  };

  /** A screenshot pasted anywhere in the entry is read as a QR code. */
  const onPaste = (id: string, e: React.ClipboardEvent) => {
    const image = Array.from(e.clipboardData.items).find((i) => i.type.startsWith("image/"));
    const blob = image?.getAsFile();
    if (blob) {
      e.preventDefault();
      importQr(id, blob);
    }
  };

  const toggle = (set: React.Dispatch<React.SetStateAction<Set<string>>>, id: string) =>
    set((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const copy = async (id: string, code: string) => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(id);
      window.setTimeout(() => setCopied((c) => (c === id ? null : c)), 1200);
    } catch {
      // The clipboard can be refused; copying is only a convenience.
    }
  };

  return (
    <section className="card twofa">
      <div className="card-title">
        <h2>2FA codes</h2>
        <span className="card-hint">Pastes the current code, like an authenticator app</span>
      </div>

      {entries.length === 0 && (
        <div className="empty">
          No 2FA codes yet. Add one with the secret key or QR code a site shows when you turn on
          two-factor authentication.
        </div>
      )}

      <div className="twofa-list">
        {entries.map((entry) => {
          const preview = previews[entry.id];
          const valid = preview !== undefined && typeof preview !== "string";
          const keyOpen = openKeys.has(entry.id) || !entry.value.trim();
          const shown = revealed.has(entry.id);
          return (
            <div
              className={keyOpen ? "twofa-entry open" : "twofa-entry"}
              key={entry.id}
              onPaste={(e) => onPaste(entry.id, e)}
            >
              <div className="twofa-main">
                <Countdown preview={valid ? preview : null} />
                <input
                  className="twofa-label"
                  value={entry.label}
                  placeholder="Label, e.g. GitHub"
                  onChange={(e) => update(entry.id, { label: e.target.value })}
                />
                {valid ? (
                  <button
                    className={preview.remaining <= 5 ? "totp-code expiring" : "totp-code"}
                    title="Click to copy"
                    onClick={() => copy(entry.id, preview.code)}
                  >
                    {copied === entry.id ? "Copied" : groupDigits(preview.code)}
                  </button>
                ) : (
                  <span className="totp-code placeholder">
                    {typeof preview === "string" ? "Invalid key" : "––– –––"}
                  </span>
                )}
                <input
                  className="code-input"
                  value={entry.code ?? ""}
                  placeholder=":2fa"
                  title="Short code"
                  spellCheck={false}
                  onChange={(e) => update(entry.id, { code: e.target.value })}
                />
                <div className="row-actions">
                  <button
                    className={keyOpen ? "subtle active" : "subtle"}
                    title="Change the secret key"
                    disabled={!entry.value.trim()}
                    onClick={() => toggle(setOpenKeys, entry.id)}
                  >
                    Edit key
                  </button>
                  <button
                    title="Delete"
                    className="danger"
                    onClick={() => edit(entries.filter((e) => e.id !== entry.id))}
                  >
                    ✕
                  </button>
                </div>
              </div>

              {keyOpen && (
                <div className="twofa-key">
                  <div className="secret-field">
                    <input
                      type={shown ? "text" : "password"}
                      value={entry.value}
                      placeholder="Secret key or otpauth:// link"
                      spellCheck={false}
                      autoComplete="off"
                      autoFocus={!entry.value}
                      onChange={(e) => update(entry.id, { value: e.target.value })}
                    />
                    {entry.value && (
                      <button
                        className="icon"
                        title={shown ? "Hide secret" : "Show secret"}
                        onClick={() => toggle(setRevealed, entry.id)}
                      >
                        {shown ? "Hide" : "Show"}
                      </button>
                    )}
                  </div>
                  <span className="twofa-or">or</span>
                  <button onClick={() => pickFile(entry.id)}>Import QR image</button>
                  <p className="twofa-tip">
                    {typeof preview === "string" ? (
                      <span className="error">{preview}</span>
                    ) : (
                      <>
                        Tip: snip the site's QR code with <kbd>Win</kbd>+<kbd>Shift</kbd>+
                        <kbd>S</kbd>, then press <kbd>Ctrl</kbd>+<kbd>V</kbd> here.
                      </>
                    )}
                  </p>
                </div>
              )}
            </div>
          );
        })}
      </div>

      <input ref={fileInput} type="file" accept="image/*" hidden onChange={onFile} />

      <div className="items-foot">
        <div className="add-buttons">
          <button className="ghost" onClick={add}>
            + Add 2FA
          </button>
        </div>
        <div className="save-area">
          {error && <span className="error inline">{error}</span>}
          {saved && !dirty && <span className="ok">Saved</span>}
          <button className="primary" disabled={!dirty} onClick={save}>
            Save
          </button>
        </div>
      </div>
    </section>
  );
}

/** "564935" → "564 935"; 7–8 digit codes split near the middle. */
function groupDigits(code: string): string {
  const half = Math.ceil(code.length / 2);
  return `${code.slice(0, half)} ${code.slice(half)}`;
}

/** A ring that empties as the current code runs out. */
function Countdown({ preview }: { preview: TotpPreview | null }) {
  const r = 9;
  const circumference = 2 * Math.PI * r;
  const fraction = preview ? preview.remaining / preview.period : 0;
  const expiring = preview !== null && preview.remaining <= 5;
  return (
    <svg
      className={expiring ? "countdown expiring" : "countdown"}
      viewBox="0 0 24 24"
      width="24"
      height="24"
      role="img"
      aria-label={preview ? `${preview.remaining} seconds left` : "No code"}
    >
      <title>{preview ? `${preview.remaining}s left` : "No code"}</title>
      <circle className="countdown-track" cx="12" cy="12" r={r} />
      <circle
        className="countdown-fill"
        cx="12"
        cy="12"
        r={r}
        strokeDasharray={circumference}
        strokeDashoffset={circumference * (1 - fraction)}
        transform="rotate(-90 12 12)"
      />
    </svg>
  );
}
