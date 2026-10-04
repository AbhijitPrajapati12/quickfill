import { useState } from "react";
import { api, errorText, type Snapshot } from "../api";
import HotkeyRecorder from "./HotkeyRecorder";

interface Props {
  snapshot: Snapshot;
  onUpdate: (s: Snapshot) => void;
}

export default function Settings({ snapshot, onUpdate }: Props) {
  const { settings, hotkeyError } = snapshot;
  const [error, setError] = useState<string | null>(null);

  const run = async (action: () => Promise<Snapshot>) => {
    try {
      onUpdate(await action());
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  };

  return (
    <section className="card settings">
      <h2>Settings</h2>

      <div className="setting">
        <div>
          <div className="setting-title">Popup shortcut</div>
          <div className="setting-desc">Press it in any text field to open the picker.</div>
        </div>
        <HotkeyRecorder
          value={settings.hotkey}
          onChange={(accel) => run(() => api.setHotkey(accel))}
        />
      </div>

      <label className="setting">
        <div>
          <div className="setting-title">Short codes</div>
          <div className="setting-desc">Type a code like :gh anywhere to replace it with its value.</div>
        </div>
        <input
          type="checkbox"
          className="switch"
          checked={settings.codesEnabled}
          onChange={(e) => run(() => api.setCodesEnabled(e.target.checked))}
        />
      </label>

      <label className="setting">
        <div>
          <div className="setting-title">Start with Windows</div>
          <div className="setting-desc">Launch quietly into the tray when you sign in.</div>
        </div>
        <input
          type="checkbox"
          className="switch"
          checked={settings.autostart}
          onChange={(e) => run(() => api.setAutostart(e.target.checked))}
        />
      </label>

      {(error ?? hotkeyError) && <div className="error">{error ?? hotkeyError}</div>}
    </section>
  );
}
