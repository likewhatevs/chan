<script lang="ts">
  // Inspector body that renders metadata for a single file or directory.
  // Looks the entry up from the global tree by path; renders nothing
  // until a path is supplied (callers that want a placeholder pass
  // their own empty state outside this component, or pass `null`
  // and the host's body slot stays empty).
  //
  // Used by:
  //   - File Browser tab/dock: shows the current selection
  //     (browserSelection.path) plus an Open / × pair so the panel
  //     doubles as the action surface for the tree. References
  //     section (tags / mentions / dates / links / backlinks) is
  //     enabled here via showRefs.
  //   - FileEditorTab: shown inside a "show info" disclosure for the
  //     currently-edited file; lean layout (no Open/Close, no refs).
  //
  // Directory mode walks the flat tree to compute aggregate counts +
  // size + most-recent mtime. The walk is O(N) in tree size and only
  // re-runs when the selected path changes ($derived dependency
  // tracking does the gating).

  import { api, withTokenQuery } from "../api/client";
  import { ApiError } from "../api/errors";
  import type {
    GraphEdge,
    InspectorPayload,
    PathClass,
    ReportFileStats,
    ReportPrefix,
    TreeEntry,
  } from "../api/types";
  import { isEditableText, isImage, isPdf } from "../state/fileTypes";
  import { basename, formatMtime, formatSize } from "../state/format";
  import {
    ensureGraphLoaded,
    graphData,
    selectionEdgesFor,
  } from "../state/graphData.svelte";
  import { openImageZoom } from "../state/imageZoom";
  import { openPdfViewer } from "../state/pdfViewer";
  import {
    drive,
    loadTreeDir,
    openGraphAtNode,
    openGraphForFile,
    openGraphForTag,
    tree,
  } from "../state/store.svelte";
  import { classifyEntry } from "../state/kinds";
  import KindChip from "./KindChip.svelte";

  /// Visual / behavioural kind for a file reference. Images route to
  /// the fullscreen zoom overlay (editor's "Zoom" button shares the
  /// same helper); contacts are markdown notes flagged with the
  /// `chan.kind: contact` frontmatter and open in the editor like
  /// other docs but get their own chip color so a glance distinguishes
  /// them. The contact bit comes off the server-side tree listing
  /// (which joins chan-drive's node-kind index) rather than a path
  /// heuristic, so contacts located outside `Contacts/` still classify
  /// correctly. Anything else is a doc.
  type RefKind = "doc" | "image" | "contact";
  function classifyRef(path: string): RefKind {
    if (isImage(path)) return "image";
    const e = entryByPath.get(path);
    if (e && !e.is_dir && e.kind === "contact") return "contact";
    return "doc";
  }

  let {
    path,
    onOpen,
    onReveal,
    onClose,
    showRefs = false,
    onNavigate,
    onContactNavigate,
    onSetAsScope,
  }: {
    path: string | null;
    onOpen?: () => void;
    /// Image / directory counterpart to `onOpen`. Renders a
    /// "Show in file browser" button on image entries and a
    /// "Show Directory" button on directory entries; the host reveals
    /// the path in its tree and closes itself. Absent = no button
    /// (e.g. when the inspector already lives inside the file
    /// browser).
    onReveal?: () => void;
    onClose?: () => void;
    /// When true, fetch + render tags / mentions / dates / links /
    /// backlinks for files. Off by default so the file editor's
    /// inline disclosure stays compact.
    showRefs?: boolean;
    /// Click handler for a link / backlink target. Receives the
    /// peer file path. Hosts decide whether to open it in the active
    /// pane and close themselves; absent = entries render as
    /// non-clickable.
    onNavigate?: (path: string) => void;
    /// Click handler for a resolved contact pill. Receives the
    /// contact file's path. Graph overlay binds this to "select
    /// the contact's node on the canvas" so contacts behave like
    /// documents in the inspector. Absent = fall back to
    /// openGraphForFile (the original "contacts are graph anchors,
    /// not destinations" behavior the file browser and search
    /// overlays still want).
    onContactNavigate?: (path: string) => void;
    /// "Graph from here" button for file selections. Graph overlay binds
    /// this to scope the current graph to the selected file (and
    /// re-pin it as the focal node). Other hosts leave it absent so
    /// the button doesn't render outside the graph.
    onSetAsScope?: () => void;
  } = $props();

  const entryByPath = $derived(
    new Map(tree.entries.map((e) => [e.path, e])),
  );

  const entry = $derived.by(() => {
    if (path === null || path === undefined) return null;
    if (path === "") {
      // Drive root: no entry exists in tree.entries (the listing only
      // contains children). Synthesize a directory-shaped record so the
      // directory branch renders aggregate stats over the whole drive.
      // The mtime field stays null because the drive root has no
      // intrinsic timestamp; dirStats picks up the latest mtime
      // across descendants instead.
      return {
        path: "",
        is_dir: true,
        mtime: null,
        size: 0,
      } as TreeEntry;
    }
    return entryByPath.get(path) ?? null;
  });

  /// The file tree lazy-loads directory contents, so opening a file
  /// the user hasn't yet drilled into via the browser (a direct URL,
  /// the editor tab's "Show Details", a search result) leaves the
  /// entry missing from `tree.entries` and the inspector blank.
  /// When that happens, fetch the parent directory's listing so the
  /// entry shows up. Cheap: `loadTreeDir` short-circuits when the
  /// dir is already loaded or in flight.
  $effect(() => {
    if (!path || entry) return;
    const parent = path.includes("/")
      ? path.slice(0, path.lastIndexOf("/"))
      : "";
    void loadTreeDir(parent);
  });

  const dirStats = $derived.by(() => {
    if (!entry || !entry.is_dir) return null;
    const prefix = entry.path ? `${entry.path}/` : "";
    let files = 0;
    let dirs = 0;
    let bytes = 0;
    let latest: number | null = null;
    for (const e of tree.entries) {
      if (prefix && !e.path.startsWith(prefix)) continue;
      if (e.path === entry.path) continue;
      if (e.is_dir) dirs += 1;
      else {
        files += 1;
        bytes += e.size;
      }
      if (e.mtime !== null && (latest === null || e.mtime > latest)) {
        latest = e.mtime;
      }
    }
    return { files, dirs, bytes, latest };
  });

  let inspectorPayload = $state<InspectorPayload | null>(null);
  let inspectorReq = 0;

  $effect(() => {
    inspectorPayload = null;
    if (path === null || path === undefined) return;
    const req = ++inspectorReq;
    void api.inspector(path)
      .then((payload) => {
        if (req === inspectorReq) inspectorPayload = payload;
      })
      .catch(() => {
        if (req === inspectorReq) inspectorPayload = null;
      });
  });

  const pathClass = $derived<PathClass | null>(
    inspectorPayload?.path_class ?? entry?.path_class ?? null,
  );
  const subtree = $derived(inspectorPayload?.subtree ?? null);
  const fileKindCounts = $derived.by(() => {
    if (!subtree) return [];
    return Object.entries(subtree.file_kinds)
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .slice(0, 6);
  });
  const specialBadges = $derived.by(() => {
    if (!pathClass) return [];
    const out: string[] = [];
    if (pathClass.permission === "read_only") out.push("read-only");
    if (pathClass.kind === "symlink") out.push("symlink");
    else if (pathClass.kind !== "regular_file" && pathClass.kind !== "directory") {
      out.push(pathClass.kind.replace(/_/g, " "));
    }
    if (pathClass.link_count > 1) out.push(`${pathClass.link_count} links`);
    if (pathClass.target_escapes_drive) out.push("outside drive");
    return out;
  });

  /// Outgoing edges (tags / mentions / dates / links) come straight
  /// out of the shared graph store; null while the graph hasn't
  /// loaded yet so the template can render a "loading" line.
  const refs = $derived.by(() => {
    if (!showRefs || !entry || entry.is_dir) return null;
    if (!graphData.view) return null;
    return selectionEdgesFor(entry.path);
  });

  /// Contact pills surfaced under the Contacts section. Merges three
  /// graph-edge cases so a file that references a contact through any
  /// path lands as a single row, and the matching entry disappears
  /// from "Links to" so the reader doesn't see Alice listed twice:
  ///
  ///   1. mention -> contact file (server resolved `@@alice` to
  ///      `Contacts/alice.md`): a navigable row using the file's
  ///      label and path.
  ///   2. mention -> standalone `@@name` (no matching contact file):
  ///      a graph-only row labeled `name`.
  ///   3. link    -> contact-kind file ([[Contacts/alice]] or a
  ///      regular markdown link to it): same shape as case 1.
  ///
  /// Deduped by the target node id so authoring both `@@alice` AND a
  /// link to `Contacts/alice.md` in the same document collapses to
  /// one pill.
  type ContactPill = {
    key: string;
    label: string;
    path: string | null;
    onClick: () => void;
  };
  const contactPills = $derived.by<ContactPill[]>(() => {
    if (!refs) return [];
    const out: ContactPill[] = [];
    const seen = new Set<string>();
    const push = (p: ContactPill) => {
      if (seen.has(p.key)) return;
      seen.add(p.key);
      out.push(p);
    };
    // Resolved contacts route through the host's `onContactNavigate`
    // when present (graph overlay: select the contact's node on the
    // canvas, matching how documents behave there). When absent, fall
    // back to opening a graph scoped to the contact — the original
    // "contacts are network anchors" behavior the file browser and
    // search overlays still rely on.
    const navigateContact = onContactNavigate
      ? (p: string) => onContactNavigate(p)
      : (p: string) => openGraphForFile(p);
    for (const m of refs.mentions) {
      if (m.kind === "file" && !m.missing) {
        push({
          key: m.id,
          label: m.label,
          path: m.path,
          onClick: () => navigateContact(m.path),
        });
      } else {
        // Unresolved `@@name` — no matching contact on disk yet.
        // Falls back to drive-scoped graph with the mention node
        // pre-selected (openGraphAtNode), since there's no file
        // to scope to.
        push({
          key: m.id,
          label: m.label.replace(/^@@/, ""),
          path: null,
          onClick: () => openGraphAtNode(m.id),
        });
      }
    }
    for (const l of refs.links) {
      if (l.kind !== "file" || l.missing) continue;
      if (classifyRef(l.path) !== "contact") continue;
      push({
        key: l.id,
        label: l.label,
        path: l.path,
        onClick: () => navigateContact(l.path),
      });
    }
    return out;
  });

  /// Non-contact outgoing links. Contact-kind targets move to the
  /// Contacts pill list above so the reader doesn't see Alice listed
  /// twice in the inspector when the source authored both an `@@alice`
  /// and a real markdown link to her contact file.
  const nonContactLinks = $derived(
    refs
      ? refs.links.filter(
          (l) => !(l.kind === "file" && !l.missing && classifyRef(l.path) === "contact"),
        )
      : [],
  );

  /// Backlinks (incoming `link` edges) live behind a separate
  /// endpoint (/api/backlinks/{path}) since the graph snapshot only
  /// records outgoing edges per file node. Refetched whenever the
  /// selected path changes; stale responses are dropped via the
  /// request-id guard.
  let backlinks = $state<GraphEdge[]>([]);
  let backlinksLoading = $state(false);
  let backlinksError = $state<string | null>(null);
  let backlinkReq = 0;

  $effect(() => {
    if (!showRefs || !entry || entry.is_dir) {
      backlinks = [];
      backlinksLoading = false;
      backlinksError = null;
      return;
    }
    void ensureGraphLoaded();
    const req = ++backlinkReq;
    const target = entry.path;
    backlinksLoading = true;
    backlinksError = null;
    void api
      .backlinks(target)
      .then((edges) => {
        if (req !== backlinkReq) return;
        backlinks = edges.filter((e) => e.kind === "link");
        backlinksLoading = false;
      })
      .catch((err: unknown) => {
        if (req !== backlinkReq) return;
        backlinks = [];
        backlinksLoading = false;
        backlinksError = (err as Error).message;
      });
  });

  function navigate(targetPath: string): void {
    if (!targetPath) return;
    onNavigate?.(targetPath);
  }

  /// chan-report integration. The "Code" section shows language /
  /// SLOC / complexity for files, and the per-directory roll-up
  /// (totals + top languages + COCOMO) for directories. Fetched
  /// lazily whenever the selected path changes; a request-id guard
  /// drops stale responses if the user clicks through several
  /// entries faster than the round-trip.
  ///
  /// 404 on the file endpoint is normal (binary / gitignored /
  /// unknown language) and silently hides the section. Other errors
  /// surface a one-line failure note so the user knows the call
  /// happened but didn't return data.
  let fileReport = $state<ReportFileStats | null>(null);
  let prefixReport = $state<ReportPrefix | null>(null);
  let reportLoading = $state(false);
  let reportError = $state<string | null>(null);
  let reportReq = 0;

  /// "Top N + see more" toggle for the per-language list in directory
  /// mode. Default of 5 matches the inspector's appetite for compact
  /// sections; the full list is one click away. Resets to collapsed
  /// whenever the selection changes so a new directory doesn't inherit
  /// the previous one's expand state.
  const LANG_PREVIEW = 5;
  let langExpanded = $state(false);

  $effect(() => {
    fileReport = null;
    prefixReport = null;
    reportError = null;
    langExpanded = false;
    if (!entry) {
      reportLoading = false;
      return;
    }
    const req = ++reportReq;
    const target = entry;
    reportLoading = true;
    const fetcher: Promise<ReportFileStats | ReportPrefix | null> = target.is_dir
      ? api.reportPrefix(target.path)
      : api.reportFile(target.path).catch((err: unknown) => {
          // 404 is the "no report row" case; treat it as an empty
          // result rather than an error so the section just hides.
          if (err instanceof ApiError && err.status === 404) return null;
          throw err;
        });
    void fetcher
      .then((res) => {
        if (req !== reportReq) return;
        if (target.is_dir) {
          prefixReport = (res as ReportPrefix | null) ?? null;
        } else {
          fileReport = (res as ReportFileStats | null) ?? null;
        }
        reportLoading = false;
      })
      .catch((err: unknown) => {
        if (req !== reportReq) return;
        reportError = (err as Error).message;
        reportLoading = false;
      });
  });

  /// Per-language roll-up sliced for display: collapse to top N by
  /// SLOC unless the user clicked "see more". The hidden count
  /// drives the "+N more" affordance label.
  const visibleLanguages = $derived.by(() => {
    if (!prefixReport) return [];
    const all = prefixReport.by_language;
    if (langExpanded || all.length <= LANG_PREVIEW) return all;
    return all.slice(0, LANG_PREVIEW);
  });
  const hiddenLanguageCount = $derived(
    prefixReport
      ? Math.max(0, prefixReport.by_language.length - visibleLanguages.length)
      : 0,
  );

  /// COCOMO formatting helpers. We deliberately drop estimated cost
  /// from the inspector: the dollar number is a default-salary
  /// extrapolation that's noisy for a personal notes app. Effort,
  /// schedule, and developer-count carry the useful signal.
  function fmtMonths(n: number): string {
    if (!Number.isFinite(n)) return "—";
    return n >= 10 ? `${Math.round(n)} mo` : `${n.toFixed(1)} mo`;
  }
  function fmtDevs(n: number): string {
    if (!Number.isFinite(n)) return "—";
    return n >= 10 ? `${Math.round(n)}` : n.toFixed(1);
  }
</script>

{#if !entry}
  <div class="empty">
    <div class="empty-title">Details</div>
    <div class="empty-hint">click a file or directory to inspect</div>
  </div>
{:else if entry.is_dir}
  <div class="info">
    <header class="head">
      <KindChip kind="folder" block />
    </header>
    <h3 class="title" title={entry.path || "/"}>
      {basename(entry.path) || drive.info?.name || "(root)"}
    </h3>
    {#if specialBadges.length > 0}
      <div class="badge-row">
        {#each specialBadges as badge}
          <span class="flag-badge">{badge}</span>
        {/each}
      </div>
    {/if}
    {#if dirStats}
      <div class="meta-grid">
        <span class="k">files</span>
        <span class="v">{subtree?.files ?? dirStats.files}</span>
        <span class="k">subdirectories</span>
        <span class="v">{subtree?.directories ?? dirStats.dirs}</span>
        <span class="k">size</span>
        <span class="v">{formatSize(subtree?.bytes ?? dirStats.bytes)}</span>
        <span class="k">last change</span>
        <span class="v">{formatMtime(dirStats.latest)}</span>
        {#if pathClass?.target}
          <span class="k">target</span>
          <span class="v mono" title={pathClass.target}>{pathClass.target}</span>
        {/if}
      </div>
    {/if}
    {#if fileKindCounts.length > 0}
      <section class="refs compact-section">
        <h4>File Kinds</h4>
        <div class="kind-counts">
          {#each fileKindCounts as [kind, count]}
            <span class="kind-count"><span>{kind}</span><strong>{count}</strong></span>
          {/each}
        </div>
      </section>
    {/if}
    {#if prefixReport && prefixReport.totals.files > 0}
      <section class="refs">
        <h4>Code</h4>
        <div class="meta-grid">
          <span class="k">indexed</span>
          <span class="v">{prefixReport.totals.files}</span>
          <span class="k">SLOC</span>
          <span class="v">{prefixReport.totals.code.toLocaleString()}</span>
          <span class="k">comments</span>
          <span class="v">{prefixReport.totals.comments.toLocaleString()}</span>
          <span class="k">blanks</span>
          <span class="v">{prefixReport.totals.blanks.toLocaleString()}</span>
          <span class="k">complexity</span>
          <span class="v">{prefixReport.totals.complexity.toLocaleString()}</span>
        </div>
        {#if prefixReport.by_language.length > 0}
          <ul class="lang-list">
            {#each visibleLanguages as lang (lang.name)}
              <li class="lang-row">
                <span class="lang-name" title={lang.name}>{lang.name}</span>
                <span class="lang-files">{lang.files} file{lang.files === 1 ? "" : "s"}</span>
                <span class="lang-sloc">{lang.code.toLocaleString()} SLOC</span>
              </li>
            {/each}
          </ul>
          {#if hiddenLanguageCount > 0}
            <button
              type="button"
              class="see-more"
              onclick={() => (langExpanded = true)}
            >+{hiddenLanguageCount} more</button>
          {:else if langExpanded && prefixReport.by_language.length > LANG_PREVIEW}
            <button
              type="button"
              class="see-more"
              onclick={() => (langExpanded = false)}
            >show fewer</button>
          {/if}
        {/if}
        <div class="cocomo">
          <div class="cocomo-title">COCOMO ({prefixReport.cocomo.model})</div>
          <div class="meta-grid">
            <span class="k">effort</span>
            <span class="v">{fmtMonths(prefixReport.cocomo.effort_person_months)}</span>
            <span class="k">schedule</span>
            <span class="v">{fmtMonths(prefixReport.cocomo.schedule_months)}</span>
            <span class="k">developers</span>
            <span class="v">{fmtDevs(prefixReport.cocomo.developers)}</span>
          </div>
        </div>
      </section>
    {:else if reportLoading}
      <div class="refs-loading">loading report…</div>
    {:else if reportError}
      <div class="refs-error">report unavailable: {reportError}</div>
    {/if}
    {#if onReveal}
      <!-- "Show Directory": jump to a file browser tab with this directory
           selected. Hosted by surfaces that don't already live inside
           the browser (graph fs-mode inspector). The file browser
           itself leaves this prop unbound so the button doesn't
           render twice on its own surface. -->
      <button class="open" onclick={onReveal}>Show Directory</button>
    {/if}
    {#if onSetAsScope}
      <button class="open" onclick={onSetAsScope}>Graph from here</button>
    {/if}
    <!-- `fullstack-42` refined: inspector buttons stay; the menu-
         level duplicates were dropped instead. Pane Mode's Cmd+K 2
         / 3 (with `fullstack-43`'s context) is the keyboard
         equivalent of these affordances. -->
  </div>
{:else}
  {@const editable = isEditableText(entry.path)}
  {@const image = isImage(entry.path)}
  {@const pdf = isPdf(entry.path)}
  {@const fileKind = classifyEntry(entry)}
  <div class="info">
    <header class="head">
      <KindChip kind={fileKind} block />
    </header>
    <h3 class="title" title={entry.path}>{basename(entry.path)}</h3>
    {#if specialBadges.length > 0}
      <div class="badge-row">
        {#each specialBadges as badge}
          <span class="flag-badge">{badge}</span>
        {/each}
      </div>
    {/if}
    {#if image}
      <!-- Inline preview. Bytes come from /api/files with the
           per-launch bearer token appended as a query param so the
           browser's <img> can fetch without a custom Authorization
           header. Object-fit contains so portrait + landscape both
           sit cleanly in the fixed-height frame.
           Wrapped in a button so a click on the preview opens the
           shared fullscreen zoom overlay (matches the editor's
           image action overlay). -->
      <button
        type="button"
        class="image-preview"
        title="Zoom"
        onclick={() => openImageZoom(entry.path)}
      >
        <img
          src={withTokenQuery(`/api/files/${encodeURIComponent(entry.path).replace(/%2F/g, "/")}`)}
          alt={basename(entry.path)}
          loading="lazy"
        />
      </button>
    {/if}
    <div class="meta-grid">
      <span class="k">size</span>
      <span class="v">{formatSize(entry.size)}</span>
      <span class="k">modified</span>
      <span class="v">{formatMtime(entry.mtime)}</span>
      {#if pathClass?.target}
        <span class="k">target</span>
        <span class="v mono" title={pathClass.target}>{pathClass.target}</span>
      {/if}
      {#if showRefs && !image && !pdf}
        <span class="k">tags</span>
        <span class="v">{refs ? refs.tags.length : "…"}</span>
        <span class="k">contacts</span>
        <span class="v">{refs ? contactPills.length : "…"}</span>
        <span class="k">dates</span>
        <span class="v">{refs ? refs.dates.length : "…"}</span>
        <span class="k">links out</span>
        <span class="v">{refs ? nonContactLinks.length : "…"}</span>
        <span class="k">backlinks</span>
        <span class="v">{backlinksLoading ? "…" : backlinks.length}</span>
      {:else if showRefs && (image || pdf)}
        <!-- Media files (images and PDFs) can be link targets but
             carry no outgoing references of their own. Show just
             the "linked from" count; tags / contacts / dates would
             always be zero. -->
        <span class="k">linked from</span>
        <span class="v">{backlinksLoading ? "…" : backlinks.length}</span>
      {/if}
    </div>
    {#if fileReport}
      <section class="refs">
        <h4>Code</h4>
        <div class="meta-grid">
          <span class="k">language</span>
          <span class="v">{fileReport.language}</span>
          <span class="k">SLOC</span>
          <span class="v">{fileReport.code.toLocaleString()}</span>
          <span class="k">comments</span>
          <span class="v">{fileReport.comments.toLocaleString()}</span>
          <span class="k">blanks</span>
          <span class="v">{fileReport.blanks.toLocaleString()}</span>
          <span class="k">complexity</span>
          <span class="v">{fileReport.complexity.toLocaleString()}</span>
        </div>
      </section>
    {:else if reportError}
      <div class="refs-error">report unavailable: {reportError}</div>
    {/if}
    {#if image}
      {#if onReveal}
        <button class="open" onclick={onReveal}>Show in file browser</button>
      {/if}
    {:else if pdf}
      <!-- PDFs ride the same `media` kind on the wire as images but
           render via `<embed type="application/pdf">` (browser's
           built-in viewer) instead of `<img>`. The fullscreen
           overlay is the equivalent of openImageZoom for PDFs. -->
      <button class="open" onclick={() => openPdfViewer(entry.path)}>
        View PDF
      </button>
      {#if onReveal}
        <button class="open" onclick={onReveal}>Show in file browser</button>
      {/if}
    {:else if onOpen}
      {#if editable}
        <button class="open" onclick={onOpen}>Open</button>
      {:else}
        <p class="view-only-hint">
          Not an editable file.
        </p>
      {/if}
    {/if}
    {#if onReveal && !image && !pdf}
      <!-- "Show File": reveal this file in the file browser. The
           editor binds this so its details panel mirrors the tab
           menu's reveal action; image / pdf entries render their
           own "Show in file browser" variant a few lines above. -->
      <button class="open" onclick={onReveal}>Show File</button>
    {/if}
    {#if onSetAsScope}
      <!-- "Graph from here" re-scopes the current graph to this file (or
           image) and re-pins it as the focal node. Only rendered
           when the host wires it up (today: the graph surface). -->
      <button class="open" onclick={onSetAsScope}>Graph from here</button>
    {/if}
    <!-- `fullstack-42` refined: inspector buttons stay; the menu-
         level duplicates (doc-tab right-click "Show File", terminal
         right-click "Show Dir", etc.) were dropped instead. -->
    {#if showRefs}
      {#if !graphData.view && graphData.loading}
        <div class="refs-loading">loading references…</div>
      {:else if graphData.error}
        <div class="refs-error">references unavailable: {graphData.error}</div>
      {:else if refs}
        {#if refs.tags.length > 0}
          <section class="refs">
            <h4>Tags</h4>
            <ul>
              {#each refs.tags as t (t.id)}
                <li>
                  <button
                    class="ref tag"
                    onclick={() => openGraphForTag(t.id, t.label)}
                    title="open in graph (scoped to this tag)"
                  >{t.label}</button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if contactPills.length > 0}
          <section class="refs">
            <h4>Contacts</h4>
            <ul>
              {#each contactPills as c (c.key)}
                <li>
                  <button
                    class="ref contact"
                    onclick={c.onClick}
                    title={c.path
                      ? `open in graph (scoped to ${c.path})`
                      : "open in graph"}
                  >{c.label}</button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if refs.dates.length > 0}
          <section class="refs">
            <h4>Dates</h4>
            <ul>
              {#each refs.dates as d (d.id)}
                <li>
                  <button
                    class="ref date"
                    onclick={() => openGraphAtNode(d.id)}
                    title="open in graph"
                  >{d.label}</button>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if nonContactLinks.length > 0}
          <section class="refs">
            <h4>Links to</h4>
            <ul>
              {#each nonContactLinks as l (l.id)}
                <li>
                  {#if l.kind !== "file"}
                    <span class="ref file">{l.label}</span>
                  {:else if l.missing}
                    <span class="ref file missing" data-refkind={classifyRef(l.path)}>{l.label}</span>
                  {:else if classifyRef(l.path) === "image"}
                    <button
                      class="ref file"
                      data-refkind="image"
                      title="Zoom"
                      onclick={() => openImageZoom(l.path)}
                    >{l.label}</button>
                  {:else if onNavigate}
                    <button
                      class="ref file"
                      data-refkind={classifyRef(l.path)}
                      onclick={() => navigate(l.path)}
                    >{l.label}</button>
                  {:else}
                    <span class="ref file" data-refkind={classifyRef(l.path)}>{l.label}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}
        {#if backlinks.length > 0}
          <section class="refs">
            <h4>Backlinks</h4>
            <ul>
              {#each backlinks as b (b.src)}
                <li>
                  {#if classifyRef(b.src) === "image"}
                    <button
                      class="ref file"
                      data-refkind="image"
                      title="Zoom"
                      onclick={() => openImageZoom(b.src)}
                    >{b.src}</button>
                  {:else if onNavigate}
                    <button
                      class="ref file"
                      data-refkind={classifyRef(b.src)}
                      onclick={() => navigate(b.src)}
                    >{b.src}</button>
                  {:else}
                    <span class="ref file" data-refkind={classifyRef(b.src)}>{b.src}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          </section>
        {:else if backlinksError}
          <div class="refs-error">backlinks unavailable: {backlinksError}</div>
        {/if}
      {/if}
    {/if}
  </div>
{/if}

<style>
  .info {
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  .empty {
    text-align: center;
    color: var(--text-secondary);
    padding: 1.2rem 0.7rem 0.8rem 0.7rem;
  }
  .empty-title {
    font-weight: 600;
    color: var(--text);
    margin-bottom: 0.25rem;
  }
  .empty-hint {
    font-style: italic;
    font-size: 14px;
    opacity: 0.85;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  /* Image preview frame: fixed max height, checkered fallback bg
     (visible while bytes are loading or for images with alpha so
     the panel doesn't show empty space). object-fit contain keeps
     the natural aspect ratio so portraits and landscapes both fit. */
  /* Image preview frame: fixed max height, checkered fallback bg
     (visible while bytes are loading or for images with alpha so
     the panel doesn't show empty space). object-fit contain keeps
     the natural aspect ratio so portraits and landscapes both fit.
     Rendered as a <button> so a click opens the fullscreen zoom
     overlay; default button chrome is stripped. */
  .image-preview {
    margin: 0 0 0.6rem 0;
    padding: 4px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    max-height: 220px;
    overflow: hidden;
    width: 100%;
    cursor: zoom-in;
    font: inherit;
    color: inherit;
  }
  .image-preview:hover { border-color: var(--btn-hover); }
  .image-preview img {
    max-width: 100%;
    max-height: 210px;
    object-fit: contain;
    display: block;
    pointer-events: none;
  }
  .view-only-hint {
    color: var(--text-secondary);
    font-size: 14px;
    font-style: italic;
    margin: .4rem 0 0 0;
  }
  .title {
    margin: 0 0 0.5rem 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
  }
  .badge-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin: -0.15rem 0 0.45rem 0;
  }
  .flag-badge {
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 1px 5px;
    color: var(--text-secondary);
    background: var(--bg-card);
    font-size: 11px;
    line-height: 1.4;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 2px 0.5rem;
    margin: 0.4rem 0 0.6rem 0;
    font-size: 14px;
  }
  .meta-grid .k { color: var(--text-secondary); }
  .meta-grid .v {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .meta-grid .mono {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .open {
    width: 100%;
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 0;
    cursor: pointer;
    font: inherit;
    /* Own the spacing above the button so it doesn't matter whether
       the preceding element has a bottom margin (file: standalone
       .meta-grid keeps 0.6rem; directory: the cocomo grid zeroes its
       margin and used to leave the button flush against
       "developers"). Adjacent buttons collapse to a tighter gap so
       a group of actions reads as a single block. */
    margin-top: 0.6rem;
  }
  .open + .open { margin-top: 0.35rem; }
  .open:hover { border-color: var(--btn-hover); }
  /* Reference sections (tags / mentions / dates / links / backlinks).
     Visual style mirrors the graph panel's aside so the two
     inspectors feel like one feature. */
  .refs {
    margin: 0.6rem 0 0 0;
  }
  .compact-section {
    margin-top: 0.35rem;
  }
  .refs h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0 0 0.25rem 0;
  }
  .refs ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .refs li { margin: 0; }
  .kind-counts {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .kind-count {
    display: inline-flex;
    gap: 5px;
    align-items: center;
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 5px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .kind-count strong {
    color: var(--text);
    font-weight: 600;
  }
  .ref {
    display: block;
    width: 100%;
    text-align: left;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 13px;
    color: var(--text);
    cursor: default;
    font: inherit;
    line-height: 1.5;
    word-break: break-word;
  }
  button.ref {
    cursor: pointer;
  }
  button.ref:hover {
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }
  .ref.tag { color: var(--accent); }
  .ref.date { color: var(--text-secondary); }
  /* Backlinks / link targets: use the standard text color rather
     than --link. The chip already reads as a clickable button
     thanks to the surface + hover treatment, and the doc/file
     name is the actual content, not a call-to-action.
     Per-kind left accent (doc / image / contact) lets the reader
     scan the list and pick out images vs contacts vs plain docs
     without a per-row icon. Padding is fixed so swapping accent
     widths doesn't shift the label. */
  .ref.file {
    color: var(--text);
    word-break: break-all;
    border-left-width: 3px;
    padding-left: 6px;
  }
  .ref.file[data-refkind="doc"] { border-left-color: var(--g-doc); }
  .ref.file[data-refkind="image"] { border-left-color: var(--g-img); }
  .ref.file[data-refkind="contact"] { border-left-color: var(--warn-text); }
  /* Broken link target: the linked file no longer exists on the drive.
     Distinct color + strikethrough so a glance flags the dangling
     reference; kind accent stripe is kept so the reader still sees
     what was being pointed at. */
  .ref.file.missing {
    color: var(--danger-text);
    font-style: italic;
    text-decoration: line-through;
    text-decoration-color: var(--danger-text);
  }
  /* Contact rows in the Contacts section: same block-button shape as
     the other ref types, with a small person silhouette prefixed in
     --warn-text so a glance tells you the entry is a person rather
     than a generic doc. Icon matches the editor wiki pill's glyph for
     visual continuity. */
  .ref.contact {
    color: var(--warn-text);
    padding-left: 22px;
    position: relative;
  }
  .ref.contact::before {
    content: "";
    position: absolute;
    left: 6px;
    top: 50%;
    width: 12px;
    height: 12px;
    transform: translateY(-50%);
    background: currentColor;
    -webkit-mask: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><circle cx='8' cy='5' r='3'/><path d='M2 14c0-3 3-5 6-5s6 2 6 5z'/></svg>") center / contain no-repeat;
    mask: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'><circle cx='8' cy='5' r='3'/><path d='M2 14c0-3 3-5 6-5s6 2 6 5z'/></svg>") center / contain no-repeat;
  }
  .refs-loading,
  .refs-error {
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 0.6rem;
    font-style: italic;
  }
  .refs-error { color: var(--warn-text); font-style: normal; }
  /* Per-language row in the Code section. Three columns: language
     name on the left (allowed to grow), file count + SLOC on the
     right (tabular-nums so the digit columns line up across rows). */
  .lang-list {
    list-style: none;
    padding: 0;
    margin: 0.4rem 0 0 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .lang-row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 0.5rem;
    font-size: 13px;
    align-items: baseline;
  }
  .lang-name {
    color: var(--text);
    word-break: break-word;
  }
  .lang-files,
  .lang-sloc {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .see-more {
    display: block;
    margin: 0.3rem 0 0 0;
    background: none;
    border: none;
    color: var(--link);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 0;
  }
  .see-more:hover { text-decoration: underline; }
  .cocomo {
    margin-top: 0.5rem;
    padding-top: 0.4rem;
    border-top: 1px dashed var(--border);
  }
  .cocomo-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin-bottom: 0.2rem;
  }
  .cocomo .meta-grid {
    margin: 0;
  }
</style>
