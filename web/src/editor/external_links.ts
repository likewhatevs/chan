import { syntaxTree } from "@codemirror/language";
import type { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

type SyntaxNode = ReturnType<ReturnType<typeof syntaxTree>["resolveInner"]>;

const OPENABLE_SCHEMES = new Set(["http", "https", "mailto", "tel"]);

type TauriWindow = Window &
  typeof globalThis & {
    __TAURI_INTERNALS__?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> };
    __TAURI__?: {
      invoke?: (cmd: string, args?: unknown) => Promise<unknown>;
      core?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> };
      opener?: { openUrl?: (url: string) => Promise<unknown> };
    };
  };

export function isOpenableExternalUrl(url: string): boolean {
  const scheme = /^([a-z][a-z0-9+.-]*):/i.exec(url.trim())?.[1]?.toLowerCase();
  return scheme ? OPENABLE_SCHEMES.has(scheme) : false;
}

export async function openExternalUrl(url: string): Promise<boolean> {
  if (!isOpenableExternalUrl(url)) return false;
  const w = window as TauriWindow;
  // Chan.app ships tauri-plugin-opener. That call happens in the
  // local desktop process, which keeps tunnel/remote chan-server
  // sessions opening links in the user's local OS browser.
  if (w.__TAURI__?.opener?.openUrl) {
    await w.__TAURI__.opener.openUrl(url);
    return true;
  }
  const invoke =
    w.__TAURI_INTERNALS__?.invoke ??
    w.__TAURI__?.core?.invoke ??
    w.__TAURI__?.invoke;
  if (invoke) {
    await invoke("plugin:opener|open_url", { url });
    return true;
  }
  window.open(url, "_blank", "noopener,noreferrer");
  return true;
}

export function externalUrlAtPos(state: EditorState, pos: number): string | null {
  for (const side of [-1, 0, 1] as const) {
    let node: SyntaxNode | null = syntaxTree(state).resolveInner(pos, side);
    while (node) {
      const url = externalUrlForNode(state, node);
      if (url) return url;
      node = node.parent;
    }
  }
  return null;
}

export function externalLinkClickHandler() {
  return EditorView.domEventHandlers({
    click(event, view) {
      const target = event.target as HTMLElement | null;
      if (!target?.closest(".cm-md-link, .cm-md-link-url")) return false;
      const pos = view.posAtCoords({ x: event.clientX, y: event.clientY });
      if (pos === null) return false;
      const url = externalUrlAtPos(view.state, pos);
      if (!url) return false;
      event.preventDefault();
      event.stopPropagation();
      void openExternalUrl(url);
      return true;
    },
  });
}

function externalUrlForNode(state: EditorState, node: SyntaxNode): string | null {
  if (node.name === "Link" || node.name === "Autolink") {
    return openableUrlFromChild(state, node);
  }
  if (node.name !== "URL") return null;
  const parent = node.parent;
  if (parent?.name === "Image") return null;
  if (parent?.name === "Link" || parent?.name === "Autolink") {
    return openableUrlFromChild(state, parent);
  }
  return openableUrl(state.doc.sliceString(node.from, node.to));
}

function openableUrlFromChild(state: EditorState, node: SyntaxNode): string | null {
  const cursor = node.cursor();
  if (!cursor.firstChild()) return null;
  do {
    if (cursor.name !== "URL") continue;
    const url = state.doc.sliceString(cursor.from, cursor.to);
    return openableUrl(url);
  } while (cursor.nextSibling());
  return null;
}

function openableUrl(url: string): string | null {
  const trimmed = url.trim();
  return isOpenableExternalUrl(trimmed) ? trimmed : null;
}
