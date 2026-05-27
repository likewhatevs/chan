// RFC 4180-ish CSV / TSV parser + serializer.
//
// Used by the CSV table renderer (#29) to turn a `.csv` / `.tsv`
// buffer into a 2D array of strings, edit individual cells, and
// re-emit the result back to text for `Workspace::write_text`. The
// parser is intentionally small and dependency-free; we don't need
// streaming or RFC-4180 strict mode for the editor's use case.
//
// Conventions:
//   - Records separated by `\n` (CRLF is normalized to LF before
//     emitting; reading accepts either).
//   - Fields separated by the configured delimiter (comma for
//     .csv, tab for .tsv).
//   - A field may be enclosed in `"`; inside, embedded delimiters,
//     newlines, and `""` (an escaped double-quote) are allowed.
//   - Unquoted fields run until the next delimiter or newline.
//   - Trailing newline is preserved on the output so a file that
//     ended in `\n` round-trips intact.

/// Parse `src` into a 2D array of strings using `delimiter` as the
/// field separator. Tolerant of `\r\n`, `\n`, and a trailing
/// newline. Returns an empty array for an empty input.
///
/// Malformed input (an opened quote that never closes, etc.) is
/// best-effort: the parser emits whatever it managed to collect
/// rather than throwing. Round-trip safety against handwritten
/// CSV is not a goal; the editor's source mode is the escape
/// hatch for genuinely broken files.
export function parseCsv(src: string, delimiter: string): string[][] {
  if (src.length === 0) return [];
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let inQuotes = false;
  let i = 0;
  while (i < src.length) {
    const ch = src[i];
    if (inQuotes) {
      if (ch === '"') {
        if (i + 1 < src.length && src[i + 1] === '"') {
          // Escaped double-quote inside a quoted field.
          field += '"';
          i += 2;
          continue;
        }
        // End of the quoted field. The next char is either the
        // delimiter, a newline, or end-of-string. Fall through.
        inQuotes = false;
        i++;
        continue;
      }
      field += ch;
      i++;
      continue;
    }
    if (ch === '"' && field === "") {
      // Opening quote at the start of a field. Quotes mid-field
      // are kept literal (matches Excel's permissive parse).
      inQuotes = true;
      i++;
      continue;
    }
    if (ch === delimiter) {
      row.push(field);
      field = "";
      i++;
      continue;
    }
    if (ch === "\r") {
      // CRLF or bare CR -> end-of-record. Swallow the LF if it
      // follows so a Windows-line-ending file doesn't produce a
      // phantom empty row.
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
      i++;
      if (i < src.length && src[i] === "\n") i++;
      continue;
    }
    if (ch === "\n") {
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
      i++;
      continue;
    }
    field += ch;
    i++;
  }
  // Flush the trailing field / row. An unterminated quote leaves
  // the partial content in `field`; we keep it rather than dropping
  // so the user can spot the mistake in source mode.
  if (field !== "" || row.length > 0) {
    row.push(field);
    rows.push(row);
  }
  return rows;
}

/// Re-emit `rows` as CSV / TSV text using `delimiter` as the
/// separator. Records joined with `\n`; trailing newline appended
/// so `parseCsv(serializeCsv(x)) === x` for clean inputs. A field
/// is quoted only when it contains the delimiter, a newline, or
/// a double-quote (matching common toolchain behavior so a diff
/// against the source stays minimal on cell-level edits).
export function serializeCsv(rows: string[][], delimiter: string): string {
  if (rows.length === 0) return "";
  const out: string[] = [];
  for (const row of rows) {
    out.push(row.map((f) => quoteIfNeeded(f, delimiter)).join(delimiter));
  }
  return `${out.join("\n")}\n`;
}

function quoteIfNeeded(field: string, delimiter: string): string {
  if (
    field.includes(delimiter)
    || field.includes('"')
    || field.includes("\n")
    || field.includes("\r")
  ) {
    return `"${field.replace(/"/g, '""')}"`;
  }
  return field;
}

/// Width of the widest row in `rows`. Used by the renderer to size
/// the header so a ragged file (rows with different field counts)
/// still aligns the columns it does have.
export function maxRowWidth(rows: string[][]): number {
  let w = 0;
  for (const row of rows) {
    if (row.length > w) w = row.length;
  }
  return w;
}
