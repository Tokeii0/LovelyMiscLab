// Helpers deciding whether a keyboard shortcut belongs to the canvas or to
// whatever the user is typing in / has selected.

/** Focus is in something that takes text input (typing must not trigger canvas keys). */
export function isTextInput(target: EventTarget | null): boolean {
  const t = target as HTMLElement | null;
  if (!t || !t.tagName) return false;
  return t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable;
}

/** The user has selected page text (Ctrl+C should copy it, not the selected nodes). */
export function hasTextSelection(): boolean {
  const s = typeof window !== "undefined" ? window.getSelection() : null;
  return !!s && !s.isCollapsed && s.toString().trim().length > 0;
}
