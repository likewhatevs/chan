<script lang="ts">
  // Workspace inspector body. Shown in the file browser's Inspector pane
  // when the user clicks the Directory row in the hamburger menu, and in
  // the graph when the workspace-root node is selected. Houses
  // the global Notes Directories config (default root + recent workspaces list).
  // Search index status lives in the Search Status overlay.
  //
  // Parity with FileInfoBody's directory mode: this body renders the same
  // aggregate stats (files, subdirs, size, last change), file-kind
  // counts, and code report a regular folder inspector shows. The
  // workspace chip + title up top stay distinct (workspace-rooted, not a
  // generic folder), and the global Notes-directory config section sits
  // below the parity content.

  import { onMount } from "svelte";
  import { api } from "../api/client";
  import type {
    GlobalConfig,
    InspectorPayload,
    ReportPrefix,
  } from "../api/types";
  import { formatMtime, formatSize } from "../state/format";
  import {
    fileOps,
    openGraphAtNode,
    openGraphForContact,
    openGraphForLanguage,
    revealPathInBrowser,
    tree,
    workspace,
  } from "../state/store.svelte";
  import { ensureGraphLoaded, graphData } from "../state/graphData.svelte";
  import {
    downloadTransfer,
    downloadTransferActive,
    clearDownloadTransfer,
  } from "../state/downloadTransfer.svelte";
  import { openTerminalInActivePane } from "../state/tabs.svelte";
  import { terminalFromHereTarget } from "../terminal/fromHere";
  import InspectorActionPill, {
    type InspectorAction,
  } from "./InspectorActionPill.svelte";

  /// Optional "Graph from here" callback. Consumers that host this
  /// body alongside an existing inspector convention pass it;
  /// surfaces that don't leave it unset and the button doesn't
  /// render. The action's semantic differs per consumer
  /// (FileBrowserSurface SPAWNS a new Graph tab while GraphPanel's own
  /// inspector RE-SCOPES the current tab), so the consumer wires the
  /// function and this body is callback-agnostic.
  ///
  /// The workspace root behaves like any other directory inspector.
  /// `variant` selects between the two surfaces:
  ///   - "inspector" (default): render the standard directory ACTION
  ///     ROW (Upload / Download / Show in File Browser / Graph from
  ///     here). The Notes-directories config does NOT render here.
  ///   - "dashboard": render the Notes-directories config. The action
  ///     row does NOT render.
  /// The aggregate stats grid, File Kinds, and Code/COCOMO sections
  /// render in both variants. `onReveal` wires the "Show in File
  /// Browser" button (inspector variant only).
  /// The workspace inspector matches FileInfoBody's clickable
  /// Languages + Contacts:
  ///   - `onLanguageClick`: fired when a language row is clicked;
  ///     opens that language's graph lens. Mirrors FileInfoBody's
  ///     `openGraphForLanguage(lang.name)` wiring.
  ///   - `onContactNavigate`: fired when a contact pill is clicked
  ///     for a RESOLVED contact (a `chan.kind: contact` file). The
  ///     graph host can select the node on the canvas; other hosts
  ///     spawn a contact-scoped lens. Unresolved `@@name` mentions
  ///     always route through `openGraphAtNode` (no file to scope to).
  /// Both default to the store helpers so a standalone mount stays
  /// functional even when a host doesn't wire them.
  let {
    variant = "inspector",
    onReveal,
    onSetAsScope,
    onLanguageClick = openGraphForLanguage,
    onContactNavigate,
  }: {
    variant?: "inspector" | "dashboard";
    onReveal?: () => void;
    onSetAsScope?: () => void;
    onLanguageClick?: (language: string) => void;
    onContactNavigate?: (path: string) => void;
  } = $props();

  /// Action-row plumbing (inspector variant). Mirrors FileInfoBody's
  /// directory branch but pinned to the workspace ROOT, i.e. the
  /// relative path is the empty string "".
  let uploadInput = $state<HTMLInputElement | null>(null);

  function triggerUpload(): void {
    uploadInput?.click();
  }

  async function onUploadPicked(e: Event): Promise<void> {
    const input = e.currentTarget as HTMLInputElement;
    const files = input.files;
    if (!files || files.length === 0) return;
    await fileOps.uploadFilesTo("", files);
    input.value = "";
  }

  function downloadSelection(): void {
    // Desktop routes through the progress-tracked capability (the
    // downloadTransfer store drives the indicator below); the browser
    // hands off to its native download manager. true = is_dir.
    fileOps.downloadPathWithProgress("", true);
  }

  /// Live desktop-download indicator (browser path leaves this null --
  /// the browser's own download manager owns the progress UI).
  const transfer = $derived(downloadTransfer.value);
  const downloadBusy = $derived(downloadTransferActive());

  const uploadTitle =
    "Upload adds the selected file to this directory. You can also drop files onto File Browser rows.";
  const downloadTitle =
    "Download this directory as a tar archive. You can also drag rows out of the File Browser where supported.";

  /// "File Browser" primary action for the workspace root. Mirrors
  /// FileInfoBody's openDirInBrowser: prefer the host's onReveal (the graph
  /// switches to a File Browser tab at the root) and otherwise reveal the
  /// root in the current browser (the File Browser tab leaves onReveal unset
  /// because the root already lives there).
  function openRootInBrowser(): void {
    if (onReveal) {
      onReveal();
      return;
    }
    revealPathInBrowser("", { enter: true, inspectorOpen: true });
  }

  /// "Terminal from here": a terminal rooted at the workspace root.
  function newTerminalHere(): void {
    openTerminalInActivePane(terminalFromHereTarget("", true));
  }

  /// Split-action model for the inspector variant: a "File Browser" primary
  /// plus the Upload / Download / Terminal / Graph dropdown. $derived so the
  /// Download item tracks the live downloadBusy state.
  const actionModel = $derived.by<{
    main: InspectorAction;
    secondary: InspectorAction[];
  }>(() => {
    const secondary: InspectorAction[] = [
      { label: "Upload", onClick: triggerUpload, title: uploadTitle },
      {
        label: "Download",
        onClick: downloadSelection,
        title: downloadTitle,
        disabled: downloadBusy,
      },
      { label: "Terminal from here", onClick: newTerminalHere },
    ];
    if (onSetAsScope) {
      secondary.push({ label: "Graph from here", onClick: onSetAsScope });
    }
    return {
      main: { label: "File Browser", onClick: openRootInBrowser },
      secondary,
    };
  });

  let globalConfig = $state<GlobalConfig | null>(null);
  let editedDefaultRoot = $state<string>("");
  let initialDefaultRoot = $state<string>("");
  let saveError = $state<string | null>(null);

  const AUTOSAVE_DELAY_MS = 500;
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  let inflight = false;

  async function loadGlobalConfig(): Promise<void> {
    try {
      globalConfig = await api.config();
      const cur = globalConfig.default_workspace_root ?? "";
      editedDefaultRoot = cur;
      initialDefaultRoot = cur;
    } catch {
      globalConfig = null;
    }
  }

  function dirty(): boolean {
    if (!globalConfig) return false;
    return editedDefaultRoot !== initialDefaultRoot;
  }

  async function save(): Promise<void> {
    if (!globalConfig || inflight) return;
    inflight = true;
    saveError = null;
    const sent = editedDefaultRoot;
    try {
      const trimmed = sent.trim();
      const body: GlobalConfig = {
        preferences: globalConfig.preferences,
        default_workspace_root: trimmed === "" ? null : trimmed,
        workspaces: globalConfig.workspaces,
      };
      const cfg = await api.updateConfig(body);
      globalConfig = cfg;
      // Don't clobber further edits the user typed while in flight.
      if (editedDefaultRoot === sent) {
        const echoed = cfg.default_workspace_root ?? "";
        editedDefaultRoot = echoed;
        initialDefaultRoot = echoed;
      } else {
        initialDefaultRoot = cfg.default_workspace_root ?? "";
      }
    } catch (e) {
      saveError = (e as Error).message;
    } finally {
      inflight = false;
      if (dirty()) scheduleSave();
    }
  }

  function scheduleSave(): void {
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      autosaveTimer = null;
      void save();
    }, AUTOSAVE_DELAY_MS);
  }

  $effect(() => {
    void editedDefaultRoot;
    if (!dirty()) return;
    scheduleSave();
  });

  function displayPathLabel(path: string): string {
    const stripped = path.replace(/[/\\]+$/, "");
    if (!stripped) return path || "(root)";
    const slash = Math.max(stripped.lastIndexOf("/"), stripped.lastIndexOf("\\"));
    return slash < 0 ? stripped : stripped.slice(slash + 1);
  }

  function formatLastSeen(iso: string): string {
    try {
      const d = new Date(iso);
      const yyyy = d.getUTCFullYear();
      const mm = String(d.getUTCMonth() + 1).padStart(2, "0");
      const dd = String(d.getUTCDate()).padStart(2, "0");
      const hh = String(d.getUTCHours()).padStart(2, "0");
      const mi = String(d.getUTCMinutes()).padStart(2, "0");
      return `${yyyy}-${mm}-${dd} ${hh}:${mi} UTC`;
    } catch {
      return iso;
    }
  }

  /// Aggregate stats walked from the loaded file tree. Mirrors
  /// FileInfoBody's `dirStats` derivation for the workspace root: every
  /// loaded entry contributes since the root has no prefix. The walk is
  /// O(N) in tree size and re-runs only when tree.entries changes
  /// ($derived dependency tracking). `latest` falls back to null when
  /// the tree is empty.
  const dirStats = $derived.by(() => {
    let files = 0;
    let dirs = 0;
    let bytes = 0;
    let latest: number | null = null;
    for (const e of tree.entries) {
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

  /// Server-side subtree summary for the workspace root. Authoritative
  /// over the tree walk because the tree only contains loaded children;
  /// `subtree.files` / `subtree.directories` / `subtree.bytes` count the
  /// whole workspace. Falls back to the dirStats walk while the request
  /// is in flight so the panel is never blank.
  let inspectorPayload = $state<InspectorPayload | null>(null);
  let inspectorReq = 0;
  $effect(() => {
    inspectorPayload = null;
    const req = ++inspectorReq;
    void api.inspector("")
      .then((payload) => {
        if (req === inspectorReq) inspectorPayload = payload;
      })
      .catch(() => {
        if (req === inspectorReq) inspectorPayload = null;
      });
  });
  const subtree = $derived(inspectorPayload?.subtree ?? null);
  const fileKindCounts = $derived.by(() => {
    if (!subtree) return [];
    return Object.entries(subtree.file_kinds)
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .slice(0, 6);
  });

  /// Whole-workspace code report. Same `/api/report/dir` cache with the
  /// `/api/report/prefix` walking fallback that FileInfoBody's dir branch
  /// uses, so the workspace inspector and a regular folder inspector both
  /// share the cheap path.
  let prefixReport = $state<ReportPrefix | null>(null);
  let reportLoading = $state(false);
  let reportError = $state<string | null>(null);
  let reportReq = 0;
  let langExpanded = $state(false);
  const LANG_PREVIEW = 5;

  $effect(() => {
    prefixReport = null;
    reportError = null;
    langExpanded = false;
    const req = ++reportReq;
    reportLoading = true;
    void api
      .reportDir("")
      .catch((e) => {
        const msg = (e as Error)?.message ?? "";
        if (/404/.test(msg) || /not found/i.test(msg)) {
          return api.reportPrefix("");
        }
        throw e;
      })
      .then((res) => {
        if (req !== reportReq) return;
        prefixReport = (res as ReportPrefix | null) ?? null;
        reportLoading = false;
      })
      .catch((err: unknown) => {
        if (req !== reportReq) return;
        reportError = (err as Error).message;
        reportLoading = false;
      });
  });

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

  /// Contacts section at the workspace root.
  /// FileInfoBody's contact pills derive from a single file's outgoing
  /// edges (`selectionEdgesFor(path)`); the workspace ROOT has no
  /// single-file refs, so the workspace-level "every contact in the
  /// workspace" source is the shared semantic graph snapshot. Two node
  /// kinds surface as contacts:
  ///   1. `kind:"file"` with `node_kind:"contact"`: a resolved contact
  ///      note (`chan.kind: contact` frontmatter). Navigates to its
  ///      contact-scoped lens (or the host's canvas selection).
  ///   2. `kind:"mention"`: an unresolved `@@name` with no contact file
  ///      on disk yet. Opens the workspace graph with the mention node
  ///      pre-selected, matching FileInfoBody's unresolved-mention arm.
  /// Loading the graph is cheap + shared: `graphData` is a global cache,
  /// and FileInfoBody already triggers the same load for any file's refs.
  $effect(() => {
    void ensureGraphLoaded();
  });

  type ContactPill = {
    key: string;
    label: string;
    path: string | null;
    onClick: () => void;
  };
  const contactPills = $derived.by<ContactPill[]>(() => {
    const view = graphData.view;
    if (!view) return [];
    const navigateContact = onContactNavigate
      ? (p: string) => onContactNavigate(p)
      : (p: string) => openGraphForContact(p);
    const out: ContactPill[] = [];
    const seen = new Set<string>();
    for (const n of view.nodes) {
      if (n.kind === "file" && n.node_kind === "contact" && !n.missing) {
        if (seen.has(n.id)) continue;
        seen.add(n.id);
        out.push({
          key: n.id,
          label: n.label,
          path: n.path,
          onClick: () => navigateContact(n.path),
        });
      } else if (n.kind === "mention") {
        if (seen.has(n.id)) continue;
        seen.add(n.id);
        const id = n.id;
        out.push({
          key: id,
          label: n.label.replace(/^@@/, ""),
          path: null,
          onClick: () => openGraphAtNode(id),
        });
      }
    }
    out.sort((a, b) => a.label.localeCompare(b.label));
    return out;
  });

  /// COCOMO formatting helpers; identical shape to FileInfoBody so the
  /// dir-mode and workspace-root inspectors format the same way.
  function fmtMonths(n: number): string {
    if (!Number.isFinite(n)) return " - ";
    return n >= 10 ? `${Math.round(n)} mo` : `${n.toFixed(1)} mo`;
  }
  function fmtDevs(n: number): string {
    if (!Number.isFinite(n)) return " - ";
    return n >= 10 ? `${Math.round(n)}` : n.toFixed(1);
  }

  onMount(() => {
    void loadGlobalConfig();
  });
</script>

<div class="info">
  <header class="head">
    <span class="kind-chip workspace">workspace</span>
  </header>
  <h3 class="title" title={workspace.info?.root}>
    {workspace.info?.label ?? "(workspace)"}
  </h3>
  <div class="meta-grid">
    <span class="k">directory</span>
    <span class="v mono path" title={workspace.info?.root}>{workspace.info?.root ?? ""}</span>
  </div>

  {#if variant === "inspector"}
    <!-- Workspace-root action row, mirroring FileInfoBody's is_dir branch on
         the workspace ROOT (relative path ""). A "File Browser" primary
         action plus a dropdown: Upload / Download / Terminal from here /
         Graph from here. "Graph from here" is gated on the host callback. -->
    <div class="actions-section">
      <InspectorActionPill
        main={actionModel.main}
        secondary={actionModel.secondary}
      />
    </div>
    {#if transfer}
      <div
        class="dl-indicator"
        class:err={!!transfer.error}
        role="status"
        aria-live="polite"
      >
        {#if transfer.error}
          <div class="dl-line">Download failed: {transfer.error}</div>
          <button
            class="dl-dismiss"
            type="button"
            onclick={clearDownloadTransfer}>Dismiss</button
          >
        {:else if transfer.savedPath}
          <div class="dl-line" title={transfer.savedPath}>
            Saved to {transfer.savedPath}
          </div>
          <button
            class="dl-dismiss"
            type="button"
            onclick={clearDownloadTransfer}>Dismiss</button
          >
        {:else}
          <div class="dl-progress" aria-hidden="true">
            <div
              class="dl-bar"
              class:indeterminate={transfer.progress === null}
              style={transfer.progress !== null
                ? `width: ${Math.round(transfer.progress * 100)}%`
                : ""}
            ></div>
          </div>
          <div class="dl-row">
            <span class="dl-line"
              >Downloading {transfer.filename}{transfer.progress !== null
                ? ` (${Math.round(transfer.progress * 100)}%)`
                : "..."}</span
            >
            {#if transfer.cancel}
              <button
                class="dl-dismiss"
                type="button"
                onclick={() => transfer.cancel?.()}>Cancel</button
              >
            {/if}
          </div>
        {/if}
      </div>
    {/if}
    <input
      bind:this={uploadInput}
      class="file-picker"
      type="file"
      multiple
      onchange={onUploadPicked}
      aria-hidden="true"
      tabindex="-1"
    />
  {/if}

  <!-- Folder-mode parity with FileInfoBody: same aggregate stats grid,
       file-kind counts, and code report a regular folder inspector
       renders. Subtree counts prefer the server-side InspectorPayload
       (authoritative for the whole workspace) and fall back to the
       loaded tree walk while the request is in flight. -->
  <div class="meta-grid">
    <span class="k">files</span>
    <span class="v">{subtree?.files ?? dirStats.files}</span>
    <span class="k">subdirectories</span>
    <span class="v">{subtree?.directories ?? dirStats.dirs}</span>
    <span class="k">size</span>
    <span class="v">{formatSize(subtree?.bytes ?? dirStats.bytes)}</span>
    <span class="k">last change</span>
    <span class="v">{formatMtime(dirStats.latest)}</span>
  </div>
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
              <button
                type="button"
                class="lang-name"
                title="open in graph (scoped to this language)"
                onclick={() => onLanguageClick(lang.name)}
              >{lang.name}</button>
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
    <div class="refs-loading">loading report...</div>
  {:else if reportError}
    <div class="refs-error">report unavailable: {reportError}</div>
  {/if}

  <!-- Contacts section, mirroring FileInfoBody's contact pill list.
       Renders in both variants whenever the workspace graph holds
       contact / mention nodes so the workspace root reads like any
       other folder inspector. -->
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

  {#if variant === "dashboard"}
  <!-- The Notes-directories config is Dashboard-only. The inspector
       variant drops it (the workspace root reads as a plain directory
       there); the Dashboard carries the full
       globalConfig/save/autosave plumbing. `.notes-dirs` adds an
       explicit divider above the heading so the COCOMO / Code content
       (or Contacts) above is visually separated from the
       Notes-directories config. -->
  <section class="refs notes-dirs">
    <h4>Workspaces</h4>
    <p class="hint">
      Your default workspace directory is where chan opens when launched
      without a specific one in mind. Leave empty to use the platform
      default (<code>~/Documents/Chan</code> on macOS,
      <code>$XDG_DATA_HOME/chan/default</code> on Linux).
    </p>
    <label class="field">
      <span>Default</span>
      <input
        bind:value={editedDefaultRoot}
        placeholder="(platform default)"
        spellcheck="false"
        autocomplete="off"
      />
    </label>
    {#if saveError}
      <div class="err-line">save failed: {saveError}</div>
    {/if}

    {#if globalConfig?.workspaces && globalConfig.workspaces.length > 0}
      <h5 class="recents-head">Recent</h5>
      <ul class="recents">
        {#each globalConfig.workspaces as u (u.path)}
          <li>
            <span class="recents-time">{formatLastSeen(u.last_seen_at)}</span>
            <span class="recents-name" title={u.path}>{displayPathLabel(u.path)}</span>
            <span class="recents-path mono" title={u.path}>{u.path}</span>
          </li>
        {/each}
      </ul>
      <p class="hint">
        Updated every time you open a directory. In-app open-from-list
        lands in a follow-up; for now use the menu's Open Directory.
      </p>
    {/if}
  </section>
  {/if}
</div>

<style>
  .info {
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  /* Workspace kind chip: black bg, white text. Sits alongside the
     existing doc / image / contact / view-only chips defined in
     FileInfoBody so the inspector's per-kind cue is consistent
     across selections. */
  .kind-chip {
    color: #fff;
    text-transform: uppercase;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    flex: 1;
    text-align: center;
  }
  .kind-chip.workspace {
    background: #000;
    color: #fff;
  }
  .title {
    margin: 0 0 0.5rem 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
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
  .meta-grid .v.path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  /* Spacing wrapper for the workspace-root action pill; InspectorActionPill
     owns the pill + dropdown styling itself. */
  .actions-section {
    margin: 0.6rem 0 0.2rem 0;
  }
  .file-picker {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }
  /* Desktop-download indicator (browser path stays null -> not
     rendered). Mirrors FileInfoBody's `.dl-*` chrome inside the
     inspector so the desktop webview, which has no native download
     manager, still shows progress + the saved path. */
  .dl-indicator {
    margin-top: 0.5rem;
    padding: 0.4rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-elev);
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .dl-indicator.err {
    border-color: var(--warn-text);
    color: var(--warn-text);
  }
  .dl-line {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .dl-indicator.err .dl-line { color: var(--warn-text); }
  .dl-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.4rem;
  }
  .dl-progress {
    height: 4px;
    border-radius: 2px;
    background: var(--border);
    overflow: hidden;
  }
  .dl-bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.15s linear;
  }
  .dl-bar.indeterminate {
    width: 40%;
    animation: dl-slide 1.1s ease-in-out infinite;
  }
  @keyframes dl-slide {
    0% { margin-left: -40%; }
    100% { margin-left: 100%; }
  }
  .dl-dismiss {
    flex-shrink: 0;
    background: transparent;
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    padding: 1px 8px;
    align-self: flex-start;
  }
  .dl-dismiss:hover { border-color: var(--btn-hover); }
  .refs { margin: 0.8rem 0 0 0; }
  .refs h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0 0 0.25rem 0;
  }
  /* Contacts pill list, mirroring FileInfoBody's `.refs ul` +
     `.ref.contact` so the workspace inspector's contacts read
     identically to a file inspector's. */
  .refs ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .refs li { margin: 0; }
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
  /* Contact rows: same block-button shape with a small person
     silhouette prefixed in --warn-text so a glance tells you the
     entry is a person rather than a generic doc. Icon matches the
     editor wiki pill + FileInfoBody contact rows. */
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
  .hint {
    color: var(--text-secondary);
    font-size: 11.5px;
    margin: 0 0 0.5rem 0;
  }
  .hint code {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    background: var(--bg-card);
    padding: 0 4px;
    border-radius: 3px;
  }
  .field {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 0.5rem;
    align-items: center;
    margin: 0.25rem 0;
  }
  .field > span { color: var(--text-secondary); font-size: 14px; }
  .field input {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 7px;
    font: inherit;
    font-size: 14px;
    outline: none;
    width: 100%;
  }
  .field input:focus { border-color: var(--link); }
  .err-line {
    color: var(--warn-text);
    font-size: 13px;
    margin: 0.25rem 0;
  }
  .recents-head {
    margin: 0.6rem 0 0.25rem 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .recents {
    list-style: none;
    padding: 0;
    margin: 0 0 0.4rem 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .recents li {
    display: grid;
    grid-template-columns: 12em auto 1fr;
    gap: 0.6rem;
    font-size: 13px;
    color: var(--text);
    align-items: baseline;
  }
  .recents-time {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .recents-name { color: var(--text); font-weight: 500; }
  .recents-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Folder-parity sections (file kinds + code report). Visual style
     mirrors FileInfoBody so the workspace inspector and a regular
     folder inspector read as one feature. */
  .compact-section { margin-top: 0.35rem; }
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
  /* A <button> so the language name routes to the Graph (scoped to
     this language). Strip default button chrome, left-align, add
     hover + focus affordance. Stays a grid cell at column 1. Mirrors
     FileInfoBody's `.lang-name`. */
  .lang-name {
    color: var(--text);
    word-break: break-word;
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    font: inherit;
    font-size: inherit;
    text-align: left;
    cursor: pointer;
  }
  .lang-name:hover { text-decoration: underline; }
  .lang-name:focus-visible {
    outline: 2px solid var(--link);
    outline-offset: 1px;
    border-radius: 2px;
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
  /* Divider between the Code/COCOMO (or Contacts) content above and
     the Notes-directories config below. Matches the dashed rule the
     COCOMO block uses so the dashboard inspector reads as cleanly
     sectioned. The `.refs` margin-top supplies the gap above the
     rule; padding-top spaces the heading below it. */
  .notes-dirs {
    padding-top: 0.7rem;
    border-top: 1px dashed var(--border);
  }
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
  .refs-loading,
  .refs-error {
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 0.6rem;
    font-style: italic;
  }
  .refs-error { color: var(--warn-text); font-style: normal; }
</style>
