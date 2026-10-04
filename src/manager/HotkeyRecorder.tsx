import { useEffect, useState } from "react";

interface Props {
  value: string;
  onChange: (accel: string) => void;
}

/** Shows "Super" (the plugin's name for the Windows key) as "Win". */
export const displayHotkey = (accel: string) => accel.replace(/Super/g, "Win");

/** Maps KeyboardEvent.code to the shortcut plugin's key names. */
function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (code === "Space") return "Space";
  return null;
}

export default function HotkeyRecorder({ value, onChange }: Props) {
  const [recording, setRecording] = useState(false);
  const [hint, setHint] = useState("");

  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(false);
        setHint("");
        return;
      }
      const key = keyName(e.code);
      if (!key) {
        setHint("Use a letter, number, F-key or Space with modifiers.");
        return;
      }
      if (!e.ctrlKey && !e.altKey && !e.metaKey) {
        setHint("Include Ctrl, Alt or Win so it doesn't clash with normal typing.");
        return;
      }
      const parts = [
        e.ctrlKey && "Ctrl",
        e.altKey && "Alt",
        e.shiftKey && "Shift",
        e.metaKey && "Super",
        key,
      ].filter(Boolean);
      setRecording(false);
      setHint("");
      onChange(parts.join("+"));
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recording, onChange]);

  return (
    <div className="hotkey">
      <button
        type="button"
        className={recording ? "hotkey-btn recording" : "hotkey-btn"}
        onClick={() => setRecording((r) => !r)}
        onBlur={() => setRecording(false)}
      >
        {recording ? "Press your shortcut… (Esc to cancel)" : displayHotkey(value)}
      </button>
      {hint && <span className="hotkey-hint">{hint}</span>}
    </div>
  );
}
