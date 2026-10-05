import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, errorText, type Item, type ItemKind, type Snapshot } from "../api";
import Settings from "./Settings";
import TemplateEditor from "./TemplateEditor";
import { displayHotkey } from "./HotkeyRecorder";

export default function Manager() {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [items, setItems] = useState<Item[]>([]);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  // The template open in the full-page editor, if any.
  const [editing, setEditing] = useState<{ item: Item; isNew: boolean } | null>(null);

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

  const add = (kind: ItemKind) => {
    const item: Item = { id: crypto.randomUUID(), label: "", value: "", code: "", kind };
    if (kind === "template") setEditing({ item, isNew: true });
    else edit([...items, item]);
  };

  const move = (index: number, delta: number) => {
    const target = index + delta;
    if (target < 0 || target >= items.length) return;
    const next = [...items];
    [next[index], next[target]] = [next[target], next[index]];
    edit(next);
  };

  const persist = async (list: Item[]) => {
    const s = await api.saveItems(list);
    setSnapshot(s);
    setItems(s.items);
    setDirty(false);
    setError(null);
    setSaved(true);
  };

  const save = useCallback(async () => {
    try {
      await persist(items);
    } catch (e) {
      setError(errorText(e));
    }
  }, [items]);

  /** Saves the template page's draft (and any other pending edits), then returns to the list. */
  const saveTemplate = async (draft: Item) => {
    const exists = items.some((i) => i.id === draft.id);
    const list = exists ? items.map((i) => (i.id === draft.id ? draft : i)) : [...items, draft];
    try {
      await persist(list);
    } catch (e) {
      throw errorText(e);
    }
    setEditing(null);
  };

  useEffect(() => {
    if (editing) return; // The template page handles its own shortcuts.
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (dirty) save();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [dirty, save, editing]);

  if (!snapshot) return null;

  if (editing) {
    return (
      <TemplateEditor
        key={editing.item.id}
        initial={editing.item}
        isNew={editing.isNew}
        onSave={saveTemplate}
        onCancel={() => setEditing(null)}
      />
    );
  }

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
        {items.map((item, index) => {
          const isTemplate = item.kind === "template";
          return (
            <div className={isTemplate ? "item-row template" : "item-row"} key={item.id}>
              <input
                value={item.label}
                placeholder={isTemplate ? "Follow-up email" : "GitHub"}
                onChange={(e) => update(index, { label: e.target.value })}
              />
              {isTemplate ? (
                <button
                  className="template-summary"
                  title="Open the template editor"
                  onClick={() => setEditing({ item, isNew: false })}
                >
                  <span className="template-badge">Template</span>
                  <span className="template-preview">
                    {item.value.split("\n").find((l) => l.trim()) || "Empty — click to write"}
                  </span>
                  <span className="template-lines">
                    {lineCount(item.value)} ›
                  </span>
                </button>
              ) : (
                <input
                  value={item.value}
                  placeholder="https://github.com/you"
                  spellCheck={false}
                  onChange={(e) => update(index, { value: e.target.value })}
                />
              )}
              <input
                className="code-input"
                value={item.code ?? ""}
                placeholder={isTemplate ? ":mail" : ":gh"}
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
          );
        })}
        <div className="items-foot">
          <div className="add-buttons">
            <button className="ghost" onClick={() => add("text")}>
              + Add item
            </button>
            <button
              className="ghost"
              title="Multi-line text: an email, notes, a list of links"
              onClick={() => add("template")}
            >
              + Add template
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

      <Settings snapshot={snapshot} onUpdate={setSnapshot} />
    </main>
  );
}

function lineCount(value: string): string {
  const n = value === "" ? 0 : value.split("\n").length;
  return n === 1 ? "1 line" : `${n} lines`;
}
