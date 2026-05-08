// Wiki-link serialization and relative-path helpers used by the
// editor. The wikiLink TipTap extension serializes atoms back to
// markdown via `wikiLinkToMarkdown`; Wysiwyg resolves clicks on
// relative-path links via `resolveRelativePath`.

/// Serialize a wikiLink atom's attrs back to markdown.
///
/// `fromPath` is the path of the file whose markdown is being
/// produced (drive-rooted POSIX, no leading slash). When provided,
/// the URL portion is rewritten to a file-relative path with an
/// explicit `./` or `../` prefix so the discriminator at parse
/// time can tell relative URLs from legacy drive-rooted ones.
/// When omitted (e.g. assistant prompt context, no source file),
/// the URL stays drive-rooted.
export function wikiLinkToMarkdown(
  target: string,
  label?: string,
  anchor?: string,
  fromPath?: string,
): string {
  const stem = (label ?? target.split("/").pop() ?? target).replace(/\.md$/, "");
  // Build the URL portion. With `fromPath` set, the URL is
  // rewritten to a file-relative path so notes stay portable
  // across project layouts (an editor opening a single file
  // outside the drive can still resolve the link). Without
  // `fromPath`, fall back to the legacy drive-rooted form so
  // the assistant prompt and other no-source-file callers keep
  // their existing semantics.
  const path = fromPath ? relativizePath(target, fromPath) : target;
  const enc = path
    .split("/")
    .map((s) => encodeURIComponent(s).replace(/%2F/g, "/"))
    .join("/");
  // Anchor is appended verbatim. Heading anchors are already
  // slugged by chan-core (kebab-case ASCII); block anchors are
  // `^id` and round-trip cleanly through encodeURIComponent's
  // identity for `^`.
  const frag = anchor ? `#${anchor}` : "";
  return `[${stem}](${enc}${frag})`;
}

/// Compute a file-relative path from `fromPath`'s directory to
/// `target`, both drive-rooted POSIX paths. Always emits a
/// `./` or `../` prefix so the parser can distinguish a relative
/// URL from a legacy drive-rooted one.
///
/// Examples (fromPath -> target -> result):
///   `Recipes/Pasta.md`    -> `Recipes/Brazilian Rice.md` -> `./Brazilian Rice.md`
///   `Recipes/Pasta.md`    -> `Notes/Foo.md`              -> `../Notes/Foo.md`
///   `README.md`           -> `Recipes/Pasta.md`          -> `./Recipes/Pasta.md`
export function relativizePath(target: string, fromPath: string): string {
  const fromDir = fromPath.split("/").slice(0, -1);
  const tgtParts = target.split("/");
  let i = 0;
  while (
    i < fromDir.length &&
    i < tgtParts.length - 1 &&
    fromDir[i] === tgtParts[i]
  ) {
    i += 1;
  }
  const ups = fromDir.length - i;
  const down = tgtParts.slice(i);
  if (ups === 0) {
    return ["."].concat(down).join("/");
  }
  return Array(ups).fill("..").concat(down).join("/");
}

/// Resolve a relative href against `fromPath`'s directory, returning
/// the canonical drive-rooted target. Hrefs that don't start with
/// `./` or `../` are treated as already-drive-rooted (legacy /
/// power-user form) and returned unchanged.
export function resolveRelativePath(href: string, fromPath: string): string {
  if (!href.startsWith("./") && !href.startsWith("../")) {
    return href;
  }
  const fromDir = fromPath.split("/").slice(0, -1);
  const parts = href.split("/");
  for (const p of parts) {
    if (p === "" || p === ".") continue;
    if (p === "..") {
      if (fromDir.length > 0) fromDir.pop();
    } else {
      fromDir.push(p);
    }
  }
  return fromDir.join("/");
}
