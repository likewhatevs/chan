// Rewrite a prompt draft's image refs from draft-file-relative to
// workspace-root-relative for terminal delivery.
//
// The Rich Prompt editor stores pasted images as `![](./image.png#w=N)`,
// relativized against the draft file (`.Drafts/{name}/draft.md`) so the editor
// preview resolves them. The receiving agent runs with the terminal's workspace
// root as its file base, so `./image.png` points at the wrong file. At submit we
// rewrite each image ref to the workspace-rooted path the agent can read, for
// example `.Drafts/{name}/image.png`, and drop the `#w=N` render hint. The
// draft text is unchanged; only the prompt frame payload is rewritten.

import { parseImageSrc } from "./extensions/image";
import { decodePercent, encodeRelPath, normalizeHref } from "./links";

const IMAGE_REF_RE = /!\[([^\]\n]*)\]\(([^)\n]*)\)/g;

/// Rewrite markdown image refs in `text` from paths relative to `fromPath` (the
/// draft file) to workspace-rooted paths the receiving agent can read.
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
