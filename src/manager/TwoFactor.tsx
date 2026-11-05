import { useCallback, useEffect, useRef, useState } from "react";
import { api, errorText, type Item, type Snapshot, type TotpPreview } from "../api";
import { decodeQrImage, labelFromLink, toTotpLink } from "./qr";

export type TwoFactorKind = "totp" | "passwordTotp";

interface Props {
  snapshot: Snapshot;
  onUpdate: (s: Snapshot) => void;
  /** "totp" pastes the code; "passwordTotp" pastes the password followed by the code. */
  kind: TwoFactorKind;
}

const COPY: Record<TwoFactorKind, { title: string; hint: string; empty: string; add: string }> = {
  totp: {
    title: "2FA codes",
    hint: "Pastes the current code, like an authenticator app",
    empty:
      "No 2FA codes yet. Add one with the secret key or QR code a site shows when you turn on two-factor authentication.",
    add: "+ Add 2FA",
  },
  passwordTotp: {
    title: "Password + 2FA",
    hint: "Pastes the password followed by the current code",
    empty:
      "Nothing here yet. Use this for logins like VPNs that want your password and 2FA code typed together in one field.",
    add: "+ Add password + 2FA",
  },
};

/**
 * A card of 2FA entries, saved separately from the links/templates list.
 * For "passwordTotp" entries the item's `value` is the password and `secret`
 * the 2FA key; for "totp" entries the `value` is the key.
 */
export default function TwoFactor({ snapshot, onUpdate, kind }: Props) {
  const withPassword = kind === "passwordTotp";
  const text = COPY[kind];
  const ofKind = useCallback((i: Item) => i.kind === kind, [kind]);
  const secretOf = (e: Item) => (withPassword ? (e.secret ?? "") : e.value);
  const secretPatch = (s: string): Partial<Item> => (withPassword ? { secret: s } : { value: s });

  const [entries, setEntries] = useState<Item[]>(() => snapshot.items.filter(ofKind));
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [previews, setPreviews] = useState<Record<string, TotpPreview | string>>({});
  // Ids of revealed fields, as `${id}:secret` or `${id}:password`.
  const [revealed, setRevealed] = useState<Set<string>>(new Set());
  // Entries whose key panel is open. New or incomplete entries are always open.
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
    const entry: Item = { id, label: "", value: "", code: "", kind };
    if (withPassword) entry.secret = "";
    edit([...entries, entry]);
    setOpenKeys((prev) => new Set(prev).add(id));
  };

  // Refresh the live codes every second so they can be checked against a phone app.
  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      const next: Record<string, TotpPreview | string> = {};
      await Promise.all(
        entries
          .filter((e) => secretOf(e).trim())
          .map(async (e) => {
            try {
              next[e.id] = await api.totpPreview(secretOf(e));
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
    // secretOf only depends on `kind`, which is fixed per card.
  }, [entries]);

  const save = useCallback(async () => {
    try {
      // Everything outside this card is saved as it was last saved; unsaved
      // edits in other cards stay where they are.
      const others = snapshot.items.filter((i) => !ofKind(i));
      const s = await api.saveItems([...others, ...entries]);
      onUpdate(s);
      setEntries(s.items.filter(ofKind));
      setDirty(false);
      setError(null);
      setSaved(true);
      setOpenKeys(new Set());
      setRevealed(new Set());
    } catch (e) {
      setError(errorText(e));
    }
  }, [snapshot, entries, onUpdate, ofKind]);

  const importQr = async (id: string, blob: Blob) => {
    try {
      const link = toTotpLink(await decodeQrImage(blob));
      const entry = entries.find((e) => e.id === id);
      const label = entry?.label.trim() ? entry.label : labelFromLink(link);
      update(id, { ...secretPatch(link), label });
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

  const toggle = (set: React.Dispatch<React.SetStateAction<Set<string>>>, key: string) =>
    set((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  const copy = async (entry: Item, code: string) => {
    try {
      // Copy exactly what the shortcut would paste.
      await navigator.clipboard.writeText(withPassword ? entry.value + code : code);
      setCopied(entry.id);
      window.setTimeout(() => setCopied((c) => (c === entry.id ? null : c)), 1200);
    } catch {
      // The clipboard can be refused; copying is only a convenience.
    }
  };

  return (
    <section className="card twofa">
      <div className="card-title">
        <h2>{text.title}</h2>
        <span className="card-hint">{text.hint}</span>
      </div>

      {entries.length === 0 && <div className="empty">{text.empty}</div>}

      <div className="twofa-list">
        {entries.map((entry) => {
          const preview = previews[entry.id];
          const valid = preview !== undefined && typeof preview !== "string";
          const secret = secretOf(entry);
          const complete = !!secret.trim() && (!withPassword || entry.value !== "");
          const keyOpen = openKeys.has(entry.id) || !complete;
          const secretShown = revealed.has(`${entry.id}:secret`);
          const passwordShown = revealed.has(`${entry.id}:password`);
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
                  placeholder={withPassword ? "Label, e.g. Work VPN" : "Label, e.g. GitHub"}
                  onChange={(e) => update(entry.id, { label: e.target.value })}
                />
                {valid ? (
                  <button
                    className={preview.remaining <= 5 ? "totp-code expiring" : "totp-code"}
                    title={withPassword ? "Click to copy password + code" : "Click to copy"}
                    onClick={() => copy(entry, preview.code)}
                  >
                    {copied === entry.id ? (
                      "Copied"
                    ) : (
                      <>
                        {withPassword && <span className="totp-prefix">•••• +</span>}
                        {groupDigits(preview.code)}
                      </>
                    )}
                  </button>
                ) : (
                  <span className="totp-code placeholder">
                    {typeof preview === "string" ? "Invalid key" : "––– –––"}
                  </span>
                )}
                <input
                  className="code-input"
                  value={entry.code ?? ""}
                  placeholder={withPassword ? ":vpn" : ":2fa"}
                  title="Short code"
                  spellCheck={false}
                  onChange={(e) => update(entry.id, { code: e.target.value })}
                />
                <div className="row-actions">
                  <button
                    className={keyOpen ? "subtle active" : "subtle"}
                    title={withPassword ? "Change the password or secret key" : "Change the secret key"}
                    disabled={!complete}
                    onClick={() => toggle(setOpenKeys, entry.id)}
                  >
                    {withPassword ? "Edit" : "Edit key"}
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
                  {withPassword && (
                    <>
                      <SecretInput
                        value={entry.value}
                        placeholder="Password"
                        shown={passwordShown}
                        autoFocus={!entry.value}
                        onToggle={() => toggle(setRevealed, `${entry.id}:password`)}
                        onChange={(v) => update(entry.id, { value: v })}
                      />
                      <span />
                      <span />
                    </>
                  )}
                  <SecretInput
                    value={secret}
                    placeholder="2FA secret key or otpauth:// link"
                    shown={secretShown}
                    autoFocus={!withPassword && !secret}
                    onToggle={() => toggle(setRevealed, `${entry.id}:secret`)}
                    onChange={(v) => update(entry.id, secretPatch(v))}
                  />
                  <span className="twofa-or">or</span>
                  <button onClick={() => pickFile(entry.id)}>Import QR image</button>
                  <p className="twofa-tip">
                    {typeof preview === "string" ? (
                      <span className="error">{preview}</span>
                    ) : (
                      <>
                        {withPassword && "Both are required. "}
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
            {text.add}
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

/** A masked input with a Show/Hide toggle. */
function SecretInput(props: {
  value: string;
  placeholder: string;
  shown: boolean;
  autoFocus?: boolean;
  onToggle: () => void;
  onChange: (value: string) => void;
}) {
  return (
    <div className="secret-field">
      <input
        type={props.shown ? "text" : "password"}
        value={props.value}
        placeholder={props.placeholder}
        spellCheck={false}
        autoComplete="off"
        autoFocus={props.autoFocus}
        onChange={(e) => props.onChange(e.target.value)}
      />
      {props.value && (
        <button
          className="icon"
          title={props.shown ? "Hide" : "Show"}
          onClick={props.onToggle}
        >
          {props.shown ? "Hide" : "Show"}
        </button>
      )}
    </div>
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
