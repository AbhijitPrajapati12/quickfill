import { useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Item, type Snapshot } from "../api";

export default function Picker() {
  const [items, setItems] = useState<Item[]>([]);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    api.getData().then((s) => setItems(s.items));
    // Rust sends fresh items every time the hotkey opens the picker.
    const unlisten = listen<Snapshot>("picker:open", ({ payload }) => {
      setItems(payload.items);
      setQuery("");
      setSelected(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    });
    const refocus = () => inputRef.current?.focus();
    window.addEventListener("focus", refocus);
    return () => {
      unlisten.then((f) => f());
      window.removeEventListener("focus", refocus);
    };
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (i) =>
        i.label.toLowerCase().includes(q) ||
        i.value.toLowerCase().includes(q) ||
        (i.code ?? "").toLowerCase().includes(q),
    );
  }, [items, query]);

  useEffect(() => {
    listRef.current
      ?.querySelector<HTMLElement>(`[data-index="${selected}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const choose = (item: Item | undefined) => {
    if (item) api.pasteItem(item.id).catch(console.error);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      api.hidePicker();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, filtered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      choose(filtered[selected]);
    } else if (query === "" && /^[1-9]$/.test(e.key)) {
      // Number quick-pick works only before you start searching.
      e.preventDefault();
      choose(filtered[Number(e.key) - 1]);
    }
  };

  return (
    <div className="picker" onKeyDown={onKeyDown}>
      <div className="picker-search">
        <input
        ref={inputRef}
        aria-label="Search items"
        placeholder="Search…"
        value={query}
        autoFocus
        spellCheck={false}
        onChange={(e) => {
          setQuery(e.target.value);
          setSelected(0);
        }}
        />
      </div>
      {filtered.length === 0 ? (
        <div className="picker-empty">
          {items.length === 0
            ? "No items yet. Add some from the tray."
            : "No matches"}
        </div>
      ) : (
        <ul className="picker-list" ref={listRef}>
          {filtered.map((item, index) => (
            <li
              key={item.id}
              data-index={index}
              className={index === selected ? "picker-row selected" : "picker-row"}
              onMouseMove={() => setSelected(index)}
              onMouseDown={(e) => {
                e.preventDefault();
                choose(item);
              }}
            >
              <span className="picker-label">{item.label}</span>
              {item.kind === "template" && <span className="picker-tag">template</span>}
            </li>
          ))}
        </ul>
      )}
      <div className="picker-hint">
        <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
        <span><kbd>↵</kbd> paste</span>
        <span><kbd>esc</kbd> close</span>
      </div>
    </div>
  );
}
