// Rewrite a prompt draft's image refs from draft-file-relative to
// workspace-root-relative for AGENT DELIVERY.
//
// The rich-prompt composer inserts pasted images as `![](./image.png#w=N)`,
// relativized against the DRAFT file (`.Drafts/{name}/draft.md`) so the
// in-compose preview resolves them correctly through `resolveImageSrc`. But
// the receiving agent runs at `$CWD` = workspace root, so `./image.png`
// resolves to `<root>/image.png` and 404s. At submit we rewrite each image
// ref to the workspace-rooted path the agent can read directly (e.g.
// `.Drafts/{name}/image.png`), dropping the `#w=N` width fragment (a chan
// render hint the agent has no use for, and which would break a literal file
// read). The draft itself is untouched — only the delivered text changes, so
// the in-compose preview stays correct (two consumers, two bases).

import { parseImageSrc } from "./extensions/image";
import { decodePercent, encodeRelPath, normalizeHref } from "./links";

// `![alt](src)` on a single line — alt and src captured. Same image shape
// the editor's decorations match (`caret_mapping.ts` IMAGE_RE), so a ref the
// preview renders is a ref we rewrite.
const IMAGE_REF_RE = /!\[([^\]\n]*)\]\(([^)\n]*)\)/g;

/// Rewrite every markdown image ref in `text` from a path relative to
/// `fromPath` (the draft file) to a workspace-rooted path the receiving agent
/// (at `$CWD` = workspace root) can read. Mirrors `resolveImageSrc`'s
/// resolution (strip the `#w=N`/align fragment, percent-decode, `normalizeHref`
/// against the draft's directory) so the delivered path points at the SAME
/// file the preview shows, then re-encodes for a valid markdown destination.
/// External (`http`/`data`/`blob`) and unresolvable refs are left untouched.
export function rewriteImagePathsForDelivery(
  text: string,
  fromPath: string | null,
): string {
  if (!fromPath || !text.includes("![")) return text;
  const sourceDir = fromPath.split("/").slice(0, -1).join("/");
  return text.replace(IMAGE_REF_RE, (whole, alt: string, src: string) => {
    const { base } = parseImageSrc(src);
    if (!base || /^(https?:|data:|blob:)/i.test(base)) return whole;
    const rooted = normalizeHref(decodePercent(base), sourceDir);
    if (rooted == null) return whole;
    return `![${alt}](${encodeRelPath(rooted)})`;
  });
}
