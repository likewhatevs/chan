// Single-character avatar fallback. Used by the topbar and any view
// that wants a placeholder when no provider picture is available.

export function initial(
  who: { display_name?: string | null; email?: string | null },
): string {
  const src = (who.display_name ?? who.email ?? "?").trim();
  return src.charAt(0).toUpperCase() || "?";
}
