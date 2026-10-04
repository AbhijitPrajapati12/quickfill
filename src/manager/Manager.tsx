import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText, type Item, type Snapshot } from "../api";
import Settings from "./Settings";
import { displayHotkey } from "./HotkeyRecorder";

export default function Manager() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [items, setItems] = useState<Item[]>([]);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    api.getData().then((s) => {
      setSnapshot(s);
      setItems(s.items);
    });
    // Settings can change from the tray menu; keep unsaved item edits intact.
    const unlisten = listen<Snapshot>("data:changed", ({ payload }) =>
      setSnapshot((prev) => (prev ? { ...prev, settings: payload.settings } : payload)),
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const edit = (next: Item[]) => {
    setItems(next);
    setDirty(true);
    setSaved(false);
  };

  const update = (index: number, patch: Partial<Item>) =>
    edit(items.map((item, i) => (i === index ? { ...item, ...patch } : item)));

  const move = (index: number, delta: number) => {
    const target = index + delta;
    if (target < 0 || target >= items.length) return;
    const next = [...items];
    [next[index], next[target]] = [next[target], next[index]];
    edit(next);
  };

  const save = useCallback(async () => {
    try {
      const s = await api.saveItems(items);
      setSnapshot(s);
      setItems(s.items);
      setDirty(false);
      setError(null);
      setSaved(true);
    } catch (e) {
      setError(errorText(e));
    }
  }, [items]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (dirty) save();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [dirty, save]);

  if (!snapshot) return null;

  return (
    <main className="manager">
      <header className="header">
        <img className="logo" src="/logo.png" alt="" />
        <div>
          <h1>QuickFill</h1>
          <p className="subtitle">
            Press <kbd>{displayHotkey(snapshot.settings.hotkey)}</kbd> in any text field, or type a
            short code.
          </p>
        </div>
      </header>

      <section className="card">
        <div className="items-head">
          <span>Label</span>
          <span>Value</span>
          <span>Short code</span>
          <span />
        </div>
        {items.length === 0 && <div className="empty">No items yet. Add your first one below.</div>}
        {items.map((item, index) => (
          <div className="item-row" key={item.id}>
            <input
              value={item.label}
              placeholder="GitHub"
              onChange={(e) => update(index, { label: e.target.value })}
            />
            <input
              value={item.value}
              placeholder="https://github.com/you"
              spellCheck={false}
              onChange={(e) => update(index, { value: e.target.value })}
            />
            <input
              className="code-input"
              value={item.code ?? ""}
              placeholder=":gh"
              spellCheck={false}
              onChange={(e) => update(index, { code: e.target.value })}
            />
            <div className="row-actions">
              <button title="Move up" disabled={index === 0} onClick={() => move(index, -1)}>
                ↑
              </button>
              <button
                title="Move down"
                disabled={index === items.length - 1}
                onClick={() => move(index, 1)}
              >
                ↓
              </button>
              <button
                title="Delete"
                className="danger"
                onClick={() => edit(items.filter((_, i) => i !== index))}
              >
                ✕
              </button>
            </div>
          </div>
        ))}
        <div className="items-foot">
          <button
            className="ghost"
            onClick={() => edit([...items, { id: crypto.randomUUID(), label: "", value: "", code: "" }])}
          >
            + Add item
          </button>
          <div className="save-area">
            {error && <span className="error inline">{error}</span>}
            {saved && !dirty && <span className="ok">Saved</span>}
            <button className="primary" disabled={!dirty} onClick={save}>
              Save
            </button>
          </div>
        </div>
      </section>

      <Settings snapshot={snapshot} onUpdate={setSnapshot} />
    </main>
  );
}
