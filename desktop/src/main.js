const { invoke } = window.__TAURI__.core;
const { open, ask, message } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;
const { check: checkForUpdate } = window.__TAURI__.updater;
const { relaunch } = window.__TAURI__.process;
const { openUrl } = window.__TAURI__.opener;

const main = document.getElementById('main');
const newBtn = document.getElementById('new-workspace');
const themeToggle = document.getElementById('theme-toggle');
const authBtn = document.getElementById('auth-btn');

/// Theme handling. Stored values:
///   - null  : follow OS via prefers-color-scheme (no data-theme attr)
///   - "dark": forced dark regardless of OS
///   - "light": forced light regardless of OS
/// Clicking the toggle flips between explicit dark and explicit light;
/// to return to "follow OS", clear localStorage.chanDesktopTheme by hand.
const THEME_KEY = 'chanDesktopTheme';
const osLight = window.matchMedia('(prefers-color-scheme: light)');

function effectiveTheme() {
  const saved = localStorage.getItem(THEME_KEY);
  if (saved === 'dark' || saved === 'light') return saved;
  return osLight.matches ? 'light' : 'dark';
}

function applyTheme() {
  const saved = localStorage.getItem(THEME_KEY);
  const root = document.documentElement;
  if (saved === 'dark' || saved === 'light') {
    root.setAttribute('data-theme', saved);
  } else {
    root.removeAttribute('data-theme');
  }
  // Mirror onto body so the toggle button can choose which icon to
  // render — CSS variables alone don't expose the active theme.
  document.body.classList.toggle('is-dark', effectiveTheme() === 'dark');
  document.body.classList.toggle('is-light', effectiveTheme() === 'light');
}

applyTheme();
osLight.addEventListener('change', applyTheme);
themeToggle.addEventListener('click', () => {
  const next = effectiveTheme() === 'dark' ? 'light' : 'dark';
  localStorage.setItem(THEME_KEY, next);
  applyTheme();
});

/// Sign-in state. The Sign In button mints a 30-day PAT via the
/// id.chan.app auth webview and stores it in the OS keychain; the
/// resulting Sign Out clears the local entry (server-side revoke
/// is a follow-up that needs the id_session cookie).
async function refreshAuth(status) {
  const s = status || (await invoke('auth_status'));
  if (s.is_signed_in) {
    authBtn.textContent = 'Sign out';
    authBtn.title = s.label
      ? `Signed in as ${s.label}` + (s.expires_at ? ` (expires ${s.expires_at})` : '')
      : 'Signed in to chan.app';
    authBtn.dataset.state = 'signed-in';
  } else {
    authBtn.textContent = 'Sign in';
    authBtn.title = 'Sign in to chan.app';
    authBtn.dataset.state = 'signed-out';
  }
}

authBtn.addEventListener('click', async () => {
  try {
    if (authBtn.dataset.state === 'signed-in') {
      await invoke('signout');
    } else {
      await invoke('open_signin');
    }
  } catch (err) {
    showError(err);
  }
  await refreshAuth();
});

refreshAuth();
listen('auth-changed', (e) => refreshAuth(e.payload));
listen('auth-error', (e) => {
  showError(typeof e.payload === 'string' ? e.payload : 'Sign-in failed');
  refreshAuth();
});

let homeDir = '';
/// True while a registry add/remove is running in the embedded
/// host. Add/remove and feature toggles run in-process (no
/// `chan` binary), but `boot()` can still take a moment on a large
/// workspace, so the launcher disables the relevant controls and shows
/// a progress banner while busy.
let chanBusy = false;
// Last rendered workspaces payload as a JSON string. The backend fires
// `serves-changed` / `registry-changed` whenever the chan registry
// is touched, which a running serve does often (timestamps, etc.).
// Re-running `main.innerHTML = ...` on every event causes the row
// to flicker. Skip the render when the payload hasn't changed.
let lastWorkspacesJson = '';

// `force` re-renders even when the workspace-list JSON is unchanged. The
// periodic / event-driven callers dedupe on the JSON to avoid flicker, but a
// user toggle must reconcile the DOM back to the true serve state even when
// the net registry JSON did not move (e.g. a native checkbox flip whose
// underlying on/off transition failed), so it forces a render.
async function refresh(force = false) {
  if (!homeDir) {
    try { homeDir = await invoke('home_dir'); } catch { homeDir = ''; }
  }
  const [workspaces, devservers] = await Promise.all([
    invoke('list_workspaces'),
    invoke('list_devservers'),
  ]);
  const json = JSON.stringify({ workspaces, devservers });
  if (force || json !== lastWorkspacesJson) {
    lastWorkspacesJson = json;
    render(workspaces, devservers);
  }
  return workspaces;
}

/// Render a workspace's filesystem path with the user's home folder
/// collapsed to a house glyph. Paths outside the home dir render
/// with a sibling computer glyph in front so the user has a visual
/// cue that this is somewhere on the machine but outside `$HOME`.
/// Returns an HTML string; caller injects into a clickable cell
/// that calls `reveal_in_finder` with the full path.
function renderPath(full) {
  if (homeDir && (full === homeDir || full.startsWith(homeDir + '/'))) {
    const rest = full.slice(homeDir.length).replace(/^\//, '');
    // Inline SVG house glyph keeps the rendering self-contained and
    // tints with currentColor for theme switches.
    const house = `<svg class="ic-home" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="home"><path d="M3 11l9-8 9 8"/><path d="M5 10v10h14V10"/></svg>`;
    if (!rest) return house;
    return `${house}<span class="path-sep">/</span>${escapeHtml(rest)}`;
  }
  // Symmetric computer-glyph branch. Matches the
  // home variant's 13x13 viewBox + currentColor stroke so theme
  // switches keep visual parity. There's no canonical "computer
  // root" to trim (unlike `$HOME`), so render the full path after
  // the glyph + separator.
  const computer = `<svg class="ic-computer" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="computer"><rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M9 20h6M12 16v4"/></svg>`;
  return `${computer}<span class="path-sep">/</span>${escapeHtml(full.replace(/^\//, ''))}`;
}

// Glyph for the WHERE column on a remote row: an arrow leaving a box
// (we connect out to a URL). Matches the ic-home / ic-computer style
// (13x13, currentColor, 1.8 stroke) so theme switches keep parity.
const ICON_OUTBOUND = `<svg class="ic-outbound" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="remote"><path d="M14 4h6v6"/><path d="M20 4l-9 9"/><path d="M19 13v6a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h6"/></svg>`;

// Glyph for a [DEVSERVER] group header: stacked server racks (a
// multi-workspace box). Matches the ic-outbound style (currentColor,
// 1.8 stroke) so theme switches keep parity.
const ICON_DEVSERVER = `<svg class="ic-devserver" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="devserver"><rect x="3" y="4" width="18" height="7" rx="1.5"/><rect x="3" y="13" width="18" height="7" rx="1.5"/><path d="M7 7.5h.01M7 16.5h.01"/></svg>`;

/// The WHERE cell, one renderer keyed on `kind`. Local reuses the
/// home/computer path glyph; a remote row leads with the remote glyph
/// plus the workspace label/URL.
function renderWhere(d) {
  if (d.kind === 'outbound') {
    const display = d.label || d.url || 'Remote workspace';
    return `${ICON_OUTBOUND}<span class="where-text">${escapeHtml(display)}</span>`;
  }
  return renderPath(d.path);
}

async function boot() {
  // No forced prompts on boot: an empty list sits on the "No
  // workspaces yet" empty state, and the standalone terminal window
  // (opened by the desktop on boot) is the thing that appears. A
  // workspace is opt-in via the empty-state New workspace button.
  await refresh();
}

function applyChanBusyState(payload) {
  chanBusy = !!(payload && payload.busy);
  newBtn.disabled = chanBusy;
  document.body.classList.toggle('chan-busy', chanBusy);

  let banner = document.getElementById('chan-busy-banner');
  if (!chanBusy) {
    if (banner) banner.remove();
    return;
  }
  if (!banner) {
    banner = document.createElement('div');
    banner.id = 'chan-busy-banner';
    banner.className = 'status-banner persistent';
    document.body.insertBefore(banner, document.body.firstChild);
  }
  const op = payload && payload.op === 'remove' ? 'Removing workspace' : 'Adding workspace';
  banner.textContent = `${op}...`;
}

/// The [New] workspace modal: one overlay carrying two choices, a
/// segmented switch swapping the body per choice:
///   - Local directory: a folder picker + Open (add_workspace).
///   - Remote: a URL + name form (add_outbound_workspace); we dial out
///     to a chan workspace already being served.
///   - Devserver: a HOST/PORT/SCRIPT form (add_devserver). The same form
///     doubles as an Edit form: pass `editDevserver` to pre-fill it from an
///     existing devserver (empty = New, pre-filled = Edit).
///
/// ESC / backdrop / [X] dismiss.
let activeNewDialog = null;

function showNewWorkspaceDialog(initialChoice = 'local', editDevserver = null) {
  // Singleton: a second [New] click just switches the open modal's
  // choice instead of stacking overlays.
  if (activeNewDialog) {
    activeNewDialog.select(initialChoice);
    return;
  }

  const overlay = document.createElement('div');
  overlay.className = 'nw-overlay';
  overlay.setAttribute('role', 'dialog');
  overlay.setAttribute('aria-modal', 'true');
  overlay.setAttribute('aria-labelledby', 'nw-title');

  const dialog = document.createElement('div');
  dialog.className = 'nw-dialog';
  overlay.appendChild(dialog);

  const header = document.createElement('div');
  header.className = 'nw-header';
  const title = document.createElement('h2');
  title.id = 'nw-title';
  title.textContent = 'New workspace';
  const closeBtn = document.createElement('button');
  closeBtn.className = 'nw-close';
  closeBtn.type = 'button';
  closeBtn.setAttribute('aria-label', 'Close');
  closeBtn.textContent = '×';
  header.appendChild(title);
  header.appendChild(closeBtn);
  dialog.appendChild(header);

  const choices = document.createElement('div');
  choices.className = 'nw-choices';
  choices.setAttribute('role', 'radiogroup');
  choices.setAttribute('aria-label', 'New workspace type');
  const CHOICES = [
    ['local', 'Local directory'],
    ['outbound', 'Remote'],
    ['devserver', 'Devserver'],
  ];
  const choiceButtons = {};
  for (const [key, label] of CHOICES) {
    const b = document.createElement('button');
    b.className = 'nw-choice';
    b.type = 'button';
    b.setAttribute('role', 'radio');
    b.dataset.choice = key;
    b.textContent = label;
    b.addEventListener('click', () => select(key));
    choices.appendChild(b);
    choiceButtons[key] = b;
  }
  dialog.appendChild(choices);

  const body = document.createElement('div');
  body.className = 'nw-body';
  dialog.appendChild(body);

  const footer = document.createElement('div');
  footer.className = 'nw-footer';
  dialog.appendChild(footer);

  document.body.appendChild(overlay);

  let choice = initialChoice;
  // Local-choice state: the picked folder (null until chosen).
  let localPath = null;

  function close() {
    document.removeEventListener('keydown', onKey);
    overlay.remove();
    activeNewDialog = null;
  }
  function onKey(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }
  overlay.addEventListener('click', (e) => { if (e.target === overlay) close(); });
  closeBtn.addEventListener('click', close);
  document.addEventListener('keydown', onKey);

  function select(next) {
    choice = next;
    for (const [key, b] of Object.entries(choiceButtons)) {
      const on = key === choice;
      b.classList.toggle('on', on);
      b.setAttribute('aria-checked', on ? 'true' : 'false');
    }
    renderBody();
  }

  function renderBody() {
    body.innerHTML = '';
    footer.innerHTML = '';
    if (choice === 'local') renderLocal();
    else if (choice === 'devserver') renderDevserver(editDevserver);
    else renderOutbound();
  }

  // ---- Local directory -------------------------------------------------
  function renderLocal() {
    if (!localPath) {
      body.innerHTML =
        `<p class="nw-intro">A local folder with your markdown files (a git repository, or any directory).</p>`;
      const choose = document.createElement('button');
      choose.className = 'btn primary';
      choose.type = 'button';
      choose.textContent = 'Choose folder...';
      choose.addEventListener('click', chooseLocalFolder);
      body.appendChild(choose);
      choose.focus();
      return;
    }
    // Folder chosen: confirm the path, then register + open. The first-boot
    // pre-flight (the workspace scan, the index / seed progress, and the
    // Semantic / Reports layer toggles) lives in chan's SPA
    // (PreflightOverlay.svelte). The desktop must NOT run its own
    // scan dialog here: it would duplicate and race the SPA boot surface
    // (a double-dialog). add_workspace defaults both optional layers
    // off; the SPA's onboarding card turns them on after boot.
    body.innerHTML = `
      <p class="nw-intro">This folder will be registered as a chan workspace:</p>
      <p class="preflight-path"></p>`;
    body.querySelector('.preflight-path').textContent = localPath;

    const back = document.createElement('button');
    back.className = 'btn';
    back.type = 'button';
    back.textContent = 'Back';
    back.addEventListener('click', () => { localPath = null; renderBody(); });

    const openWs = document.createElement('button');
    openWs.className = 'btn primary';
    openWs.type = 'button';
    openWs.textContent = 'Open';
    openWs.addEventListener('click', async () => {
      try {
        await invoke('add_workspace', { path: localPath });
      } catch (e) {
        showError(e);
        return;
      }
      close();
      await refresh();
    });
    footer.appendChild(back);
    footer.appendChild(openWs);
    openWs.focus();
  }

  async function chooseLocalFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Select a folder containing markdown files',
    });
    if (typeof selected !== 'string' || !selected.length) return;
    localPath = selected;
    renderBody();
  }

  // ---- Remote (we connect out to a URL) --------------------------------
  function renderOutbound() {
    body.innerHTML = `
      <p class="nw-intro">Connect to a chan workspace already being served at a URL (we dial out to it).</p>
      <p class="nw-muted">Run chan where your repo lives, then paste the URL it prints above:</p>
      <pre class="snippet" data-copy="chan serve ./path/to/repo" title="click to copy">chan serve ./path/to/repo</pre>
      <p class="nw-muted">Or reach it over an SSH local forward:</p>
      <pre class="snippet" data-copy="ssh user@host -L 8787:localhost:8787 chan serve ./path/to/repo" title="click to copy">ssh user@host -L 8787:localhost:8787 chan serve ./path/to/repo</pre>
      <div class="nw-row">
        <label class="nw-url-field">URL
          <input id="nw-outbound-url" type="url" autocomplete="off" spellcheck="false"
                 placeholder="http://127.0.0.1:4000/?t=..."/>
        </label>
        <label>Name
          <input id="nw-outbound-label" type="text" maxlength="120" autocomplete="off"/>
        </label>
      </div>`;
    const attach = document.createElement('button');
    attach.className = 'btn primary';
    attach.type = 'button';
    attach.textContent = 'Attach URL';
    attach.addEventListener('click', submitOutbound);
    footer.appendChild(attach);
    const urlInput = body.querySelector('#nw-outbound-url');
    for (const inp of [urlInput, body.querySelector('#nw-outbound-label')]) {
      inp.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') { e.preventDefault(); attach.click(); }
      });
    }
    wireSnippetCopy(body);
    urlInput.focus();
  }

  async function submitOutbound() {
    const urlInput = body.querySelector('#nw-outbound-url');
    const labelInput = body.querySelector('#nw-outbound-label');
    const url = (urlInput && urlInput.value || '').trim();
    const label = (labelInput && labelInput.value || '').trim();
    if (!url) {
      if (urlInput) urlInput.focus();
      showError('Remote URL is required.');
      return;
    }
    try {
      await invoke('add_outbound_workspace', { url, label });
    } catch (e) {
      showError(e);
      return;
    }
    close();
    await refresh();
  }

  // ---- Devserver (multi-workspace aggregator we dial out to) -----------
  // `existing` is null for New (add a devserver) or a Devserver object for
  // Edit (pre-fill host/port/script/label, "Save changes"). The entry point
  // that supplies `existing` lives on the devserver row's dropdown.
  function renderDevserver(existing = null) {
    const ds = existing || {};
    const editing = !!existing;
    body.innerHTML = `
      <p class="nw-intro">Connect to a chan <em>devserver</em>, a headless box serving many workspaces at once. The desktop dials <b>host:port</b>; its workspaces appear in their own launcher group.</p>
      <div class="nw-row">
        <label class="nw-host-field">Host
          <input id="nw-ds-host" type="text" autocomplete="off" spellcheck="false" placeholder="127.0.0.1" value="${escapeAttr(ds.host || '')}"/>
        </label>
        <label class="nw-port-field">Port
          <input id="nw-ds-port" type="number" min="1" max="65535" autocomplete="off" placeholder="8787" value="${escapeAttr(ds.port != null ? String(ds.port) : '')}"/>
        </label>
        <label class="nw-name-field">Name
          <input id="nw-ds-label" type="text" maxlength="120" autocomplete="off" placeholder="optional" value="${escapeAttr(ds.label || '')}"/>
        </label>
      </div>
      <label class="nw-script-field">Connect command <span class="nw-muted">(optional; runs in a control terminal, e.g. an SSH local forward)</span>
        <textarea id="nw-ds-script" class="nw-script" rows="2" autocomplete="off" spellcheck="false" placeholder="ssh user@box -L 8787:localhost:8787 chan devserver --bind 127.0.0.1 --port 8787">${escapeHtml(ds.script || '')}</textarea>
      </label>
      <p class="nw-muted">On the box, run a devserver, then register workspaces into it with <code>chan serve</code>:</p>
      <pre class="snippet" data-copy="chan devserver --bind 127.0.0.1 --port 8787" title="click to copy">chan devserver --bind 127.0.0.1 --port 8787</pre>`;
    const submit = document.createElement('button');
    submit.className = 'btn primary';
    submit.type = 'button';
    submit.textContent = editing ? 'Save changes' : 'Add devserver';
    submit.addEventListener('click', () => submitDevserver(existing));
    footer.appendChild(submit);
    // Enter submits from the single-line fields (not the multi-line
    // script textarea, where Enter inserts a newline).
    for (const sel of ['#nw-ds-host', '#nw-ds-port', '#nw-ds-label']) {
      const inp = body.querySelector(sel);
      if (inp) {
        inp.addEventListener('keydown', (e) => {
          if (e.key === 'Enter') { e.preventDefault(); submit.click(); }
        });
      }
    }
    wireSnippetCopy(body);
    body.querySelector('#nw-ds-host').focus();
  }

  async function submitDevserver(existing = null) {
    const host = (body.querySelector('#nw-ds-host')?.value || '').trim();
    const portRaw = (body.querySelector('#nw-ds-port')?.value || '').trim();
    const script = (body.querySelector('#nw-ds-script')?.value || '').trim();
    const label = (body.querySelector('#nw-ds-label')?.value || '').trim();
    if (!host) {
      body.querySelector('#nw-ds-host')?.focus();
      showError('Devserver host is required.');
      return;
    }
    const port = Number.parseInt(portRaw, 10);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      body.querySelector('#nw-ds-port')?.focus();
      showError('Devserver port must be a number between 1 and 65535.');
      return;
    }
    if (existing) {
      // Edit mode persists through `update_devserver`, wired alongside the
      // devserver row's Edit entry. Until that entry exists no caller passes
      // `existing`, so this branch only guards the parameterized form.
      showError('Editing a devserver is not available yet.');
      return;
    }
    try {
      await invoke('add_devserver', { host, port, script, label });
    } catch (e) {
      showError(e);
      return;
    }
    close();
    await refresh();
  }

  // The singleton handle: a second [New] click switches the open
  // modal's choice instead of stacking overlays.
  activeNewDialog = { select };

  select(choice);
}

function render(workspaces, devservers = []) {
  const chanCommandDisabledAttr = chanBusy ? 'disabled' : '';
  const localRuntimeDisabledAttr = chanBusy ? 'disabled' : '';

  if (!workspaces.length && !devservers.length) {
    main.innerHTML = `
      <div class="empty">
        <h2>No workspaces yet</h2>
        <p>A <em>workspace</em> is a local folder with your markdown files.
           Pick one to get started.</p>
        <button class="btn primary" id="empty-pick" ${chanCommandDisabledAttr}>New workspace</button>
      </div>`;
    document.getElementById('empty-pick').onclick = () => showNewWorkspaceDialog('local');
    return;
  }

  const rows = workspaces.map((d) => {
    const hasUrl = !!d.url;
    const urlAttr = escapeAttr(d.url || '');
    const dotClass = hasUrl ? 'conn-dot on' : 'conn-dot';
    if (d.kind === 'outbound') {
      return `
      <tr data-kind="outbound"
          data-outbound-id="${escapeAttr(d.id || '')}"
          data-url="${urlAttr}">
        <td><span class="${dotClass}" title="Attached URL"></span></td>
        <td class="path-cell remote-cell where-cell" title="${escapeAttr(d.url || '')}">${renderWhere(d)}</td>
        <td>
          <div class="row-actions">
            ${renderOpenSplit({
              hasUrl,
              includeForget: true,
              forgetDisabledAttr: '',
              forgetLabel: 'Forget URL',
            })}
          </div>
        </td>
      </tr>`;
    }
    return `
    <tr data-path="${escapeAttr(d.path)}" data-url="${urlAttr}">
      <td>
        <label class="switch">
          <input type="checkbox" data-act="toggle-on" ${d.on ? 'checked' : ''} ${localRuntimeDisabledAttr}/>
          <span class="slider"></span>
        </label>
      </td>
      <td class="path-cell where-cell" data-act="reveal" title="${escapeAttr(d.path)} — click to open in Finder">${renderWhere(d)}</td>
      <td>
        <div class="row-actions">
          ${renderOpenSplit({ hasUrl, includeForget: true, forgetDisabledAttr: chanCommandDisabledAttr })}
        </div>
      </td>
    </tr>`;
  }).join('');

  const localTable = workspaces.length
    ? `
    <table class="workspaces">
      <thead>
        <tr>
          <th style="width:60px">On</th>
          <th>Where</th>
          <th style="width:150px"></th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>`
    : `<p class="nw-muted ws-group-pending">No local workspaces yet.</p>`;

  // Category headers appear only once there's something to group (at
  // least one devserver). With no devservers the launcher stays the
  // plain, header-less table it has always been.
  const grouped = devservers.length > 0;
  let html = '';
  if (grouped) {
    html += `<h3 class="ws-group ws-group-local"><span class="ws-group-host">Local</span></h3>`;
  }
  html += localTable;
  for (const ds of devservers) html += renderDevserverSection(ds);
  main.innerHTML = html;

  bindRowEvents();
  if (grouped) {
    bindDevserverSectionEvents();
    // Fill the connected sections' workspace rows right after layout; the
    // periodic interval keeps them fresh thereafter.
    pollDevserverWorkspaces().catch(() => {});
  }
}

/// One `[DEVSERVER {host}]` launcher section: a header (name + endpoint +
/// Connect/Disconnect + Forget) over the section body. While disconnected
/// the body is a placeholder; once connected it lists the devserver's live
/// workspace rows, filled by `pollDevserverWorkspaces`.
function renderDevserverSection(ds) {
  const name = (ds.label && ds.label.trim()) || ds.host || 'devserver';
  const endpoint = `${ds.host}:${ds.port}`;
  const connectAct = ds.connected ? 'disconnect-devserver' : 'connect-devserver';
  const connectLabel = ds.connected ? 'Disconnect' : 'Connect';
  const body = ds.connected
    ? `<div class="ds-workspaces"><p class="nw-muted ws-group-pending">Loading workspaces...</p></div>`
    : `<p class="nw-muted ws-group-pending">Not connected.</p>`;
  return `
    <section class="ws-group-block" data-devserver-id="${escapeAttr(ds.id || '')}">
      <h3 class="ws-group">
        ${ICON_DEVSERVER}
        <span class="ws-group-host">${escapeHtml(name)}</span>
        <span class="ws-group-sub">${escapeHtml(endpoint)}</span>
        <span class="ws-group-actions">
          <button class="btn ws-group-btn" data-act="${connectAct}" type="button">${connectLabel}</button>
          <button class="btn danger ws-group-btn" data-act="forget-devserver" type="button"
                  title="Forget this devserver">Forget</button>
        </span>
      </h3>
      ${body}
    </section>`;
}

/// One devserver workspace row: an outbound-style row whose Open button
/// hands the pre-assembled tenant URL to `open_devserver_workspace`.
function renderDevserverWorkspaceRow(ws) {
  return `
    <tr data-ds-prefix="${escapeAttr(ws.prefix)}" data-url="${escapeAttr(ws.url)}">
      <td><span class="conn-dot on" title="Workspace"></span></td>
      <td class="path-cell where-cell remote-cell" title="${escapeAttr(ws.path)}">${ICON_OUTBOUND}<span class="where-text">${escapeHtml(ws.label || ws.path)}</span></td>
      <td><div class="row-actions"><button class="btn primary" data-act="open-ds-ws">Open</button></div></td>
    </tr>`;
}

/// Wire the Connect/Disconnect and Forget buttons on each `[DEVSERVER]`
/// section. Connect acquires the devserver token and opens a terminal;
/// Disconnect drops the connection; Forget removes the persisted devserver.
function bindDevserverSectionEvents() {
  main.querySelectorAll('section[data-devserver-id]').forEach((section) => {
    const id = section.dataset.devserverId || '';
    const forget = section.querySelector('[data-act="forget-devserver"]');
    if (forget) {
      forget.addEventListener('click', async () => {
        if (!id) return;
        try {
          await invoke('remove_devserver', { id });
        } catch (e) {
          showError(e);
          return;
        }
        await refresh();
      });
    }
    const connect = section.querySelector('[data-act="connect-devserver"]');
    if (connect) {
      connect.addEventListener('click', async () => {
        if (!id) return;
        connect.disabled = true;
        connect.textContent = 'Connecting...';
        try {
          await invoke('connect_devserver', { id });
        } catch (e) {
          showError(e);
        }
        await refresh();
      });
    }
    const disconnect = section.querySelector('[data-act="disconnect-devserver"]');
    if (disconnect) {
      disconnect.addEventListener('click', async () => {
        if (!id) return;
        try {
          await invoke('disconnect_devserver', { id });
        } catch (e) {
          showError(e);
        }
        await refresh();
      });
    }
  });
}

// Per-devserver last-rendered workspace-rows JSON so the periodic poll only
// touches the DOM when the list actually changed (no flicker at idle).
const lastDevserverRowsJson = {};

/// Fill each connected `[DEVSERVER]` section with its live workspace rows.
/// Runs after a render and on a periodic interval; updates only the section
/// bodies (not the whole launcher), so it never disturbs the local table.
async function pollDevserverWorkspaces() {
  const sections = [...main.querySelectorAll('section[data-devserver-id]')];
  if (!sections.length) return;
  let devservers;
  try {
    devservers = await invoke('list_devservers');
  } catch {
    return;
  }
  const byId = Object.fromEntries(devservers.map((d) => [d.id, d]));
  for (const section of sections) {
    const id = section.dataset.devserverId;
    const ds = byId[id];
    const container = section.querySelector('.ds-workspaces');
    if (!ds || !ds.connected || !container) continue;
    let rows;
    try {
      rows = await invoke('list_devserver_workspaces', { id });
    } catch (e) {
      container.innerHTML = `<p class="nw-muted ws-group-pending">${escapeHtml(String(e && e.message ? e.message : e))}</p>`;
      continue;
    }
    const json = JSON.stringify(rows);
    if (lastDevserverRowsJson[id] === json) continue;
    lastDevserverRowsJson[id] = json;
    container.innerHTML = rows.length
      ? `<table class="workspaces"><tbody>${rows.map(renderDevserverWorkspaceRow).join('')}</tbody></table>`
      : `<p class="nw-muted ws-group-pending">No workspaces on this devserver yet.</p>`;
    container.querySelectorAll('tr[data-ds-prefix]').forEach((tr) => {
      const open = tr.querySelector('[data-act="open-ds-ws"]');
      if (open) {
        open.addEventListener('click', async () => {
          try {
            await invoke('open_devserver_workspace', { prefix: tr.dataset.dsPrefix, url: tr.dataset.url });
          } catch (e) {
            showError(e);
          }
        });
      }
    });
  }
}

/// Per-row "Open" split button: primary action opens the workspace in
/// an in-app webview; caret reveals a menu with "Open in Browser"
/// and (for local workspaces only) "Forget Workspace". The primary
/// Open is always enabled: for an off local workspace the launch
/// handler turns it on first (remote rows only exist while their URL
/// is live, so they never render off). "Open in Browser" needs the
/// live URL and stays gated by `hasUrl`; Forget stays enabled
/// regardless of URL state since it just removes the registry entry.
function renderOpenSplit({ hasUrl, includeForget, forgetDisabledAttr, forgetLabel = 'Forget Workspace' }) {
  const browserDisabled = hasUrl ? '' : 'disabled';
  const forgetDisabled = forgetDisabledAttr || '';
  const caretDisabled = hasUrl || (includeForget && !forgetDisabled) ? '' : 'disabled';
  const forgetItem = includeForget
    ? `<li><button class="menu-item" data-act="remove" role="menuitem" ${forgetDisabled}>${escapeHtml(forgetLabel)}</button></li>`
    : '';
  return `
    <div class="split-btn">
      <button class="btn primary" data-act="launch">Open</button>
      <button class="btn primary split-caret" data-act="menu-toggle"
              aria-haspopup="true" aria-expanded="false" aria-label="More actions" ${caretDisabled}>
        <svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M2 4l4 4 4-4"/></svg>
      </button>
      <ul class="split-menu" hidden role="menu">
        <li><button class="menu-item" data-act="open-browser" role="menuitem" ${browserDisabled}>Open in Browser</button></li>
        ${forgetItem}
      </ul>
    </div>`;
}

function bindRowEvents() {
  main.querySelectorAll('tr[data-kind="outbound"]').forEach((tr) => {
    const id = tr.dataset.outboundId || '';
    const launch = tr.querySelector('[data-act="launch"]');
    if (launch) {
      launch.addEventListener('click', async () => {
        if (!id) return;
        try {
          await invoke('open_outbound_workspace', { id });
        } catch (e) {
          showError(e);
        }
      });
    }
    const forget = tr.querySelector('[data-act="remove"]');
    if (forget) {
      forget.addEventListener('click', async () => {
        if (!id) return;
        closeAllSplitMenus();
        try {
          await invoke('remove_outbound_workspace', { id });
        } catch (err) {
          showError(err);
        }
        await refresh();
      });
    }
    bindSplitMenu(tr);
  });

  main.querySelectorAll('tr[data-path]').forEach((tr) => {
    const path = tr.dataset.path;

    tr.querySelector('[data-act="toggle-on"]').addEventListener('change', async (e) => {
      const toggle = e.target;
      // Serve start/stop is not instant: stop removes the runtime but a
      // background indexer / in-flight request can hold the workspace flock
      // for a beat, and start awaits a fresh open. The native checkbox flips
      // the instant it is clicked, so without locking the control a second
      // click races the still-transitioning server -> WorkspaceLocked
      // ("open in another chan process") and the row sticks ON with Open
      // disabled. Disable the toggle for the whole transition so it can't be
      // re-clicked mid-flight, then force a re-render from the TRUE serve
      // state (bypassing the list-JSON dedupe) so the toggle + Open reconcile
      // to reality on every outcome - including a failed re-enable, which
      // then cleanly reverts the toggle instead of stranding it.
      toggle.disabled = true;
      try {
        await invoke('set_workspace_on', { path, on: toggle.checked });
      } catch (err) {
        // Turn-on failures get the modal with the real reason; the
        // refresh below flips the pill back to off, the dialog says
        // why. Turn-off failures keep the lightweight banner.
        if (toggle.checked) {
          showTurnOnFailureDialog(err);
        } else {
          showError(err);
        }
      }
      await refresh(true);
    });

    tr.querySelector('[data-act="launch"]').addEventListener('click', async (e) => {
      // In-app Tauri webview; each click adds another window so
      // multi-window per workspace is the default.
      const btn = e.currentTarget;
      if (!tr.dataset.url) {
        // Workspace is off: turn it on first, then open. The button
        // stays disabled for the whole transition so a double-click
        // can't race the still-starting serve (same hazard as the
        // pill toggle above); refresh(true) re-renders the row from
        // the true serve state on both outcomes.
        btn.disabled = true;
        try {
          await invoke('set_workspace_on', { path, on: true });
        } catch (err) {
          showTurnOnFailureDialog(err);
          btn.disabled = false;
          await refresh(true);
          return;
        }
        await refresh(true);
      }
      try {
        await invoke('open_local_workspace', { path });
      } catch (err) {
        showError(err);
      }
    });

    tr.querySelector('[data-act="reveal"]').addEventListener('click', async () => {
      try {
        await invoke('reveal_in_finder', { path });
      } catch (err) {
        showError(err);
      }
    });

    const forget = tr.querySelector('[data-act="remove"]');
    if (forget) {
      // "Forget Workspace" removes the workspace entry from the chan
      // registry. Files on disk are untouched; the user can re-add
      // the folder later via New workspace.
      forget.addEventListener('click', async () => {
        closeAllSplitMenus();
        try {
          await invoke('remove_workspace', { path });
        } catch (err) {
          showError(err);
        }
        await refresh();
      });
    }

    bindSplitMenu(tr);
  });
}

/// Wire the split-button caret + dropdown items shared between
/// local and remote rows. The "Open in Browser" item delegates
/// to tauri-plugin-opener with the URL stored on the row's
/// `data-url` attribute (populated by `render`).
function bindSplitMenu(tr) {
  const caret = tr.querySelector('[data-act="menu-toggle"]');
  const menu = tr.querySelector('.split-menu');
  if (caret && menu) {
    caret.addEventListener('click', (e) => {
      e.stopPropagation();
      const willOpen = menu.hasAttribute('hidden');
      closeAllSplitMenus();
      if (willOpen) {
        menu.classList.remove('open-up');
        menu.removeAttribute('hidden');
        caret.setAttribute('aria-expanded', 'true');
        // Flip the menu above the caret when opening downward would
        // clip it below the scroll container's bottom edge (the
        // last-row case). Measured after un-hiding so offsetHeight is
        // real.
        const scroller = document.getElementById('main');
        const limit = scroller
          ? scroller.getBoundingClientRect().bottom
          : window.innerHeight;
        if (caret.getBoundingClientRect().bottom + 4 + menu.offsetHeight > limit) {
          menu.classList.add('open-up');
        }
      }
    });
  }
  const openInBrowser = tr.querySelector('[data-act="open-browser"]');
  if (openInBrowser) {
    openInBrowser.addEventListener('click', async () => {
      closeAllSplitMenus();
      const url = tr.dataset.url || '';
      if (!url) return;
      try {
        await openUrl(url);
      } catch (e) {
        showError(e);
      }
    });
  }
}

function closeAllSplitMenus() {
  document.querySelectorAll('.split-menu:not([hidden])').forEach((m) => {
    m.setAttribute('hidden', '');
    m.classList.remove('open-up');
  });
  document.querySelectorAll('[data-act="menu-toggle"][aria-expanded="true"]').forEach((b) => {
    b.setAttribute('aria-expanded', 'false');
  });
}

// Click anywhere outside an open split menu closes it. Caret
// clicks call stopPropagation so they don't trigger this.
document.addEventListener('click', (e) => {
  if (!e.target.closest('.split-menu')) closeAllSplitMenus();
});
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') closeAllSplitMenus();
});

function showError(e) {
  const msg = typeof e === 'string' ? e : (e && e.message) || String(e);
  // Simple inline banner above the table; replaced on next render.
  const banner = document.createElement('div');
  banner.className = 'error-banner';
  banner.textContent = msg;
  main.prepend(banner);
  setTimeout(() => banner.remove(), 5000);
}

/// Modal for a failed workspace turn-on (pill toggle or Open's
/// auto-turn-on). The Rust error string is already user-friendly —
/// notably "open in another chan process" for the lock case — and the
/// caller's refresh(true) reconciles the pill back to off behind the
/// overlay, so the dialog only has to say why. Resolves on dismiss
/// (OK / Escape / backdrop click); the keydown listener is removed in
/// close() so repeated failures don't stack document listeners.
function showTurnOnFailureDialog(reason) {
  return new Promise((resolve) => {
    const msg =
      typeof reason === 'string' ? reason : (reason && reason.message) || String(reason);

    const overlay = document.createElement('div');
    overlay.className = 'preflight-overlay';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('aria-labelledby', 'turn-on-failure-title');

    const dialog = document.createElement('div');
    dialog.className = 'preflight-dialog';

    const title = document.createElement('h2');
    title.id = 'turn-on-failure-title';
    title.textContent = 'Cannot turn on workspace';
    dialog.appendChild(title);

    const body = document.createElement('p');
    body.className = 'preflight-intro';
    body.textContent = msg;
    dialog.appendChild(body);

    const buttons = document.createElement('div');
    buttons.className = 'preflight-buttons';
    const okBtn = document.createElement('button');
    okBtn.className = 'btn primary';
    okBtn.type = 'button';
    okBtn.textContent = 'OK';
    buttons.appendChild(okBtn);
    dialog.appendChild(buttons);

    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    function close() {
      document.removeEventListener('keydown', onKey);
      overlay.remove();
      resolve();
    }
    function onKey(e) {
      if (e.key === 'Escape') {
        e.preventDefault();
        close();
      }
    }
    okBtn.addEventListener('click', close);
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) close();
    });
    document.addEventListener('keydown', onKey);
    okBtn.focus();
  });
}

// Click-to-copy wiring for every `.snippet[data-copy]` under `scope`.
// Used by the Remote (outbound) choice body.
function wireSnippetCopy(scope) {
  scope.querySelectorAll('.snippet[data-copy]').forEach((node) => {
    node.addEventListener('click', async () => {
      try {
        await navigator.clipboard.writeText(node.dataset.copy);
        const old = node.textContent;
        node.textContent = 'Copied';
        setTimeout(() => { node.textContent = old; }, 1200);
      } catch {
        // Clipboard denied; nothing to do.
      }
    });
  });
}

function escapeHtml(s) {
  return String(s)
    .replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}
function escapeAttr(s) {
  return escapeHtml(s).replaceAll('"', '&quot;');
}

newBtn.addEventListener('click', () => showNewWorkspaceDialog('local'));

// Fire-and-forget update check. Runs once per process launch.
// Endpoint / pubkey live in tauri.conf.json under `plugins.updater`.
// Failure modes (offline, endpoint 4xx/5xx, malformed manifest)
// are swallowed: an air-gapped launch should not pop a dialog
// about a failed update probe.
async function maybeOfferUpdate() {
  let update;
  try {
    update = await checkForUpdate();
  } catch (e) {
    console.warn('update check failed:', e);
    return;
  }
  if (!update) return;
  // Plain text only. The release notes (`update.body`) arrive as GitHub
  // markdown; a native dialog renders it literally (asterisks, raw URL),
  // so we drop it and show a single changelog link instead. The native
  // dialog cannot make the URL clickable, but a bare compare link is
  // selectable and self-explanatory. `currentVersion` is the installed
  // build; `version` is the offered one.
  const changelog = update.currentVersion
    ? `https://github.com/fiorix/chan/compare/v${update.currentVersion}...v${update.version}`
    : `https://github.com/fiorix/chan/releases/tag/v${update.version}`;
  const accepted = await ask(
    `A new version of Chan Desktop is available: ${update.version}.\n\n` +
    `Changelog: ${changelog}\n\n` +
    'Install and restart now?',
    { title: 'Chan Desktop update', okLabel: 'Install', cancelLabel: 'Later', kind: 'info' }
  );
  if (!accepted) return;
  try {
    await update.downloadAndInstall();
    await relaunch();
  } catch (e) {
    showError(e);
  }
}

// Re-render whenever the chan registry changes from anywhere
// (the desktop itself, the chan CLI, or another tool editing the
// TOML directly), or when a serve starts / discovers its URL / exits.
listen('registry-changed', () => { refresh().catch(showError); });
listen('serves-changed', () => { refresh().catch(showError); });
listen('system-notice', (e) => {
  const p = e.payload || {};
  showError(typeof p.message === 'string' ? p.message : 'Chan Desktop notice');
});
listen('chan-busy', (e) => {
  applyChanBusyState(e.payload || {});
  lastWorkspacesJson = '';
  refresh().catch(showError);
});

boot().catch(showError);
maybeOfferUpdate().catch((e) => console.warn('update flow error:', e));

// Keep each connected devserver's workspace rows fresh. The poll only
// touches the DOM when a list changed, so an idle launcher stays still; the
// cadence mirrors the in-SPA indexing carousel.
setInterval(() => { pollDevserverWorkspaces().catch(() => {}); }, 3000);
