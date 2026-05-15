// ASCII banners for the assistant overlay's empty state. Renders
// the active backend's name in a 6-row block-character font (the
// classic "ANSI Shadow" figlet style). Theme-agnostic: each glyph
// is plain text, so the surrounding <pre> picks up the current
// `color` from CSS and inverts cleanly in dark mode.
//
// Why bake the alphabet by hand instead of shipping a figlet:
// we only need ~12 letters, the glyphs never change, and the
// alternative is dragging a multi-kilobyte font runtime + parser
// into the bundle to render 6 lines of text on overlay open.

/// Each glyph is six lines of equal width (the trailing space on
/// each row provides inter-letter kerning when glyphs concatenate).
const ALPHABET: Record<string, string[]> = {
  A: [
    " █████╗  ",
    "██╔══██╗ ",
    "███████║ ",
    "██╔══██║ ",
    "██║  ██║ ",
    "╚═╝  ╚═╝ ",
  ],
  C: [
    " ██████╗ ",
    "██╔════╝ ",
    "██║      ",
    "██║      ",
    "╚██████╗ ",
    " ╚═════╝ ",
  ],
  D: [
    "██████╗  ",
    "██╔══██╗ ",
    "██║  ██║ ",
    "██║  ██║ ",
    "██████╔╝ ",
    "╚═════╝  ",
  ],
  E: [
    "███████╗ ",
    "██╔════╝ ",
    "█████╗   ",
    "██╔══╝   ",
    "███████╗ ",
    "╚══════╝ ",
  ],
  G: [
    " ██████╗  ",
    "██╔════╝  ",
    "██║  ███╗ ",
    "██║   ██║ ",
    "╚██████╔╝ ",
    " ╚═════╝  ",
  ],
  I: [
    "██╗ ",
    "██║ ",
    "██║ ",
    "██║ ",
    "██║ ",
    "╚═╝ ",
  ],
  L: [
    "██╗      ",
    "██║      ",
    "██║      ",
    "██║      ",
    "███████╗ ",
    "╚══════╝ ",
  ],
  M: [
    "███╗   ███╗ ",
    "████╗ ████║ ",
    "██╔████╔██║ ",
    "██║╚██╔╝██║ ",
    "██║ ╚═╝ ██║ ",
    "╚═╝     ╚═╝ ",
  ],
  N: [
    "███╗   ██╗ ",
    "████╗  ██║ ",
    "██╔██╗ ██║ ",
    "██║╚██╗██║ ",
    "██║ ╚████║ ",
    "╚═╝  ╚═══╝ ",
  ],
  O: [
    " ██████╗  ",
    "██╔═══██╗ ",
    "██║   ██║ ",
    "██║   ██║ ",
    "╚██████╔╝ ",
    " ╚═════╝  ",
  ],
  U: [
    "██╗   ██╗ ",
    "██║   ██║ ",
    "██║   ██║ ",
    "██║   ██║ ",
    "╚██████╔╝ ",
    " ╚═════╝  ",
  ],
  X: [
    "██╗  ██╗ ",
    "╚██╗██╔╝ ",
    " ╚███╔╝  ",
    " ██╔██╗  ",
    "██╔╝ ██╗ ",
    "╚═╝  ╚═╝ ",
  ],
  " ": [
    "    ",
    "    ",
    "    ",
    "    ",
    "    ",
    "    ",
  ],
};

/// Render `text` as a 6-line ANSI Shadow banner. Letters outside the
/// baked alphabet are skipped (assistant names are uppercase A-Z so
/// this never trips today; the no-op fallback keeps the function
/// total instead of throwing on an unexpected character).
export function banner(text: string): string {
  const rows: string[] = ["", "", "", "", "", ""];
  for (const raw of text.toUpperCase()) {
    const glyph = ALPHABET[raw];
    if (!glyph) continue;
    for (let i = 0; i < 6; i++) rows[i] += glyph[i];
  }
  return rows.join("\n");
}

/// Friendly display name for an LLM backend id. Maps the snake_case
/// kinds chan-llm uses on the wire to the marketing names users
/// recognise. Falls back to the raw id uppercased so a future
/// backend lands legibly without an update here.
export function displayAgentName(backend: string | null | undefined): string {
  switch (backend) {
    case "anthropic":
    case "claude":
      return "CLAUDE";
    case "claude_cli":
      return "CLAUDE CLI";
    case "gemini":
      return "GEMINI";
    case "gemini_cli":
      return "GEMINI CLI";
    case "codex_cli":
      return "CODEX CLI";
    case "ollama":
      return "OLLAMA";
    default:
      return (backend ?? "").toUpperCase();
  }
}
