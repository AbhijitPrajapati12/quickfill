import { useEffect, useRef, useState } from "react";
import type { Item } from "../api";

interface Props {
  initial: Item;
  isNew: boolean;
  /** Resolves when saved; rejects with a message to show (e.g. a validation error). */
  onSave: (item: Item) => Promise<void>;
  onCancel: () => void;
}

/** Full-page editor for a multi-line template (an email, notes, a list of links). */
export default function TemplateEditor({ initial, isNew, onSave, onCancel }: Props) {
  const [draft, setDraft] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const labelRef = useRef<HTMLInputElement>(null);
  const bodyRef = useRef<HTMLTextAreaElement>(null);

  const changed =
    draft.label !== initial.label ||
    draft.value !== initial.value ||
    (draft.code ?? "") !== (initial.code ?? "");

  useEffect(() => {
    (isNew ? labelRef : bodyRef).current?.focus();
  }, [isNew]);

  const save = async () => {
    setSaving(true);
    try {
      await onSave(draft);
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (!saving) save();
      } else if (e.key === "Escape" && !changed) {
        onCancel();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  const lines = draft.value === "" ? 0 : draft.value.split("\n").length;

  return (
    <main className="manager template-page">
      <header className="template-page-head">
        <button className="back" onClick={onCancel} title="Back to items">
          ← Back
        </button>
        <h1>{isNew ? "New template" : "Edit template"}</h1>
      </header>

      <section className="card template-form">
        <div className="template-fields">
          <label>
            <span>Label</span>
            <input
              ref={labelRef}
              value={draft.label}
              placeholder="Follow-up email"
              onChange={(e) => setDraft({ ...draft, label: e.target.value })}
            />
          </label>
          <label className="template-code">
            <span>Short code</span>
            <input
              className="code-input"
              value={draft.code ?? ""}
              placeholder=":mail"
              spellCheck={false}
              onChange={(e) => setDraft({ ...draft, code: e.target.value })}
            />
          </label>
        </div>

        <label className="template-body">
          <span>Text</span>
          <textarea
            ref={bodyRef}
            className="template-editor"
            value={draft.value}
            placeholder={
              "Hi {name},\n\nThanks for your time today. Here are the links we discussed:\n- https://…\n\nBest,\nYou"
            }
            onChange={(e) => setDraft({ ...draft, value: e.target.value })}
          />
        </label>

        <div className="items-foot">
          <span className="template-meta">
            {lines === 1 ? "1 line" : `${lines} lines`} · {draft.value.length} characters
          </span>
          <div className="save-area">
            {error && <span className="error inline">{error}</span>}
            <button onClick={onCancel}>{changed ? "Discard" : "Cancel"}</button>
            <button className="primary" disabled={saving || (!isNew && !changed)} onClick={save}>
              Save
            </button>
          </div>
        </div>
      </section>
    </main>
  );
}
