// Fullscreen slide preview. Mirrors the image/diagram viewer chrome:
// theme-aware backdrop, centered content surface, prev/next controls,
// and Escape cleanup. The caller owns the editor read-only state via
// the returned close handle and onClose callback.

import { isTauriDesktop, setWindowFullscreen } from "../api/desktop";
import { renderMarkdown } from "../api/markdown";
import { type DiagramResult } from "../editor/diagram_render";
import {
  isExcalidrawImageSrc,
  parseImageSrc,
  resolveImageSrc,
} from "../editor/extensions/image";
import {
  renderExcalidraw,
  renderExcalidrawFile,
} from "../editor/excalidraw_render";
import { renderMermaid } from "../editor/mermaid_render";
import {
  parseSlidesSpec,
  slideIndexForLine,
  splitSlidePages,
  type SlideAspectRatio,
  type SlidePage,
} from "../editor/slides";

export type SlidePreviewTheme = "light" | "dark";
export type SlidePreviewMode = "preview" | "play";

export type OpenSlidePreviewOptions = {
  source: string;
  currentLine: number | null;
  initialIndex?: number | null;
  fromPath: string | null;
  styleSource?: Element | null;
  theme: SlidePreviewTheme;
  mode?: SlidePreviewMode;
  onSlideChange?: (index: number) => void;
  onClose?: () => void;
};

export type SlidePreviewUpdate = Partial<
  Pick<
    OpenSlidePreviewOptions,
    "source" | "fromPath" | "initialIndex" | "styleSource" | "theme" | "mode"
  >
>;

export type SlidePreviewHandle = {
  update: (opts: SlidePreviewUpdate) => void;
  close: (opts?: { notify?: boolean }) => void;
};

type SlidePreviewState = {
  pages: SlidePage[];
  aspectRatio: SlideAspectRatio;
  zoomFactor: number;
  index: number;
  source: string;
  fromPath: string | null;
  styleSource: Element | null | undefined;
  theme: SlidePreviewTheme;
  mode: SlidePreviewMode;
};

type SlideMediaAlign = "left" | "center" | "right";
type SlideDiagramKind = "mermaid" | "excalidraw";
type SlideDiagramSpec = {
  kind: SlideDiagramKind;
  align: SlideMediaAlign;
};

const BLANK_LINE_SPACER =
  '<div class="chan-slide-blank-line" aria-hidden="true"></div>';

const EDITOR_VARS = [
  "--chan-editor-heading-family",
  "--chan-editor-heading-color",
  "--chan-editor-h1-size",
  "--chan-editor-h1-weight",
  "--chan-editor-h1-line-height",
  "--chan-editor-h1-border-bottom",
  "--chan-editor-h1-padding-bottom",
  "--chan-editor-h2-size",
  "--chan-editor-h2-weight",
  "--chan-editor-h2-line-height",
  "--chan-editor-h3-size",
  "--chan-editor-h3-weight",
  "--chan-editor-h3-line-height",
  "--chan-editor-h4-size",
  "--chan-editor-h4-weight",
  "--chan-editor-h4-line-height",
  "--chan-editor-h5-size",
  "--chan-editor-h5-weight",
  "--chan-editor-h5-line-height",
  "--chan-editor-h6-size",
  "--chan-editor-h6-weight",
  "--chan-editor-h6-line-height",
  "--chan-editor-h6-color",
  "--chan-editor-code-family",
  "--chan-editor-code-size",
  "--chan-editor-inline-code-bg",
  "--chan-editor-inline-code-color",
  "--chan-editor-code-block-bg",
  "--chan-editor-code-block-color",
  "--chan-editor-code-block-border",
  "--chan-editor-link-color",
  "--chan-editor-quote-color",
  "--chan-editor-quote-border",
  "--chan-editor-hr-color",
  "--chan-editor-table-border",
  "--chan-editor-table-header-bg",
  "--chan-editor-table-stripe-bg",
] as const;

export function openSlidePreview(opts: OpenSlidePreviewOptions): SlidePreviewHandle | null {
  const spec = parseSlidesSpec(opts.source);
  if (!spec) return null;
  const pages = splitSlidePages(opts.source);
  if (pages.length === 0) return null;

  const state: SlidePreviewState = {
    pages,
    aspectRatio: spec.aspectRatio,
    zoomFactor: spec.zoomFactor,
    index: initialSlideIndex(pages, opts.initialIndex, opts.currentLine),
    source: opts.source,
    fromPath: opts.fromPath,
    styleSource: opts.styleSource,
    theme: opts.theme,
    mode: opts.mode ?? "preview",
  };

  const previouslyFocused =
    document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const backdrop = document.createElement("div");
  backdrop.className = "md-image-zoom md-slide-preview";
  backdrop.tabIndex = -1;
  backdrop.setAttribute("role", "dialog");
  backdrop.setAttribute("aria-modal", "true");
  backdrop.setAttribute("aria-label", "Slide preview");

  const style = document.createElement("style");
  style.textContent = slidePreviewCss();
  backdrop.appendChild(style);

  const page = document.createElement("article");
  page.className = "md-slide-preview-page";
  page.addEventListener("click", (event) => event.stopPropagation());

  const content = document.createElement("div");
  content.className = "md-slide-preview-content";
  page.appendChild(content);
  backdrop.appendChild(page);

  const prev = navButton("prev", "‹", "Previous slide", () => step(-1));
  const next = navButton("next", "›", "Next slide", () => step(1));
  const counter = document.createElement("div");
  counter.className = "md-image-zoom-counter md-slide-preview-counter";
  backdrop.append(prev, next, counter);
  let diagramRenderRun = 0;

  function applyTheme(): void {
    backdrop.dataset.theme = state.theme;
    backdrop.dataset.mode = state.mode;
    backdrop.setAttribute(
      "aria-label",
      state.mode === "play" ? "Slide player" : "Slide preview",
    );
    backdrop.style.cssText = backdropStyle(state.theme);
    page.style.cssText = pageStyle(
      state.aspectRatio,
      state.styleSource,
      state.theme,
      state.mode,
    );
    content.style.cssText = contentStyle(state.zoomFactor);
    content.dataset.zoomFactor = cssNumber(state.zoomFactor);
    applyNavTheme(prev, state.theme, prev.disabled);
    applyNavTheme(next, state.theme, next.disabled);
    counter.style.cssText = counterStyle(state.theme);
    applyChromeMode(prev, next, counter, state.mode);
  }

  function show(): void {
    const current = state.pages[state.index]!;
    const renderRun = ++diagramRenderRun;
    content.innerHTML = renderSlideMarkdown(current.markdown);
    prepareSlideImages(content, state.fromPath, state.theme, () => {
      return !closed && renderRun === diagramRenderRun;
    });
    renderSlideDiagrams(content, current.markdown, state.theme, () => {
      return !closed && renderRun === diagramRenderRun;
    });
    page.setAttribute("aria-label", `Slide ${current.number}`);
    counter.textContent =
      state.mode === "preview" ? `${state.index + 1} / ${state.pages.length}` : "";
    syncNavButton(prev, state.index === 0);
    syncNavButton(next, state.index === state.pages.length - 1);
    applyChromeMode(prev, next, counter, state.mode);
  }

  function step(delta: number): void {
    const nextIndex = state.index + delta;
    if (nextIndex < 0 || nextIndex >= state.pages.length) return;
    state.index = nextIndex;
    show();
    opts.onSlideChange?.(state.index);
  }

  function update(next: SlidePreviewUpdate): void {
    let needsShow = false;
    const previousIndex = state.index;
    if (typeof next.source === "string" && next.source !== state.source) {
      const spec = parseSlidesSpec(next.source);
      if (!spec) return;
      state.source = next.source;
      state.aspectRatio = spec.aspectRatio;
      state.zoomFactor = spec.zoomFactor;
      state.pages = splitSlidePages(next.source);
      state.index = clampSlideIndex(state.pages, state.index);
      needsShow = true;
    }
    if (next.fromPath !== undefined) state.fromPath = next.fromPath;
    if (next.styleSource !== undefined) state.styleSource = next.styleSource;
    if (next.theme && next.theme !== state.theme) {
      state.theme = next.theme;
      needsShow = true;
    }
    if (next.mode && next.mode !== state.mode) {
      state.mode = next.mode;
      needsShow = true;
    }
    if (next.initialIndex !== undefined && next.initialIndex !== null) {
      const idx = clampSlideIndex(state.pages, next.initialIndex);
      if (idx !== state.index) {
        state.index = idx;
        needsShow = true;
      }
    }
    applyTheme();
    if (needsShow) show();
    if (next.mode === "play") requestSlideFullscreen(backdrop);
    else if (next.mode === "preview") exitSlideFullscreen(backdrop);
    if (state.index !== previousIndex) opts.onSlideChange?.(state.index);
  }

  let closed = false;
  const dismiss = ({ notify = true }: { notify?: boolean } = {}): void => {
    if (closed) return;
    closed = true;
    document.removeEventListener("keydown", onKey, true);
    exitSlideFullscreen(backdrop);
    backdrop.remove();
    if (notify) opts.onClose?.();
    previouslyFocused?.focus({ preventScroll: true });
  };

  const onKey = (event: KeyboardEvent): void => {
    switch (event.key) {
      case "Escape":
        consume(event);
        dismiss();
        break;
      case "Backspace":
      case "ArrowUp":
      case "ArrowLeft":
        consume(event);
        step(-1);
        break;
      case " ":
      case "Spacebar":
      case "ArrowDown":
      case "ArrowRight":
        consume(event);
        step(1);
        break;
    }
  };

  backdrop.addEventListener("click", () => dismiss());
  document.addEventListener("keydown", onKey, true);
  applyTheme();
  show();
  opts.onSlideChange?.(state.index);
  document.body.appendChild(backdrop);
  if (state.mode === "play") requestSlideFullscreen(backdrop);
  backdrop.focus({ preventScroll: true });
  return { update, close: dismiss };
}

function renderSlideMarkdown(markdown: string): string {
  return renderMarkdown(preserveExtraBlankLines(markdown));
}

function preserveExtraBlankLines(markdown: string): string {
  const lines = markdown.split("\n");
  const out: string[] = [];
  let blankRun = 0;
  let inFence = false;
  let fenceMarker: "`" | "~" | null = null;

  function flushBlankRun(): void {
    if (blankRun === 0) return;
    out.push("");
    for (let i = 1; i < blankRun; i++) out.push(BLANK_LINE_SPACER);
    if (blankRun > 1) out.push("");
    blankRun = 0;
  }

  for (const line of lines) {
    const fence = line.match(/^\s*(`{3,}|~{3,})/);
    if (!inFence && line.trim() === "") {
      blankRun++;
      continue;
    }

    flushBlankRun();
    out.push(line);

    if (!fence) continue;
    const marker = fence[1]![0] as "`" | "~";
    if (!inFence) {
      inFence = true;
      fenceMarker = marker;
    } else if (fenceMarker === marker) {
      inFence = false;
      fenceMarker = null;
    }
  }

  flushBlankRun();
  return out.join("\n");
}

function renderSlideDiagrams(
  root: ParentNode,
  markdown: string,
  theme: SlidePreviewTheme,
  isCurrent: () => boolean,
): void {
  const specs = slideDiagramSpecs(markdown);
  let specIndex = 0;
  for (const code of Array.from(root.querySelectorAll("pre > code"))) {
    const kind = slideDiagramKind(code);
    if (!kind) continue;
    const align = specs[specIndex]?.align ?? "center";
    specIndex++;
    const pre = code.parentElement;
    if (!(pre instanceof HTMLElement)) continue;

    const source = code.textContent ?? "";
    const shell = document.createElement("div");
    shell.className = "md-slide-diagram";
    applySlideAlignClass(shell, align);
    const body = document.createElement("div");
    body.className = "md-slide-diagram-body";
    body.textContent = "rendering...";
    shell.append(body);
    pre.replaceWith(shell);

    const render =
      kind === "excalidraw" ? renderExcalidraw : renderMermaid;
    const label = kind === "excalidraw" ? "Excalidraw" : "Mermaid";
    void render(source, theme === "dark").then((res) => {
      if (!isCurrent()) return;
      if (res.ok && res.svg) {
        body.classList.remove("md-slide-diagram-error");
        body.innerHTML = res.svg;
      } else {
        renderSlideDiagramError(body, source, res, label);
      }
    });
  }
}

function slideDiagramKind(code: Element): "mermaid" | "excalidraw" | null {
  if (code.classList.contains("language-mermaid-to-excalidraw")) {
    return "excalidraw";
  }
  if (code.classList.contains("language-mermaid")) return "mermaid";
  return null;
}

function slideDiagramSpecs(markdown: string): SlideDiagramSpec[] {
  const specs: SlideDiagramSpec[] = [];
  const lines = markdown.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const open = lines[i]?.match(/^\s*(`{3,}|~{3,})\s*([^\r\n]*)$/);
    if (!open) continue;

    const marker = open[1]!;
    const info = open[2] ?? "";
    const spec = slideDiagramSpecFromInfo(info);
    if (spec) specs.push(spec);

    const close = new RegExp(
      `^\\s*${escapeRegExp(marker[0]!)}{${marker.length},}\\s*$`,
    );
    for (i = i + 1; i < lines.length; i++) {
      if (close.test(lines[i] ?? "")) break;
    }
  }

  return specs;
}

function slideDiagramSpecFromInfo(info: string): SlideDiagramSpec | null {
  const parts = info.trim().toLowerCase().split(/\s+/).filter(Boolean);
  const lang = parts[0];
  const kind =
    lang === "mermaid"
      ? "mermaid"
      : lang === "mermaid-to-excalidraw"
        ? "excalidraw"
        : null;
  if (!kind) return null;

  return {
    kind,
    align: slideAlignFromTokens(parts.slice(1)) ?? "center",
  };
}

function slideAlignFromTokens(tokens: readonly string[]): SlideMediaAlign | null {
  for (const token of tokens) {
    const normalized = token.replace(/^[{#]+|[},]+$/g, "");
    if (
      normalized === "left" ||
      normalized === "center" ||
      normalized === "right"
    ) {
      return normalized;
    }
    const align = normalized.match(/^align[:=](left|center|right)$/)?.[1];
    if (align === "left" || align === "center" || align === "right") {
      return align;
    }
  }
  return null;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function renderSlideDiagramError(
  body: HTMLElement,
  source: string,
  res: DiagramResult,
  label: "Mermaid" | "Excalidraw",
): void {
  body.classList.add("md-slide-diagram-error");
  body.replaceChildren();
  if (res.errorLine) {
    const head = document.createElement("div");
    head.className = "md-slide-diagram-error-head";
    head.textContent = `${label} error - line ${res.errorLine}`;
    body.append(head);
    const lineText = source.split("\n")[res.errorLine - 1];
    if (lineText !== undefined) {
      const code = document.createElement("div");
      code.className = "md-slide-diagram-error-src";
      code.textContent = lineText;
      body.append(code);
    }
  }
  const reason = document.createElement("div");
  reason.textContent = res.error ?? "render failed";
  body.append(reason);
}

function consume(event: KeyboardEvent): void {
  event.preventDefault();
  event.stopPropagation();
}

function navButton(
  kind: "prev" | "next",
  glyph: string,
  label: string,
  onClick: () => void,
): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = `md-image-zoom-nav md-image-zoom-${kind} md-slide-preview-${kind}`;
  btn.setAttribute("aria-label", label);
  btn.textContent = glyph;
  const side = kind === "prev" ? "left:12px;" : "right:12px;";
  btn.style.cssText =
    "position:fixed;top:50%;transform:translateY(-50%);" +
    side +
    "width:48px;height:64px;border:none;border-radius:8px;" +
    "background:rgba(0,0,0,0.45);color:#fff;cursor:pointer;" +
    "font-size:34px;line-height:1;display:flex;align-items:center;" +
    "justify-content:center;";
  btn.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    onClick();
  });
  return btn;
}

function syncNavButton(btn: HTMLButtonElement, disabled: boolean): void {
  btn.disabled = disabled;
  applyNavTheme(
    btn,
    (btn.closest(".md-slide-preview") as HTMLElement | null)?.dataset.theme === "light"
      ? "light"
      : "dark",
    disabled,
  );
}

function applyChromeMode(
  prev: HTMLButtonElement,
  next: HTMLButtonElement,
  counter: HTMLElement,
  mode: SlidePreviewMode,
): void {
  const hidden = mode === "play";
  for (const el of [prev, next, counter]) {
    el.hidden = hidden;
    el.setAttribute("aria-hidden", hidden ? "true" : "false");
    el.style.display = hidden ? "none" : "";
  }
}

function requestSlideFullscreen(el: HTMLElement): void {
  if (isTauriDesktop()) {
    // chan-desktop's WKWebView disables the HTML element Fullscreen API, so
    // "play" drives the native window fullscreen through Tauri instead. Unlike
    // element.requestFullscreen() this needs no user activation, so every play
    // transition (open, mode change, restore) goes fullscreen deterministically.
    void setWindowFullscreen(true);
    return;
  }
  const request = el.requestFullscreen?.bind(el);
  if (!request) return;
  try {
    void request().catch(() => {});
  } catch {
    // Browsers can reject fullscreen when Play is restored without a
    // user activation. The slide player still opens in-window.
  }
}

function exitSlideFullscreen(el: HTMLElement): void {
  if (isTauriDesktop()) {
    void setWindowFullscreen(false);
    return;
  }
  if (document.fullscreenElement !== el) return;
  const exit = document.exitFullscreen?.bind(document);
  if (!exit) return;
  try {
    void exit().catch(() => {});
  } catch {
    // Best effort cleanup; removing the fullscreen element also exits.
  }
}

function pageStyle(
  aspectRatio: SlideAspectRatio,
  styleSource: Element | null | undefined,
  theme: SlidePreviewTheme,
  mode: SlidePreviewMode,
): string {
  const [w, h] = aspectRatio.split(":").map((part) => Number(part));
  const ratio = w! / h!;
  const previewWidthLimit = `${Math.round(86 * ratio * 100) / 100}vh`;
  const playWidthLimit = `${cssNumber(100 * ratio)}vh`;
  const isPlay = mode === "play";
  const tokens = editorTokens(styleSource, theme);

  return [
    tokens.vars,
    "box-sizing:border-box",
    `color-scheme:${theme}`,
    isPlay ? "width:100vw" : `width:min(86vw,${previewWidthLimit})`,
    ...(isPlay ? [`max-width:${playWidthLimit}`] : []),
    `aspect-ratio:${w} / ${h}`,
    isPlay ? "max-height:100vh" : "max-height:86vh",
    "overflow:auto",
    "padding:clamp(22px,4vw,54px)",
    `background:${tokens.bg}`,
    `color:${tokens.fg}`,
    `font-family:${tokens.bodyFamily}`,
    `font-size:${tokens.bodySize}`,
    "line-height:1.55",
    isPlay
      ? "box-shadow:none"
      : `box-shadow:${theme === "dark" ? "0 10px 48px rgba(0,0,0,0.55)" : "0 16px 56px rgba(31,41,55,0.22)"}`,
    `border-radius:${isPlay ? "0" : "4px"}`,
    "cursor:auto",
  ].join(";");
}

function contentStyle(zoomFactor: number): string {
  const zoom = positiveZoomFactor(zoomFactor);
  return [
    "display:block",
    `zoom:${cssNumber(zoom)}`,
    "width:100%",
    "min-height:100%",
    "transform-origin:top left",
  ].join(";");
}

function positiveZoomFactor(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 2;
}

function cssNumber(value: number): string {
  return String(Math.round(value * 100) / 100);
}

function backdropStyle(theme: SlidePreviewTheme): string {
  // Fully opaque: a translucent backdrop let the editor's own layered
  // backgrounds (the dark tab bar over the light content, the outline/pane
  // divider) bleed through at different shades, so the presenter surface showed
  // a two-tone horizontal/vertical seam. A solid fill reads as one clean stage.
  const bg = theme === "dark" ? "rgb(0,0,0)" : "rgb(238,241,245)";
  return (
    "position:fixed;inset:0;z-index:40000;" +
    `background:${bg};display:flex;align-items:center;` +
    "justify-content:center;cursor:zoom-out;overflow:hidden;"
  );
}

function counterStyle(theme: SlidePreviewTheme): string {
  return (
    "position:fixed;bottom:18px;left:50%;transform:translateX(-50%);" +
    "font:13px/1.4 ui-monospace,Menlo,monospace;" +
    "padding:2px 10px;border-radius:10px;pointer-events:none;" +
    (theme === "dark"
      ? "color:#ddd;background:rgba(0,0,0,0.5);"
      : "color:#1f2937;background:rgba(255,255,255,0.86);box-shadow:0 2px 10px rgba(31,41,55,0.12);")
  );
}

function applyNavTheme(
  btn: HTMLButtonElement,
  theme: SlidePreviewTheme,
  disabled: boolean,
): void {
  const kind = btn.classList.contains("md-slide-preview-prev") ? "prev" : "next";
  const side = kind === "prev" ? "left:12px;" : "right:12px;";
  btn.style.cssText =
    "position:fixed;top:50%;transform:translateY(-50%);" +
    side +
    "width:48px;height:64px;border-radius:8px;cursor:pointer;" +
    "font-size:34px;line-height:1;display:flex;align-items:center;" +
    "justify-content:center;" +
    (theme === "dark"
      ? "border:none;background:rgba(0,0,0,0.45);color:#fff;"
      : "border:1px solid rgba(31,41,55,0.16);background:rgba(255,255,255,0.88);color:#111827;box-shadow:0 6px 24px rgba(31,41,55,0.16);") +
    `opacity:${disabled ? "0.35" : "1"};cursor:${disabled ? "default" : "pointer"};`;
}

function initialSlideIndex(
  pages: readonly Pick<SlidePage, "startLine">[],
  initialIndex: number | null | undefined,
  currentLine: number | null,
): number {
  if (initialIndex !== undefined && initialIndex !== null) {
    return clampSlideIndex(pages, initialIndex);
  }
  return slideIndexForLine(pages, currentLine);
}

function clampSlideIndex(pages: readonly unknown[], index: number): number {
  if (pages.length === 0) return 0;
  if (!Number.isFinite(index)) return 0;
  return Math.max(0, Math.min(pages.length - 1, Math.floor(index)));
}

function editorTokens(
  styleSource: Element | null | undefined,
  theme: SlidePreviewTheme,
): {
  bg: string;
  fg: string;
  bodyFamily: string;
  bodySize: string;
  vars: string;
} {
  const host = styleSource ?? document.body ?? document.documentElement;
  const probe = document.createElement("div");
  probe.dataset.theme = theme;
  probe.style.cssText =
    "position:fixed;left:-9999px;top:-9999px;width:0;height:0;" +
    "overflow:hidden;visibility:hidden;pointer-events:none;" +
    "background:var(--chan-editor-bg);" +
    "color:var(--chan-editor-body-color);" +
    "font-family:var(--chan-editor-body-family);" +
    "font-size:var(--chan-editor-body-size);";
  host.appendChild(probe);
  const style = getComputedStyle(probe);
  const vars = EDITOR_VARS.map((name) => {
    const value = style.getPropertyValue(name).trim();
    return value ? `${name}:${value};` : "";
  })
    .filter(Boolean)
    .join("");
  const tokens = {
    bg: style.backgroundColor || (theme === "dark" ? "#111827" : "#fff"),
    fg: style.color || (theme === "dark" ? "#f3f4f6" : "#1f2328"),
    bodyFamily:
      style.fontFamily || "-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif",
    bodySize: style.fontSize || "16px",
    vars,
  };
  probe.remove();
  return tokens;
}

function prepareSlideImages(
  root: ParentNode,
  fromPath: string | null,
  theme: SlidePreviewTheme,
  isCurrent: () => boolean,
): void {
  for (const img of Array.from(root.querySelectorAll("img"))) {
    const raw = img.getAttribute("src") ?? "";
    const { width, align } = parseImageSrc(raw);
    if (isExcalidrawImageSrc(raw)) {
      renderSlideExcalidraw(img, raw, width, align, fromPath, theme, isCurrent);
      continue;
    }
    const resolved = resolveImageSrc(raw, fromPath);
    if (resolved) img.setAttribute("src", resolved);
    if (width != null) img.style.width = `${width}px`;
    applySlideMediaAlignment(img, align ?? "center");
  }
  for (const link of Array.from(root.querySelectorAll("a"))) {
    link.setAttribute("target", "_blank");
    link.setAttribute("rel", "noreferrer");
  }
}

function renderSlideExcalidraw(
  img: HTMLImageElement,
  raw: string,
  width: number | null,
  align: "left" | "right" | null,
  fromPath: string | null,
  theme: SlidePreviewTheme,
  isCurrent: () => boolean,
): void {
  const shell = document.createElement("div");
  shell.className = "md-slide-excalidraw";
  applySlideAlignClass(shell, align ?? "center");
  const body = document.createElement("div");
  body.className = "md-slide-excalidraw-body";
  body.textContent = "rendering...";
  if (width != null) body.style.width = `${width}px`;
  shell.append(body);
  img.replaceWith(shell);
  applyStandaloneMediaParentAlignment(shell, align ?? "center");

  const resolved = resolveImageSrc(raw, fromPath);
  if (!resolved) {
    renderSlideExcalidrawError(body, "cannot resolve Excalidraw file");
    return;
  }
  void renderExcalidrawFile(resolved, theme === "dark").then((res) => {
    if (!isCurrent()) return;
    if (res.ok && res.svg) {
      body.classList.remove("md-slide-excalidraw-error");
      body.innerHTML = res.svg;
    } else {
      renderSlideExcalidrawError(body, res.error ?? "render failed");
    }
  });
}

function renderSlideExcalidrawError(body: HTMLElement, message: string): void {
  body.classList.add("md-slide-excalidraw-error");
  body.textContent = `Excalidraw render failed: ${message}`;
}

function applySlideMediaAlignment(el: HTMLElement, align: SlideMediaAlign): void {
  applySlideAlignClass(el, align);
  applyStandaloneMediaParentAlignment(el, align);
}

function applySlideAlignClass(el: HTMLElement, align: SlideMediaAlign): void {
  el.classList.add(`chan-slide-align-${align}`);
}

function applyStandaloneMediaParentAlignment(
  el: HTMLElement,
  align: SlideMediaAlign,
): void {
  const parent = el.parentElement;
  if (!parent || parent.tagName !== "P") return;
  if (!isOnlySignificantChild(parent, el)) return;
  parent.classList.add("chan-slide-media");
  applySlideAlignClass(parent, align);
}

function isOnlySignificantChild(parent: HTMLElement, child: Node): boolean {
  return Array.from(parent.childNodes).every((node) => {
    return (
      node === child ||
      (node.nodeType === Node.TEXT_NODE && !node.textContent?.trim())
    );
  });
}

function slidePreviewCss(): string {
  return `
.md-slide-preview-content {
  min-width: 0;
}
.md-slide-preview-page h1,
.md-slide-preview-page h2,
.md-slide-preview-page h3,
.md-slide-preview-page h4,
.md-slide-preview-page h5,
.md-slide-preview-page h6 {
  color: var(--chan-editor-heading-color, currentColor);
  font-family: var(--chan-editor-heading-family, var(--chan-editor-body-family, inherit));
  line-height: 1.2;
  margin: 0 0 0.55em;
}
.md-slide-preview-page h1 {
  font-size: var(--chan-editor-h1-size, 2em);
  font-weight: var(--chan-editor-h1-weight, 700);
  border-bottom: var(--chan-editor-h1-border-bottom, none);
  padding-bottom: var(--chan-editor-h1-padding-bottom, 0);
}
.md-slide-preview-page h2 {
  font-size: var(--chan-editor-h2-size, 1.55em);
  font-weight: var(--chan-editor-h2-weight, 650);
}
.md-slide-preview-page h3 { font-size: var(--chan-editor-h3-size, 1.25em); }
.md-slide-preview-page p,
.md-slide-preview-page ul,
.md-slide-preview-page ol,
.md-slide-preview-page blockquote,
.md-slide-preview-page pre,
.md-slide-preview-page table {
  margin: 0 0 0.8em;
}
.md-slide-preview-content > :last-child { margin-bottom: 0; }
.md-slide-preview-page .chan-slide-blank-line {
  height: 1.55em;
}
.md-slide-preview-page .chan-slide-media {
  display: flex;
  margin: 0 0 0.8em;
}
.md-slide-preview-page .chan-slide-align-left { justify-content: flex-start; }
.md-slide-preview-page .chan-slide-align-center { justify-content: center; }
.md-slide-preview-page .chan-slide-align-right { justify-content: flex-end; }
.md-slide-preview-page img {
  display: block;
  max-width: 100%;
  height: auto;
  margin: 0.7em auto;
}
.md-slide-preview-page .chan-slide-media > img { margin: 0.7em 0; }
.md-slide-preview-page img.chan-slide-align-left { margin-left: 0; margin-right: auto; }
.md-slide-preview-page img.chan-slide-align-right { margin-left: auto; margin-right: 0; }
.md-slide-preview-page pre {
  overflow: auto;
  padding: 0.7em 0.8em;
  border-radius: 6px;
  background: var(--chan-editor-code-block-bg, rgba(127,127,127,0.12));
  color: var(--chan-editor-code-block-color, inherit);
}
.md-slide-preview-page code {
  font-family: var(--chan-editor-code-family, ui-monospace, SFMono-Regular, Menlo, monospace);
  font-size: var(--chan-editor-code-size, 0.92em);
  background: var(--chan-editor-inline-code-bg, rgba(127,127,127,0.12));
  color: var(--chan-editor-inline-code-color, inherit);
}
.md-slide-preview-page pre code { background: transparent; color: inherit; }
.md-slide-preview-page .md-slide-diagram {
  margin: 0.7em 0 0.9em;
}
.md-slide-preview-page .md-slide-diagram-body {
  display: flex;
  justify-content: center;
  min-height: 40px;
  padding: 8px 0;
  color: var(--text-secondary, currentColor);
}
.md-slide-preview-page .md-slide-diagram.chan-slide-align-left .md-slide-diagram-body {
  justify-content: flex-start;
}
.md-slide-preview-page .md-slide-diagram.chan-slide-align-right .md-slide-diagram-body {
  justify-content: flex-end;
}
.md-slide-preview-page .md-slide-diagram-body svg {
  max-width: 100%;
  height: auto;
}
.md-slide-preview-page .md-slide-excalidraw {
  display: flex;
  justify-content: center;
  margin: 0.7em 0 0.9em;
}
.md-slide-preview-page .md-slide-excalidraw-body {
  display: flex;
  justify-content: center;
  max-width: 100%;
  min-height: 40px;
  padding: 8px 0;
  color: var(--text-secondary, currentColor);
}
.md-slide-preview-page .md-slide-excalidraw-body svg {
  max-width: 100%;
  height: auto;
}
.md-slide-preview-page .md-slide-excalidraw-body.md-slide-excalidraw-error {
  display: block;
  color: var(--danger-text, #d33);
  font-family: ui-monospace, monospace;
  font-size: 12px;
  white-space: pre-wrap;
}
.md-slide-preview-page .md-slide-diagram-body.md-slide-diagram-error {
  display: block;
  color: var(--danger-text, #d33);
  font-family: ui-monospace, monospace;
  font-size: 12px;
  white-space: pre-wrap;
}
.md-slide-preview-page .md-slide-diagram-error-head {
  font-weight: 600;
  margin-bottom: 2px;
}
.md-slide-preview-page .md-slide-diagram-error-src {
  padding: 2px 6px;
  margin-bottom: 4px;
  border-left: 2px solid var(--danger-text, #d33);
  background: var(--bg-card, rgba(0, 0, 0, 0.04));
  white-space: pre;
  overflow-x: auto;
}
.md-slide-preview-page a { color: var(--chan-editor-link-color, var(--link, #0969da)); }
.md-slide-preview-page blockquote {
  color: var(--chan-editor-quote-color, inherit);
  border-left: 3px solid var(--chan-editor-quote-border, rgba(127,127,127,0.4));
  padding-left: 0.8em;
}
.md-slide-preview-page table {
  width: 100%;
  border-collapse: collapse;
}
.md-slide-preview-page th,
.md-slide-preview-page td {
  border: 1px solid var(--chan-editor-table-border, rgba(127,127,127,0.35));
  padding: 0.35em 0.5em;
}
`;
}
