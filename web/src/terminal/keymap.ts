export function terminalMetaKeyBytes(ev: KeyboardEvent): string | null {
  if (ev.type !== "keydown") return null;
  if (!ev.altKey || ev.ctrlKey || ev.metaKey) return null;
  switch (ev.key) {
    case "ArrowLeft":
      return "\x1bb";
    case "ArrowRight":
      return "\x1bf";
    case "Backspace":
      return "\x1b\x7f";
    case "Delete":
      return "\x1bd";
    default:
      return null;
  }
}

export function handleTerminalMetaKey(
  ev: KeyboardEvent,
  sendInput: (data: string) => void,
): boolean {
  const bytes = terminalMetaKeyBytes(ev);
  if (bytes === null) return true;
  sendInput(bytes);
  ev.preventDefault();
  return false;
}
