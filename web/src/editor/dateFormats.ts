// Date pill format catalog.
//
// One source of truth for the date pill UX: each entry pairs a
// human-readable label, a formatter (Date -> string), and a regex
// used both for live-detection (typing a matching string converts
// to a pill) and for round-trip parsing (the markdown carries the
// formatted string verbatim; on load the regex re-pills it).
//
// Year-less formats (e.g. "Mon, 18 Feb") store only month + day on
// disk; the underlying date attribute reattaches the current year
// on parse. That's a deliberate trade-off: a "Mon, 18 Feb" written
// last December reads as this year next time the file is opened.
// If the user wants stable years they pick a year-bearing format.
//
// Adding a new format means: extend `DATE_FORMATS`, add the
// matching `<select>` option (auto-derived), and bump nothing.
// Detection is regex-driven and runs every catalog entry on every
// scan, so old documents keep being auto-pilled.

const MONTH_LONG = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];
const MONTH_SHORT = [
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DOW_SHORT = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

const MONTH_SHORT_RE = MONTH_SHORT.join("|");
const DOW_SHORT_RE = DOW_SHORT.join("|");

function pad2(n: number): string {
  return n.toString().padStart(2, "0");
}

export type DateFormatId = "iso" | "medium" | "short";

export type DateFormatDef = {
  id: DateFormatId;
  /// Label for the picker dropdown.
  label: string;
  /// Whether the formatted output carries a year. Year-less formats
  /// reattach the current year on parse; the pill displays without
  /// the year regardless.
  hasYear: boolean;
  /// Format a Date as the canonical string for this id.
  format: (d: Date) => string;
  /// Pattern fragment (no anchors, no flags) used to detect
  /// occurrences of this format in arbitrary text. Wrapped at
  /// match time with word boundaries.
  pattern: string;
  /// Parse a string previously produced by `format` (or matched
  /// by `pattern`) back into a Date. Returns null on parse failure.
  parse: (s: string) => Date | null;
};

/// Year-less parses default to the current year. Pulled into a
/// function so tests can stub it; production code calls without
/// args and gets `new Date().getFullYear()`.
function currentYear(): number {
  return new Date().getFullYear();
}

const ISO: DateFormatDef = {
  id: "iso",
  label: "2026-05-05 (ISO)",
  hasYear: true,
  format: (d) => `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`,
  pattern: "\\d{4}-\\d{2}-\\d{2}",
  parse: (s) => {
    const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(s);
    if (!m) return null;
    const [, y, mo, da] = m;
    const d = new Date(Number(y), Number(mo) - 1, Number(da));
    if (
      d.getFullYear() !== Number(y) ||
      d.getMonth() !== Number(mo) - 1 ||
      d.getDate() !== Number(da)
    ) {
      return null;
    }
    return d;
  },
};

const MEDIUM: DateFormatDef = {
  id: "medium",
  label: "02 Jan 2029",
  hasYear: true,
  format: (d) => `${pad2(d.getDate())} ${MONTH_SHORT[d.getMonth()]} ${d.getFullYear()}`,
  pattern: `\\d{2} (?:${MONTH_SHORT_RE}) \\d{4}`,
  parse: (s) => {
    const re = new RegExp(`^(\\d{2}) (${MONTH_SHORT_RE}) (\\d{4})$`);
    const m = re.exec(s);
    if (!m) return null;
    const [, da, mo, y] = m;
    const moIdx = MONTH_SHORT.indexOf(mo);
    if (moIdx < 0) return null;
    const d = new Date(Number(y), moIdx, Number(da));
    if (
      d.getFullYear() !== Number(y) ||
      d.getMonth() !== moIdx ||
      d.getDate() !== Number(da)
    ) {
      return null;
    }
    return d;
  },
};

const SHORT: DateFormatDef = {
  id: "short",
  label: "Mon, 18 Feb (no year)",
  hasYear: false,
  format: (d) => `${DOW_SHORT[d.getDay()]}, ${pad2(d.getDate())} ${MONTH_SHORT[d.getMonth()]}`,
  pattern: `(?:${DOW_SHORT_RE}), \\d{2} (?:${MONTH_SHORT_RE})`,
  parse: (s) => {
    const re = new RegExp(`^(${DOW_SHORT_RE}), (\\d{2}) (${MONTH_SHORT_RE})$`);
    const m = re.exec(s);
    if (!m) return null;
    const [, , da, mo] = m;
    const moIdx = MONTH_SHORT.indexOf(mo);
    if (moIdx < 0) return null;
    const y = currentYear();
    const d = new Date(y, moIdx, Number(da));
    if (d.getMonth() !== moIdx || d.getDate() !== Number(da)) return null;
    // We deliberately don't validate that the weekday matches:
    // matching today's calendar would force users to manually pick
    // a future "Mon, 18 Feb" out of an inexact set, and a typed
    // weekday is probably the user's intent regardless.
    return d;
  },
};

export const DATE_FORMATS: readonly DateFormatDef[] = [ISO, MEDIUM, SHORT];

const BY_ID = new Map<DateFormatId, DateFormatDef>(
  DATE_FORMATS.map((f) => [f.id, f]),
);

/// Look up a format by id. Falls back to ISO for unknown ids
/// (which can happen if an older client wrote a format we no
/// longer ship).
export function dateFormat(id: string): DateFormatDef {
  return BY_ID.get(id as DateFormatId) ?? ISO;
}

/// Format a date using the named format.
export function formatDate(d: Date, id: string): string {
  return dateFormat(id).format(d);
}

/// Parse a previously-formatted string back into a Date.
export function parseFormatted(s: string, id: string): Date | null {
  return dateFormat(id).parse(s);
}

/// Convert an ISO YYYY-MM-DD string to a Date (local time, midnight).
/// Returns null if the input doesn't match the ISO shape; this is the
/// canonical attribute-storage format on the date node.
export function dateFromIso(iso: string): Date | null {
  return ISO.parse(iso);
}

/// Format a Date as the canonical ISO YYYY-MM-DD attribute value.
export function isoOf(d: Date): string {
  return ISO.format(d);
}

export type DateMatch = {
  /// 0-indexed character offset of the match start within the
  /// scanned string.
  start: number;
  /// 0-indexed offset just past the match (slice-style).
  end: number;
  /// The matched substring.
  text: string;
  /// Format id whose pattern produced the match.
  formatId: DateFormatId;
  /// Parsed underlying date (year reattached for year-less formats).
  date: Date;
};

/// Find every date occurrence in `text` across every catalog format.
/// Matches are returned in document order; overlapping matches resolve
/// by taking the LONGEST match starting at the leftmost position
/// (so "2026-05-05" doesn't lose to a partial alternative).
///
/// Word boundaries: we anchor the regex on either side with a
/// non-alphanumeric / non-dash sentinel so partial typing
/// ("2026-05-0") and dates embedded in identifiers ("v2026-05-05.1")
/// don't false-positive. The trailing sentinel is required, which
/// is what defers detection while the user is still typing the
/// final character group.
export function findDateMatches(text: string): DateMatch[] {
  const out: DateMatch[] = [];
  // Combine all patterns into one global regex with named groups
  // so we know which format produced each hit.
  const parts = DATE_FORMATS.map((f, i) => `(?<g${i}>${f.pattern})`);
  // Sentinel: start of string OR a non-word, non-dash char before;
  // end of string OR a non-word, non-dash char after. Dash is in
  // the exclusion list so "2026-05-05" doesn't match the leading
  // "0" of a longer hyphenated token.
  const re = new RegExp(`(?:^|[^A-Za-z0-9-])(?:${parts.join("|")})(?=$|[^A-Za-z0-9-])`, "g");
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    // Find which alternative matched.
    let formatIdx = -1;
    for (let i = 0; i < DATE_FORMATS.length; i++) {
      if (m.groups && m.groups[`g${i}`] !== undefined) {
        formatIdx = i;
        break;
      }
    }
    if (formatIdx < 0) continue;
    const fmt = DATE_FORMATS[formatIdx];
    const matched = m.groups![`g${formatIdx}`];
    // The whole match m[0] may include a leading sentinel char
    // (when the date isn't at offset 0). Compute the date's own
    // offset as the position of the matched text within m[0].
    const offsetWithin = m[0].lastIndexOf(matched);
    const start = m.index + offsetWithin;
    const end = start + matched.length;
    const date = fmt.parse(matched);
    if (!date) continue;
    out.push({ start, end, text: matched, formatId: fmt.id, date });
  }
  return out;
}
