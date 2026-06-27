// Rewrite a prompt draft's image refs into the plain on-disk paths the
// receiving agent reads.
//
// The rich-prompt composer inserts pasted images as `![](./image.png#w=N)`,
// relativized against the DRAFT file (`.Drafts/{name}/draft.md`) so the
// in-compose preview resolves them through `resolveImageSrc`. At submit we
// deliver a PLAIN path (no `![]()` wrapper, no `#w=N` render hint) computed
// for the terminal's live CWD: relative to that CWD when it is known and
// inside the workspace root, else an absolute on-disk path that always
// resolves. The draft itself is untouched -- only the delivered text
// changes, so the in-compose preview stays correct (two consumers, two
// bases).

import { parseImageSrc } from "./extensions/image";
import { decodePercent, normalizeHref, relativizePath } from "./links";

// `![alt](src)` on a single line. Same image shape the editor's
// decorations match (`caret_mapping.ts` IMAGE_RE), so a ref the preview
// renders is a ref we rewrite.
const IMAGE_REF_RE = /!\[([^\]\n]*)\]\(([^)\n]*)\)/g;

/// Rewrite every markdown image ref in `text` into the plain path the
/// receiving agent (running at the terminal's CWD) can read. Each ref is
/// resolved to its workspace-rooted target the same way `resolveImageSrc`
/// does (strip the `#w=N`/align fragment, percent-decode, `normalizeHref`
/// against the draft's directory), then mapped to a CWD-relative or
/// absolute path by `deliverPathFor`. External (`http`/`data`/`blob`) and
/// unresolvable refs are left untouched, wrapper and all.
export function rewriteImagePathsForDelivery(
  text: string,
  fromPath: string | null,
  terminalCwdRel: string | null,
  workspaceRoot: string | null,
): string {
  if (!fromPath || !text.includes("![")) return text;
  const sourceDir = fromPath.split("/").slice(0, -1).join("/");
  return text.replace(IMAGE_REF_RE, (whole, _alt: string, src: string) => {
    const { base } = parseImageSrc(src);
    if (!base || /^(https?:|data:|blob:)/i.test(base)) return whole;
    const rooted = normalizeHref(decodePercent(base), sourceDir);
    if (rooted == null) return whole;
    return deliverPathFor(rooted, terminalCwdRel, workspaceRoot);
  });
}

/// Map a workspace-rooted image path to the plain path the agent reads.
/// The agent runs at the terminal's CWD, so a CWD-relative path resolves
/// directly; when the CWD is unknown or sits outside the workspace root
/// (`terminalCwdRel` null) fall back to an absolute on-disk path, which
/// always resolves. `terminalCwdRel` is the CWD relative to the workspace
/// root, or "" when the CWD IS the root.
function deliverPathFor(
  rooted: string,
  terminalCwdRel: string | null,
  workspaceRoot: string | null,
): string {
  if (terminalCwdRel !== null) {
    // relativizePath works from a file path's PARENT dir; hand it a dummy
    // file inside the CWD so the result is rooted at the CWD.
    const cwdFile = terminalCwdRel === "" ? "_" : `${terminalCwdRel}/_`;
    return relativizePath(rooted, cwdFile);
  }
  if (workspaceRoot) {
    const root = workspaceRoot.replace(/\\/g, "/").replace(/\/+$/, "");
    return `${root}/${rooted}`;
  }
  return rooted;
}
