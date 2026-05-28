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
  import { tree, workspace } from "../state/store.svelte";

  /// `fullstack-73`: optional "Graph from here" callback. Consumers
  /// that host this body alongside an existing inspector convention
  /// pass it; surfaces that don't (legacy callers) leave it unset
  /// and the button doesn't render. The action's semantic differs
  /// per consumer — FileBrowserSurface SPAWNS a new Graph tab while
  /// GraphPanel's own inspector RE-SCOPES the current tab — so the
  /// consumer wires the function and this body is callback-agnostic.
  let {
    onSetAsScope,
  }: { onSetAsScope?: () => void } = $props();

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

  /// COCOMO formatting helpers; identical shape to FileInfoBody so the
  /// dir-mode and workspace-root inspectors format the same way.
  function fmtMonths(n: number): string {
    if (!Number.isFinite(n)) return "—";
    return n >= 10 ? `${Math.round(n)} mo` : `${n.toFixed(1)} mo`;
  }
  function fmtDevs(n: number): string {
    if (!Number.isFinite(n)) return "—";
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

  {#if onSetAsScope}
    <!-- `fullstack-73`: parity with FileInfoBody so every inspector
         surface offers the same "Graph from here" affordance. The
         hosting surface decides whether the click spawns a new
         Graph tab (file browser) or re-scopes the current one
         (Graph inspector). -->
    <button class="open" onclick={onSetAsScope}>Graph from here</button>
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

  <section class="refs">
    <h4>Notes directories</h4>
    <p class="hint">
      Your default notes directory is where chan opens when launched
      without a specific one in mind. Leave empty to use the platform
      default (<code>~/Documents/Chan</code> on macOS,
      <code>$XDG_DATA_HOME/chan/default</code> on Linux,
      <code>%USERPROFILE%\Documents\Chan</code> on Windows).
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
  /* `fullstack-73`: mirrors the FileInfoBody `.open` styling so the
     "Graph from here" affordance reads consistently across every
     inspector body. */
  .open {
    width: 100%;
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 0;
    cursor: pointer;
    font: inherit;
    margin-top: 0.6rem;
  }
  .open:hover { border-color: var(--btn-hover); }
  .refs { margin: 0.8rem 0 0 0; }
  .refs h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0 0 0.25rem 0;
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
  .refs-loading,
  .refs-error {
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 0.6rem;
    font-style: italic;
  }
  .refs-error { color: var(--warn-text); font-style: normal; }
</style>
