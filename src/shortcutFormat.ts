const MODIFIER_ORDER = ["Cmd", "Shift", "Alt", "Ctrl"] as const;
const MODIFIER_ALIASES: Record<string, (typeof MODIFIER_ORDER)[number]> = {
  cmd: "Cmd",
  command: "Cmd",
  super: "Cmd",
  meta: "Cmd",
  shift: "Shift",
  alt: "Alt",
  option: "Alt",
  ctrl: "Ctrl",
  control: "Ctrl",
};

function normalizeKeyToken(token: string): string | null {
  const trimmed = token.trim();
  if (!trimmed) return null;

  const lower = trimmed.toLowerCase();
  if (lower === "minus" || trimmed === "-") {
    return "-";
  }
  if (lower === "equal" || lower === "equals" || trimmed === "=") {
    return "=";
  }
  if (lower === "bracketleft" || lower === "lbracket" || trimmed === "[") {
    return "[";
  }
  if (lower === "bracketright" || lower === "rbracket" || trimmed === "]") {
    return "]";
  }
  if (lower === "semicolon" || trimmed === ";") {
    return ";";
  }
  if (lower === "quote" || trimmed === "'") {
    return "'";
  }
  if (lower === "comma" || trimmed === ",") {
    return ",";
  }
  if (lower === "period" || trimmed === ".") {
    return ".";
  }
  if (lower === "slash" || trimmed === "/") {
    return "/";
  }
  if (lower === "backslash" || trimmed === "\\") {
    return "\\";
  }
  if (lower === "backquote" || lower === "grave" || trimmed === "`") {
    return "`";
  }
  if (lower === "intlbackslash") {
    return "\\";
  }
  if (/^digit[0-9]$/.test(lower)) {
    return lower.replace("digit", "");
  }
  if (/^[0-9]$/.test(trimmed)) {
    return trimmed;
  }
  if (/^key[a-z]$/.test(lower)) {
    return lower.replace("key", "").toUpperCase();
  }
  if (/^[a-z]$/.test(lower)) {
    return lower.toUpperCase();
  }
  if (/^f([1-9]|1[0-2])$/.test(lower)) {
    return lower.toUpperCase();
  }

  switch (lower) {
    case "space":
      return "Space";
    case "enter":
      return "Enter";
    case "tab":
      return "Tab";
    case "escape":
    case "esc":
      return "Escape";
    case "backspace":
      return "Backspace";
    default:
      return null;
  }
}

export function normalizeShortcutString(shortcut: string): string | null {
  if (!shortcut) return null;

  const parts = shortcut
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);

  if (parts.length < 2) return null;

  const modifiers = new Set<(typeof MODIFIER_ORDER)[number]>();
  let key: string | null = null;

  for (const part of parts) {
    const mapped = MODIFIER_ALIASES[part.toLowerCase()];
    if (mapped) {
      modifiers.add(mapped);
      continue;
    }

    if (key) {
      return null;
    }

    const normalizedKey = normalizeKeyToken(part);
    if (!normalizedKey) {
      return null;
    }
    key = normalizedKey;
  }

  if (!key || modifiers.size === 0) return null;

  const orderedModifiers = MODIFIER_ORDER.filter((modifier) => modifiers.has(modifier));
  return [...orderedModifiers, key].join("+");
}

export function formatShortcutForDisplay(shortcut: string): string {
  const normalized = normalizeShortcutString(shortcut) ?? shortcut;
  return normalized
    .replace(/Cmd/g, "⌘")
    .replace(/Shift/g, "⇧")
    .replace(/Alt/g, "⌥")
    .replace(/Ctrl/g, "⌃")
    .replace(/\+/g, "");
}
