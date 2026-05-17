// Longest-common-prefix helper used by PathPromptModal's tab-
// completion. Lives outside the .svelte file so it can be unit-
// tested without mounting a component. Pure string logic; no DOM
// or store dependencies.

/// Return the longest common prefix shared by every string in
/// `paths`. Empty input → empty string. Single input → the input
/// itself. Used to grow the user's typed value as far as the
/// matching set allows without making a guess at branch points.
export function longestCommonPrefix(paths: readonly string[]): string {
  if (paths.length === 0) return "";
  let pre = paths[0]!;
  for (let i = 1; i < paths.length && pre.length > 0; i++) {
    const p = paths[i]!;
    let j = 0;
    const max = Math.min(pre.length, p.length);
    while (j < max && pre[j] === p[j]) j++;
    pre = pre.slice(0, j);
  }
  return pre;
}
