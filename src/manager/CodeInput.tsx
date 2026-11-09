/** Every short code starts with ":" so it never fires inside ordinary words. */
export const CODE_PREFIX = ":";

/** The part of a code after the ":" (codes saved before the prefix existed may lack it). */
function stripPrefix(code: string | null | undefined): string {
  return (code ?? "").replace(/^:+/, "");
}

type Props = {
  code: string | null | undefined;
  /** Receives the full code including ":", or "" when cleared. */
  onChange: (code: string) => void;
  placeholder: string;
  title?: string;
};

/** Short-code field with a fixed ":" in front; the user types only the rest. */
export default function CodeInput({ code, onChange, placeholder, title }: Props) {
  return (
    <span className="code-field" title={title}>
      <span className="code-prefix" aria-hidden="true">
        {CODE_PREFIX}
      </span>
      <input
        className="code-input"
        value={stripPrefix(code)}
        placeholder={placeholder}
        aria-label={title ?? "Short code"}
        spellCheck={false}
        onChange={(e) => {
          const rest = stripPrefix(e.target.value);
          onChange(rest ? CODE_PREFIX + rest : "");
        }}
      />
    </span>
  );
}
