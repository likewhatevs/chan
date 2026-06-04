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

let booted = false;
let homeDir = '';
/// True while a registry add/remove is running in the embedded
/// host. Add/remove and feature toggles run in-process now (no
/// `chan` binary), but `boot()` can still take a moment on a large
/// workspace, so the launcher disables the relevant controls and shows
/// a progress banner while busy.
let chanBusy = false;
let defaultWorkspacePromptDismissed = false;
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
  const workspaces = await invoke('list_workspaces');
  const json = JSON.stringify(workspaces);
  if (force || json !== lastWorkspacesJson) {
    lastWorkspacesJson = json;
    render(workspaces);
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
  // `fullstack-53`: symmetric computer-glyph branch. Matches the
  // home variant's 13x13 viewBox + currentColor stroke so theme
  // switches keep visual parity. There's no canonical "computer
  // root" to trim (unlike `$HOME`), so render the full path after
  // the glyph + separator.
  const computer = `<svg class="ic-computer" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="computer"><rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M9 20h6M12 16v4"/></svg>`;
  return `${computer}<span class="path-sep">/</span>${escapeHtml(full.replace(/^\//, ''))}`;
}

// `new-team-1`: directional glyphs for the WHERE column on remote rows.
// out = arrow leaving a box (we connect OUT to a URL); in = arrow
// arriving into a tray (we LISTEN for an incoming connection). Match
// the ic-home / ic-computer style (13x13, currentColor, 1.8 stroke) so
// theme switches keep parity.
const ICON_OUTBOUND = `<svg class="ic-outbound" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="outbound"><path d="M14 4h6v6"/><path d="M20 4l-9 9"/><path d="M19 13v6a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h6"/></svg>`;
const ICON_INBOUND = `<svg class="ic-inbound" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="inbound"><path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3"/><path d="M12 4v11"/><path d="M7 10l5 5 5-5"/></svg>`;

/// The WHERE cell, one renderer keyed on `kind`. Local reuses the
/// home/computer path glyph; remote rows lead with a directional icon
/// (outbound = we connect to a URL; inbound = we listen) plus a muted
/// direction caption, which is the launcher's INBOUND vs OUTBOUND
/// indication now that the ON-column text tags are gone.
function renderWhere(d) {
  if (d.kind === 'tunneled') {
    return `${ICON_INBOUND}<span class="where-text">${escapeHtml(d.label || '')}</span><span class="where-dir">inbound</span>`;
  }
  if (d.kind === 'outbound') {
    const display = d.label || d.url || 'Remote workspace';
    return `${ICON_OUTBOUND}<span class="where-text">${escapeHtml(display)}</span><span class="where-dir">outbound</span>`;
  }
  return renderPath(d.path);
}

async function boot() {
  let workspaces = await refresh();
  await maybePromptDefaultWorkspace();
  workspaces = await refresh();
  if (!booted && workspaces.length === 0) {
    booted = true;
    showNewWorkspaceDialog('local');
  } else {
    booted = true;
  }
}

async function maybePromptDefaultWorkspace() {
  if (defaultWorkspacePromptDismissed) return;
  let status;
  try {
    status = await invoke('default_workspace_status');
  } catch (e) {
    showError(e);
    return;
  }
  if (!status) return;
  if (status.needs_factory_reset) {
    const confirmed = await showMissingDefaultWorkspaceDialog(status);
    if (!confirmed) {
      defaultWorkspacePromptDismissed = true;
      return;
    }
    try {
      await invoke('factory_reset_default_workspace');
    } catch (e) {
      showError(e);
      return;
    }
    await refresh();
    return;
  }
  if (!status.needs_prompt) return;
  const choice = await showDefaultWorkspaceDialog(status);
  if (!choice.accepted) {
    defaultWorkspacePromptDismissed = true;
    return;
  }
  try {
    if (choice.mode === 'create') {
      await invoke('create_default_workspace');
    } else {
      await invoke('choose_default_workspace', { path: choice.path });
    }
  } catch (e) {
    showError(e);
    return;
  }
  await refresh();
}

function showMissingDefaultWorkspaceDialog(status) {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'preflight-overlay';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('aria-labelledby', 'missing-default-title');

    const dialog = document.createElement('div');
    dialog.className = 'preflight-dialog default-workspace-dialog';

    const title = document.createElement('h2');
    title.id = 'missing-default-title';
    title.textContent = 'Default Chan workspace missing';
    dialog.appendChild(title);

    const intro = document.createElement('p');
    intro.className = 'preflight-intro';
    intro.textContent =
      'The default Chan workspace path no longer exists. To continue with a fresh default workspace, confirm a factory reset of chan metadata on this machine.';
    dialog.appendChild(intro);

    const pathEl = document.createElement('p');
    pathEl.className = 'preflight-path';
    pathEl.textContent =
      status.missing_default_root || status.default_root || status.suggested_root || '';
    dialog.appendChild(pathEl);

    const detail = document.createElement('p');
    detail.className = 'preflight-intro';
    detail.textContent =
      'Factory reset clears the chan registry, indexes, sessions, tokens, drafts, and generated reports. It does not delete note folders outside chan metadata. A new Documents/Chan workspace will be created and seeded with the manual.';
    dialog.appendChild(detail);

    const buttons = document.createElement('div');
    buttons.className = 'preflight-buttons';
    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn';
    cancelBtn.type = 'button';
    cancelBtn.textContent = 'Cancel';
    const resetBtn = document.createElement('button');
    resetBtn.className = 'btn danger';
    resetBtn.type = 'button';
    resetBtn.textContent = 'Factory reset';
    buttons.appendChild(cancelBtn);
    buttons.appendChild(resetBtn);
    dialog.appendChild(buttons);

    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    function close(confirmed) {
      document.removeEventListener('keydown', onKey);
      overlay.remove();
      resolve(confirmed);
    }
    function onKey(e) {
      if (e.key === 'Escape') {
        e.preventDefault();
        close(false);
      }
    }
    cancelBtn.addEventListener('click', () => close(false));
    resetBtn.addEventListener('click', () => close(true));
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) close(false);
    });
    document.addEventListener('keydown', onKey);
    cancelBtn.focus();
  });
}

function showDefaultWorkspaceDialog(status) {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'preflight-overlay';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('aria-labelledby', 'default-workspace-title');

    const dialog = document.createElement('div');
    dialog.className = 'preflight-dialog default-workspace-dialog';

    const title = document.createElement('h2');
    title.id = 'default-workspace-title';
    title.textContent = 'Choose default workspace';
    dialog.appendChild(title);

    const intro = document.createElement('p');
    intro.className = 'preflight-intro';
    intro.textContent =
      'Pick the workspace Chan should open by default, or create a new Chan workspace under Documents.';
    dialog.appendChild(intro);

    const form = document.createElement('div');
    form.className = 'default-workspace-options';

    const workspaces = Array.isArray(status.workspaces) ? status.workspaces : [];
    workspaces.forEach((workspace, index) => {
      const label = document.createElement('label');
      label.className = 'default-workspace-option';
      const input = document.createElement('input');
      input.type = 'radio';
      input.name = 'default-workspace-choice';
      input.value = workspace.path || '';
      input.dataset.mode = 'existing';
      input.checked = index === 0;
      const span = document.createElement('span');
      span.className = 'default-workspace-path';
      span.textContent = workspace.path || '';
      label.appendChild(input);
      label.appendChild(span);
      form.appendChild(label);
    });

    const createLabel = document.createElement('label');
    createLabel.className = 'default-workspace-option';
    const createInput = document.createElement('input');
    createInput.type = 'radio';
    createInput.name = 'default-workspace-choice';
    createInput.value = status.suggested_root || '';
    createInput.dataset.mode = 'create';
    createInput.checked = workspaces.length === 0;
    const createText = document.createElement('span');
    createText.className = 'default-workspace-path';
    createText.textContent = `Create ${status.suggested_root || 'Documents/Chan'}`;
    createLabel.appendChild(createInput);
    createLabel.appendChild(createText);
    form.appendChild(createLabel);

    dialog.appendChild(form);

    const buttons = document.createElement('div');
    buttons.className = 'preflight-buttons';
    const laterBtn = document.createElement('button');
    laterBtn.className = 'btn';
    laterBtn.type = 'button';
    laterBtn.textContent = 'Later';
    const continueBtn = document.createElement('button');
    continueBtn.className = 'btn primary';
    continueBtn.type = 'button';
    continueBtn.textContent = 'Continue';
    buttons.appendChild(laterBtn);
    buttons.appendChild(continueBtn);
    dialog.appendChild(buttons);

    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    function close(result) {
      document.removeEventListener('keydown', onKey);
      overlay.remove();
      resolve(result);
    }
    function selectedChoice() {
      const selected = dialog.querySelector('input[name="default-workspace-choice"]:checked');
      if (!selected) return { accepted: false };
      return {
        accepted: true,
        mode: selected.dataset.mode || 'existing',
        path: selected.value || '',
      };
    }
    function onKey(e) {
      if (e.key === 'Escape') {
        e.preventDefault();
        close({ accepted: false });
      } else if (e.key === 'Enter') {
        e.preventDefault();
        close(selectedChoice());
      }
    }
    laterBtn.addEventListener('click', () => close({ accepted: false }));
    continueBtn.addEventListener('click', () => close(selectedChoice()));
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) close({ accepted: false });
    });
    document.addEventListener('keydown', onKey);
    continueBtn.focus();
  });
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

/// Local|Tunnel snippet mode for the [New] -> Inbound listening state.
/// `local` means "the remote chan serve runs on the same machine as
/// this desktop, no SSH needed". `tunnel` means "chan serve lives on a
/// remote host and an SSH reverse-forward bridges to this desktop's
/// loopback port". Persisted in localStorage; the backend doesn't care
/// since both snippets are pre-formatted.
const TUNNEL_MODE_KEY = 'chanDesktopTunnelMode';
function tunnelMode() {
  const v = localStorage.getItem(TUNNEL_MODE_KEY);
  return v === 'local' ? 'local' : 'tunnel';
}
function setTunnelMode(mode) {
  localStorage.setItem(TUNNEL_MODE_KEY, mode === 'local' ? 'local' : 'tunnel');
}

/// The [New] workspace modal. Replaces the old [Open workspace] +
/// [Attach] entry points with one overlay carrying three choices, each
/// a different body (modeled on showPreflightDialog for the scaffold and
/// on the Team Work dialog's real-estate toggle for the segmented switch
/// that swaps the body per choice):
///   - Local directory: a folder picker + in-body preflight scan + the
///     two add-time feature toggles + Open (add_workspace).
///   - Remote outbound: a URL + name form (add_outbound_workspace); we
///     dial out to the remote.
///   - Remote inbound: a port-listen form, then a listening state with
///     the chan serve snippet (tunnel_start / tunnel_stop); we listen.
///
/// ESC / backdrop / [X] dismiss. Dismiss NEVER stops a live inbound
/// listener: it lives in the Rust supervisor and tunnel_status is the
/// source of truth, so reopening New -> Inbound shows it still
/// listening (matches the old always-visible Attach panel).
let activeNewDialog = null;

function showNewWorkspaceDialog(initialChoice = 'local') {
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
    ['outbound', 'Remote outbound'],
    ['inbound', 'Remote inbound'],
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
    else if (choice === 'outbound') renderOutbound();
    else renderInbound();
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
    // Semantic / Reports layer toggles) moved to chan's SPA
    // (PreflightOverlay.svelte) in phase-17. The desktop must NOT run its own
    // scan dialog here: it duplicates and races the SPA boot surface (the
    // double-dialog @@Alex hit). add_workspace defaults both optional layers
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

  // ---- Remote outbound (we connect to a URL) ---------------------------
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

  // ---- Remote inbound (we listen for an incoming connection) ----------
  async function renderInbound() {
    body.innerHTML = `<p class="nw-intro">Reading listener status…</p>`;
    let status;
    try {
      status = await invoke('tunnel_status');
    } catch (e) {
      showError(e);
      return;
    }
    // The user may have switched choices while tunnel_status was in
    // flight; only paint if Inbound is still the active choice.
    if (choice !== 'inbound') return;
    if (status.listening && status.port != null) {
      renderInboundListening(status);
    } else {
      renderInboundForm(status);
    }
  }

  function renderInboundForm(status) {
    body.innerHTML = `
      <p class="nw-intro">Listen for incoming connections on a configurable port, or use 0 to let the OS pick one. Then connect to it:</p>
      <pre class="snippet" data-copy="chan serve ./path/to/repo --tunnel-url={chan-desktop-listener}" title="click to copy">chan serve ./path/to/repo --tunnel-url={chan-desktop-listener}</pre>
      <div class="nw-row">
        <label>Port
          <input id="nw-tunnel-port" type="number" min="0" max="65535" placeholder="auto"
                 value="${status.preferred_port ? status.preferred_port : ''}"/>
        </label>
        <label>Label
          <input id="nw-tunnel-label" type="text" maxlength="64"
                 value="${escapeAttr(status.preferred_label || '')}"/>
        </label>
        <label>Workspace
          <input id="nw-tunnel-workspace" type="text" maxlength="32"
                 value="${escapeAttr(status.preferred_workspace || '')}"/>
        </label>
      </div>
      <p class="nw-hint">Port 0 / empty lets the OS pick. Label appears as the first URL segment. Workspace name is lowercase ASCII + hyphens.</p>`;
    const start = document.createElement('button');
    start.className = 'btn primary';
    start.type = 'button';
    start.textContent = 'Start listening';
    start.addEventListener('click', async () => {
      const rawPort = (body.querySelector('#nw-tunnel-port').value || '').trim();
      const preferred = rawPort === '' ? 0 : Math.max(0, Math.min(65535, Number(rawPort) | 0));
      const label = (body.querySelector('#nw-tunnel-label').value || '').trim();
      const workspace = (body.querySelector('#nw-tunnel-workspace').value || '').trim();
      try {
        await invoke('tunnel_start', { preferredPort: preferred, label, workspace });
      } catch (e) {
        showError(e);
        return;
      }
      renderInbound();
    });
    footer.appendChild(start);
    wireSnippetCopy(body);
  }

  function renderInboundListening(status) {
    const ssh = status.ssh_snippet || '';
    const cmd = status.chan_serve_snippet || '';
    const mode = tunnelMode();
    const isTunnel = mode === 'tunnel';
    const sshBlock = isTunnel
      ? `<p class="nw-muted">SSH from this machine to the remote with a reverse forward:</p>
         <pre class="snippet" data-copy="${escapeAttr(ssh)}" title="click to copy">${escapeHtml(ssh)}</pre>
         <p class="nw-muted">Then on the remote run:</p>`
      : `<p class="nw-muted">On this machine, run:</p>`;
    body.innerHTML = `
      <div class="nw-listening-head">
        <strong>Listening on 127.0.0.1:${status.port}</strong>
        <div class="seg-toggle" role="tablist" aria-label="Where will chan serve run?">
          <button class="seg ${mode === 'local' ? 'on' : ''}" data-mode="local" role="tab" aria-selected="${mode === 'local'}">Local</button>
          <button class="seg ${mode === 'tunnel' ? 'on' : ''}" data-mode="tunnel" role="tab" aria-selected="${mode === 'tunnel'}">Tunnel</button>
        </div>
      </div>
      ${sshBlock}
      <pre class="snippet" data-copy="${escapeAttr(cmd)}" title="click to copy">${escapeHtml(cmd)}</pre>
      <p class="nw-muted">Connected workspaces appear in the launcher list and open automatically.</p>`;
    body.querySelectorAll('.seg-toggle .seg').forEach((btn) => {
      btn.addEventListener('click', () => {
        setTunnelMode(btn.dataset.mode);
        renderInboundListening(status);
      });
    });
    wireSnippetCopy(body);
    const stop = document.createElement('button');
    stop.className = 'btn danger';
    stop.type = 'button';
    stop.textContent = 'Stop';
    stop.addEventListener('click', async () => {
      try {
        await invoke('tunnel_stop');
      } catch (e) {
        showError(e);
        return;
      }
      await refresh();
      renderInbound();
    });
    const done = document.createElement('button');
    done.className = 'btn';
    done.type = 'button';
    done.textContent = 'Done';
    done.addEventListener('click', close);
    footer.appendChild(stop);
    footer.appendChild(done);
  }

  // Re-render hook so tunnel events can refresh a live inbound body
  // while the modal is open (e.g. a remote connecting, or the listener
  // stopping from elsewhere).
  activeNewDialog = {
    select,
    refreshInbound() { if (choice === 'inbound') renderInbound(); },
  };

  select(choice);
}

function render(workspaces) {
  const chanCommandDisabledAttr = chanBusy ? 'disabled' : '';
  const localRuntimeDisabledAttr = chanBusy ? 'disabled' : '';

  if (!workspaces.length) {
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
    if (d.kind === 'tunneled') {
      // Tunneled row: no On toggle (the remote owns the lifecycle),
      // no Path (it's a remote folder), no Forget (the remote drops
      // the registration by shutting `chan serve` down). The label
      // is the bearer token the remote chose; we show it verbatim
      // and trust the user's naming convention.
      const tip = [
        d.peer_addr ? `peer ${d.peer_addr}` : null,
        d.public ? 'public' : null,
        d.connected_at ? `connected ${d.connected_at}` : null,
      ].filter(Boolean).join(' · ');
      return `
      <tr data-kind="tunneled"
          data-tunnel-label="${escapeAttr(d.label || '')}"
          data-tunnel-workspace="${escapeAttr(d.workspace || '')}"
          data-url="${urlAttr}">
        <td><span class="${dotClass}" title="${escapeAttr(tip || 'inbound listener')}"></span></td>
        <td class="path-cell remote-cell where-cell" title="${escapeAttr(d.url || '')}">${renderWhere(d)}</td>
        <td>
          <div class="row-actions">
            ${renderOpenSplit({ hasUrl, includeForget: false, forgetDisabledAttr: 'disabled' })}
          </div>
        </td>
      </tr>`;
    }
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

  main.innerHTML = `
    <table class="workspaces">
      <thead>
        <tr>
          <th style="width:60px">On</th>
          <th>Where</th>
          <th style="width:150px"></th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>`;

  bindRowEvents();
}

/// Per-row "Open" split button: primary action opens the workspace in
/// an in-app webview; caret reveals a menu with "Open in Browser"
/// and (for local workspaces only) "Forget Workspace". The primary + caret
/// are both gated by `hasUrl` so a workspace that isn't running can't
/// be opened; Forget stays enabled regardless of URL state since
/// it just removes the registry entry.
function renderOpenSplit({ hasUrl, includeForget, forgetDisabledAttr, forgetLabel = 'Forget Workspace' }) {
  const openDisabled = hasUrl ? '' : 'disabled';
  const forgetDisabled = forgetDisabledAttr || '';
  const caretDisabled = hasUrl || (includeForget && !forgetDisabled) ? '' : 'disabled';
  const forgetItem = includeForget
    ? `<li><button class="menu-item" data-act="remove" role="menuitem" ${forgetDisabled}>${escapeHtml(forgetLabel)}</button></li>`
    : '';
  return `
    <div class="split-btn">
      <button class="btn primary" data-act="launch" ${openDisabled}>Open</button>
      <button class="btn primary split-caret" data-act="menu-toggle"
              aria-haspopup="true" aria-expanded="false" aria-label="More actions" ${caretDisabled}>
        <svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M2 4l4 4 4-4"/></svg>
      </button>
      <ul class="split-menu" hidden role="menu">
        <li><button class="menu-item" data-act="open-browser" role="menuitem" ${openDisabled}>Open in Browser</button></li>
        ${forgetItem}
      </ul>
    </div>`;
}

function bindRowEvents() {
  // Tunneled rows: Open + Open in Browser only. No toggle / reveal
  // / Forget handlers because there is no desktop-side lifecycle
  // to control — the remote `chan serve` owns it.
  main.querySelectorAll('tr[data-kind="tunneled"]').forEach((tr) => {
    const launch = tr.querySelector('[data-act="launch"]');
    if (launch) {
      launch.addEventListener('click', async () => {
        // Open reuses the same in-app Tauri webview the supervisor
        // opens on first registration. Going to the system browser
        // is reachable through the dropdown's "Open in Browser".
        const label = tr.dataset.tunnelLabel || '';
        const workspace = tr.dataset.tunnelWorkspace || '';
        if (!label || !workspace) return;
        try {
          await invoke('open_tunneled_workspace', { label, workspace });
        } catch (e) {
          showError(e);
        }
      });
    }
    bindSplitMenu(tr);
  });

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
        showError(err);
      }
      await refresh(true);
    });

    tr.querySelector('[data-act="launch"]').addEventListener('click', async () => {
      // In-app Tauri webview; each click adds another window so
      // multi-window per workspace is the default.
      try {
        await invoke('open_local_workspace', { path });
      } catch (e) {
        showError(e);
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
      // the folder later via New workspace. Tunneled workspaces have no
      // Forget — the remote `chan serve` owns that lifecycle.
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
/// local and tunneled rows. The "Open in Browser" item delegates
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
        menu.removeAttribute('hidden');
        caret.setAttribute('aria-expanded', 'true');
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

// Click-to-copy wiring for every `.snippet[data-copy]` under `scope`.
// Shared by the outbound + inbound-form + inbound-listening code blocks.
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
  const accepted = await ask(
    `A new version of Chan Desktop is available: ${update.version}.\n\n` +
    (update.body ? update.body + '\n\n' : '') +
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

// `tunneled-workspace-ready` is informational on this side: the Rust
// supervisor already opened the in-app webview window the moment
// the per-tenant listener bound. We just refresh the workspace table
// so the new row shows up alongside its URL, and refresh the [New]
// modal's inbound body if it happens to be open.
listen('tunneled-workspace-ready', () => {
  refresh().catch(showError);
  if (activeNewDialog) activeNewDialog.refreshInbound();
});

// The listener state changed (started / stopped, possibly from
// elsewhere). Refresh the [New] modal's inbound body if it's open and
// showing inbound; the launcher list refreshes via serves-changed.
listen('tunnel-state-changed', () => {
  if (activeNewDialog) activeNewDialog.refreshInbound();
});

boot().catch(showError);
maybeOfferUpdate().catch((e) => console.warn('update flow error:', e));
