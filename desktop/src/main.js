const { invoke } = window.__TAURI__.core;
const { open, ask, message } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;
const { check: checkForUpdate } = window.__TAURI__.updater;
const { relaunch } = window.__TAURI__.process;
const { openUrl } = window.__TAURI__.opener;

const main = document.getElementById('main');
const openBtn = document.getElementById('open-drive');
const themeToggle = document.getElementById('theme-toggle');
const authBtn = document.getElementById('auth-btn');
const tunnelBtn = document.getElementById('tunnel-btn');

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
/// drive, so the launcher disables the relevant controls and shows
/// a progress banner while busy.
let chanBusy = false;
let defaultDrivePromptDismissed = false;
// Last rendered drives payload as a JSON string. The backend fires
// `serves-changed` / `registry-changed` whenever the chan registry
// is touched, which a running serve does often (timestamps, etc.).
// Re-running `main.innerHTML = ...` on every event causes the row
// to flicker. Skip the render when the payload hasn't changed.
let lastDrivesJson = '';

async function refresh() {
  if (!homeDir) {
    try { homeDir = await invoke('home_dir'); } catch { homeDir = ''; }
  }
  const drives = await invoke('list_workspaces');
  const json = JSON.stringify(drives);
  if (json !== lastDrivesJson) {
    lastDrivesJson = json;
    render(drives);
  }
  return drives;
}

/// Render a drive's filesystem path with the user's home folder
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

async function boot() {
  let drives = await refresh();
  await maybePromptDefaultDrive();
  drives = await refresh();
  if (!booted && drives.length === 0) {
    booted = true;
    await pickAndAdd();
  } else {
    booted = true;
  }
}

async function maybePromptDefaultDrive() {
  if (defaultDrivePromptDismissed) return;
  let status;
  try {
    status = await invoke('default_drive_status');
  } catch (e) {
    showError(e);
    return;
  }
  if (!status) return;
  if (status.needs_factory_reset) {
    const confirmed = await showMissingDefaultDriveDialog(status);
    if (!confirmed) {
      defaultDrivePromptDismissed = true;
      return;
    }
    try {
      await invoke('factory_reset_default_drive');
    } catch (e) {
      showError(e);
      return;
    }
    await refresh();
    return;
  }
  if (!status.needs_prompt) return;
  const choice = await showDefaultDriveDialog(status);
  if (!choice.accepted) {
    defaultDrivePromptDismissed = true;
    return;
  }
  try {
    if (choice.mode === 'create') {
      await invoke('create_default_drive');
    } else {
      await invoke('choose_default_drive', { path: choice.path });
    }
  } catch (e) {
    showError(e);
    return;
  }
  await refresh();
}

function showMissingDefaultDriveDialog(status) {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'preflight-overlay';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('aria-labelledby', 'missing-default-title');

    const dialog = document.createElement('div');
    dialog.className = 'preflight-dialog default-drive-dialog';

    const title = document.createElement('h2');
    title.id = 'missing-default-title';
    title.textContent = 'Default Chan drive missing';
    dialog.appendChild(title);

    const intro = document.createElement('p');
    intro.className = 'preflight-intro';
    intro.textContent =
      'The default Chan drive path no longer exists. To continue with a fresh default drive, confirm a factory reset of chan metadata on this machine.';
    dialog.appendChild(intro);

    const pathEl = document.createElement('p');
    pathEl.className = 'preflight-path';
    pathEl.textContent =
      status.missing_default_root || status.default_root || status.suggested_root || '';
    dialog.appendChild(pathEl);

    const detail = document.createElement('p');
    detail.className = 'preflight-intro';
    detail.textContent =
      'Factory reset clears the chan registry, indexes, sessions, tokens, drafts, and generated reports. It does not delete note folders outside chan metadata. A new Documents/Chan drive will be created and seeded with the manual.';
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

function showDefaultDriveDialog(status) {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'preflight-overlay';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('aria-labelledby', 'default-drive-title');

    const dialog = document.createElement('div');
    dialog.className = 'preflight-dialog default-drive-dialog';

    const title = document.createElement('h2');
    title.id = 'default-drive-title';
    title.textContent = 'Choose default drive';
    dialog.appendChild(title);

    const intro = document.createElement('p');
    intro.className = 'preflight-intro';
    intro.textContent =
      'Pick the drive Chan should open by default, or create a new Chan drive under Documents.';
    dialog.appendChild(intro);

    const form = document.createElement('div');
    form.className = 'default-drive-options';

    const drives = Array.isArray(status.drives) ? status.drives : [];
    drives.forEach((drive, index) => {
      const label = document.createElement('label');
      label.className = 'default-drive-option';
      const input = document.createElement('input');
      input.type = 'radio';
      input.name = 'default-drive-choice';
      input.value = drive.path || '';
      input.dataset.mode = 'existing';
      input.checked = index === 0;
      const span = document.createElement('span');
      span.className = 'default-drive-path';
      span.textContent = drive.path || '';
      label.appendChild(input);
      label.appendChild(span);
      form.appendChild(label);
    });

    const createLabel = document.createElement('label');
    createLabel.className = 'default-drive-option';
    const createInput = document.createElement('input');
    createInput.type = 'radio';
    createInput.name = 'default-drive-choice';
    createInput.value = status.suggested_root || '';
    createInput.dataset.mode = 'create';
    createInput.checked = drives.length === 0;
    const createText = document.createElement('span');
    createText.className = 'default-drive-path';
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
      const selected = dialog.querySelector('input[name="default-drive-choice"]:checked');
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
  openBtn.disabled = chanBusy;
  tunnelBtn.disabled = chanBusy;
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
  const op = payload && payload.op === 'remove' ? 'Removing drive' : 'Adding drive';
  banner.textContent = `${op}...`;
}

async function pickAndAdd() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Select a folder containing markdown files',
  });
  if (typeof selected !== 'string' || !selected.length) return;
  // `fullstack-b-28b` slice iii: interpose the pre-flight modal
  // between the directory picker and add_drive so the user
  // chooses BGE + reports BEFORE chan-drive's BOOT process runs.
  // Cancel exits without any chan-side side effect — the folder
  // wasn't registered yet, so closing the modal is a clean
  // back-out.
  const choice = await showPreflightDialog(selected);
  if (!choice.accepted) return;
  try {
    await invoke('add_drive', {
      path: selected,
      features: choice.features,
    });
  } catch (e) {
    showError(e);
    return;
  }
  await refresh();
}

/// `fullstack-b-28b` slice iii: pre-flight modal. Round-2-plan
/// §"UI surface" requires a load-bearing explanatory paragraph
/// above the toggles so users understand the baseline before
/// they choose what to layer on. The two toggles default OFF;
/// Open passes the chosen state through to `add_drive` which
/// forwards `--semantic-search` / `--reports` to `chan add`,
/// so chan-drive's BOOT process picks up the choice on the
/// first open.
///
/// Backdrop click + Escape cancel; Open button gets initial
/// focus + Enter triggers it.
function showPreflightDialog(path) {
  return new Promise((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'preflight-overlay';
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('aria-labelledby', 'preflight-title');

    const dialog = document.createElement('div');
    dialog.className = 'preflight-dialog';

    const title = document.createElement('h2');
    title.id = 'preflight-title';
    title.textContent = 'Open drive';
    dialog.appendChild(title);

    const intro = document.createElement('p');
    intro.className = 'preflight-intro';
    intro.textContent = `This folder will be registered as a chan drive:`;
    dialog.appendChild(intro);

    const pathEl = document.createElement('p');
    pathEl.className = 'preflight-path';
    pathEl.textContent = path;
    dialog.appendChild(pathEl);

    // `fullstack-b-28b` slice iv: report rows populated by
    // `compute_drive_preflight` IPC. Renders a "Scanning..."
    // placeholder while the walk runs so the modal opens fast
    // even on a large drive; resolved facts replace the row
    // contents in-place.
    const reportEl = document.createElement('div');
    reportEl.className = 'preflight-report';
    reportEl.setAttribute('aria-busy', 'true');
    reportEl.textContent = 'Scanning drive…';
    dialog.appendChild(reportEl);

    const baseline = document.createElement('p');
    baseline.className = 'preflight-baseline';
    baseline.textContent =
      "Chan will walk this drive, read every markdown file, and build a documentation graph from the wiki-links between them. This graph plus BM25 keyword search is the minimum needed to operate — it can't be disabled.";
    dialog.appendChild(baseline);

    const layered = document.createElement('p');
    layered.className = 'preflight-layered';
    layered.textContent =
      'Two optional layers can be enabled on top. Both default off and drop their per-drive data when disabled (the shared model file stays).';
    dialog.appendChild(layered);

    const togglesWrap = document.createElement('div');
    togglesWrap.className = 'preflight-toggles';

    const bgeRow = document.createElement('label');
    bgeRow.className = 'preflight-toggle';
    const bgeBox = document.createElement('input');
    bgeBox.type = 'checkbox';
    bgeBox.dataset.feat = 'bge';
    bgeRow.appendChild(bgeBox);
    const bgeLabel = document.createElement('span');
    bgeLabel.className = 'preflight-toggle-label';
    bgeLabel.innerHTML =
      '<strong>Semantic search</strong>' +
      '<span class="preflight-toggle-hint">Adds dense-vector embeddings for find-by-meaning queries. Needs the BGE-small model (~63 MB, downloaded once + shared across drives) and produces per-drive vector data.</span>';
    bgeRow.appendChild(bgeLabel);
    togglesWrap.appendChild(bgeRow);

    const reportsRow = document.createElement('label');
    reportsRow.className = 'preflight-toggle';
    const reportsBox = document.createElement('input');
    reportsBox.type = 'checkbox';
    reportsBox.dataset.feat = 'reports';
    reportsRow.appendChild(reportsBox);
    const reportsLabel = document.createElement('span');
    reportsLabel.className = 'preflight-toggle-label';
    reportsLabel.innerHTML =
      '<strong>Reports</strong>' +
      '<span class="preflight-toggle-hint">Runs code analysis on every file — language detection (tokei), source-lines-of-code counts per file + per-language roll-ups, and a Basic COCOMO estimate on top. Maintained incrementally from filesystem events. Per-drive.</span>';
    reportsRow.appendChild(reportsLabel);
    togglesWrap.appendChild(reportsRow);

    dialog.appendChild(togglesWrap);

    const footer = document.createElement('p');
    footer.className = 'preflight-footer';
    footer.textContent =
      'Both layers can be enabled later from the drive row or Settings.';
    dialog.appendChild(footer);

    const buttons = document.createElement('div');
    buttons.className = 'preflight-buttons';

    const cancelBtn = document.createElement('button');
    cancelBtn.className = 'btn';
    cancelBtn.type = 'button';
    cancelBtn.textContent = 'Cancel';

    const openBtn = document.createElement('button');
    openBtn.className = 'btn primary';
    openBtn.type = 'button';
    openBtn.textContent = 'Open';

    buttons.appendChild(cancelBtn);
    buttons.appendChild(openBtn);
    dialog.appendChild(buttons);

    overlay.appendChild(dialog);
    document.body.appendChild(overlay);

    function close(result) {
      document.removeEventListener('keydown', onKey);
      overlay.remove();
      resolve(result);
    }
    function snapshot() {
      return {
        accepted: true,
        features: { bge: bgeBox.checked, reports: reportsBox.checked },
      };
    }
    function onKey(e) {
      if (e.key === 'Escape') {
        e.preventDefault();
        close({ accepted: false, features: { bge: false, reports: false } });
      } else if (e.key === 'Enter' && document.activeElement === openBtn) {
        e.preventDefault();
        close(snapshot());
      }
    }

    cancelBtn.addEventListener('click', () =>
      close({ accepted: false, features: { bge: false, reports: false } }),
    );
    openBtn.addEventListener('click', () => close(snapshot()));
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay)
        close({ accepted: false, features: { bge: false, reports: false } });
    });
    document.addEventListener('keydown', onKey);

    openBtn.focus();

    // `fullstack-b-28b` slice iv: kick off the pre-flight walk
    // in parallel with the modal mount so the user can read
    // the explanatory copy while the report is filling in. The
    // dialog stays usable (Open/Cancel still respond) even if
    // the IPC takes the full cap (5s).
    invoke('compute_drive_preflight', { path })
      .then((report) => renderPreflightReport(reportEl, report))
      .catch((err) => {
        reportEl.removeAttribute('aria-busy');
        reportEl.classList.add('preflight-report-error');
        reportEl.textContent = `Couldn't scan drive: ${(err && err.message) || String(err)}`;
      });
  });
}

/// `fullstack-b-28b` slice iv: replace the "Scanning…" placeholder
/// with the resolved report rows. Each row carries one fact the
/// user needs to confirm "this is the folder I meant" + "I know
/// what I'm committing to" before chan-drive's BOOT runs.
function renderPreflightReport(host, report) {
  host.removeAttribute('aria-busy');
  host.textContent = '';

  if (report.already_registered) {
    const warn = document.createElement('p');
    warn.className = 'preflight-warn';
    warn.textContent =
      'This folder is already a registered chan drive. Opening it from the launcher row is the safer path.';
    host.appendChild(warn);
  }
  if (!report.writable) {
    const warn = document.createElement('p');
    warn.className = 'preflight-warn';
    warn.textContent =
      'Read-only mount: chan can still index this drive but you will not be able to create or edit notes from chan.';
    host.appendChild(warn);
  }

  const rows = document.createElement('dl');
  rows.className = 'preflight-report-rows';

  const filesLabel = report.truncated
    ? `${report.file_count.toLocaleString()}+ (scan capped)`
    : report.file_count.toLocaleString();
  appendPreflightRow(rows, 'Files', filesLabel);
  appendPreflightRow(
    rows,
    'Markdown',
    report.markdown_count.toLocaleString(),
  );
  appendPreflightRow(rows, 'Size', formatPreflightBytes(report.size_bytes));

  const mediaParts = [];
  if (report.image_count) mediaParts.push(`${report.image_count.toLocaleString()} images`);
  if (report.audio_count) mediaParts.push(`${report.audio_count.toLocaleString()} audio`);
  if (report.video_count) mediaParts.push(`${report.video_count.toLocaleString()} video`);
  appendPreflightRow(
    rows,
    'Media',
    mediaParts.length ? mediaParts.join(' · ') : 'none',
  );

  if (report.scm) {
    appendPreflightRow(rows, 'Source control', report.scm);
  }

  host.appendChild(rows);
}

function appendPreflightRow(parent, label, value) {
  const dt = document.createElement('dt');
  dt.textContent = label;
  const dd = document.createElement('dd');
  dd.textContent = value;
  parent.appendChild(dt);
  parent.appendChild(dd);
}

/// Human-friendly byte formatter. Caps at 999 G so the modal
/// never spills past a reasonable column width.
function formatPreflightBytes(bytes) {
  const units = ['B', 'KB', 'MB', 'GB'];
  let n = bytes;
  let unit = 0;
  while (n >= 1024 && unit < units.length - 1) {
    n = n / 1024;
    unit += 1;
  }
  const formatted = unit === 0 ? `${n}` : n.toFixed(n >= 10 ? 0 : 1);
  return `${formatted} ${units[unit]}`;
}

function render(drives) {
  const chanCommandDisabledAttr = chanBusy ? 'disabled' : '';
  const localRuntimeDisabledAttr = chanBusy ? 'disabled' : '';

  if (!drives.length) {
    main.innerHTML = `
      <div class="empty">
        <h2>No drives yet</h2>
        <p>A <em>drive</em> is a local folder with your markdown files.
           Pick one to get started.</p>
        <button class="btn primary" id="empty-pick" ${chanCommandDisabledAttr}>Open drive</button>
      </div>`;
    document.getElementById('empty-pick').onclick = pickAndAdd;
    return;
  }

  const rows = drives.map((d) => {
    const hasUrl = !!d.url;
    const urlAttr = escapeAttr(d.url || '');
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
      // `fullstack-53`: dropped the name cell. Path + actions are
      // enough; the rename display surface was leftover from a
      // previous launcher iteration.
      return `
      <tr data-kind="tunneled"
          data-tunnel-label="${escapeAttr(d.label || '')}"
          data-tunnel-drive="${escapeAttr(d.drive || '')}"
          data-url="${urlAttr}">
        <td><span class="tag tag-tunnel" title="${escapeAttr(tip)}">tunnel</span></td>
        <td class="path-cell muted">${escapeHtml(d.label || '')}</td>
        <td>
          <div class="row-actions">
            ${renderOpenSplit({ hasUrl, includeForget: false, forgetDisabledAttr: 'disabled' })}
          </div>
        </td>
      </tr>`;
    }
    if (d.kind === 'outbound') {
      const display = d.label || d.url || 'Remote drive';
      return `
      <tr data-kind="outbound"
          data-outbound-id="${escapeAttr(d.id || '')}"
          data-url="${urlAttr}">
        <td><span class="tag tag-outbound" title="Attached URL">url</span></td>
        <td class="path-cell remote-cell" title="${escapeAttr(d.url || '')}">${escapeHtml(display)}</td>
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
      <td class="path-cell" data-act="reveal" title="${escapeAttr(d.path)} — click to open in Finder">${renderPath(d.path)}</td>
      <td>
        <div class="row-actions">
          ${renderFeaturesToggle(d.path, chanCommandDisabledAttr)}
          ${renderOpenSplit({ hasUrl, includeForget: true, forgetDisabledAttr: chanCommandDisabledAttr })}
        </div>
      </td>
    </tr>
    ${renderFeaturesPanel(d.path)}`;
  }).join('');

  main.innerHTML = `
    <table class="drives">
      <thead>
        <tr>
          <th style="width:60px">On</th>
          <th>Path</th>
          <th style="width:150px"></th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>`;

  bindRowEvents();
}

/// `fullstack-b-28a`: per-drive features toggle. Renders the
/// "⚙" expand button shown alongside the Open split. Clicking
/// expands the sibling feature-panel row showing the BGE +
/// reports checkboxes. Stub-persisted via chan-desktop's
/// desktop config until `systacean-27` lands the chan-drive
/// config API + `-b-28b` swaps the IPC body.
function renderFeaturesToggle(path, disabledAttr = '') {
  return `
    <button class="btn features-toggle" data-act="toggle-features"
            data-features-path="${escapeAttr(path)}"
            aria-expanded="false"
            aria-controls="features-${escapeAttr(path)}"
            ${disabledAttr}
            title="Per-drive feature toggles">
      <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor"
           stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h0a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h0a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v0a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
    </button>`;
}

/// `fullstack-b-28a`: sibling row holding the BGE + reports
/// checkboxes + brief explanatory copy. Hidden by default; the
/// features-toggle button flips the `hidden` attribute on click
/// and lazy-loads the persisted state via `get_drive_features`.
/// Round-2-plan's full explanatory paragraph + per-toggle hint
/// copy will land in `-b-28b`'s full pre-flight screen.
function renderFeaturesPanel(path) {
  return `
    <tr class="features-panel" id="features-${escapeAttr(path)}"
        data-features-for="${escapeAttr(path)}" hidden>
      <td colspan="3">
        <div class="features-content">
          <p class="features-copy">
            Chan walks this drive and indexes the wiki-link graph plus
            BM25 keyword search by default. Two optional layers can be
            enabled per-drive; both default off and drop their data when
            disabled.
          </p>
          <label class="features-row">
            <input type="checkbox" data-feat="bge" disabled />
            <span class="features-label">
              <strong>Semantic search</strong>
              <span class="features-hint">
                Dense embeddings via BGE-small (~63 MB shared model;
                per-drive vector index).
              </span>
            </span>
          </label>
          <label class="features-row">
            <input type="checkbox" data-feat="reports" disabled />
            <span class="features-label">
              <strong>Reports</strong>
              <span class="features-hint">
                File classification + per-language SLOC + Basic COCOMO
                via chan-report.
              </span>
            </span>
          </label>
        </div>
      </td>
    </tr>`;
}

/// Per-row "Open" split button: primary action opens the drive in
/// an in-app webview; caret reveals a menu with "Open in Browser"
/// and (for local drives only) "Forget Drive". The primary + caret
/// are both gated by `hasUrl` so a drive that isn't running can't
/// be opened; Forget stays enabled regardless of URL state since
/// it just removes the registry entry.
function renderOpenSplit({ hasUrl, includeForget, forgetDisabledAttr, forgetLabel = 'Forget Drive' }) {
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
        const drive = tr.dataset.tunnelDrive || '';
        if (!label || !drive) return;
        try {
          await invoke('open_tunneled_drive', { label, drive });
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
          await invoke('open_outbound_drive', { id });
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
          await invoke('remove_outbound_drive', { id });
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
      try {
        await invoke('set_drive_on', { path, on: e.target.checked });
      } catch (err) {
        showError(err);
      }
      await refresh();
    });

    tr.querySelector('[data-act="launch"]').addEventListener('click', async () => {
      // In-app Tauri webview; each click adds another window so
      // multi-window per drive is the default.
      try {
        await invoke('open_local_drive', { path });
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
      // "Forget Drive" removes the drive entry from the chan
      // registry. Files on disk are untouched; the user can re-add
      // the folder later via Open drive. Tunneled drives have no
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

    bindFeaturesToggle(tr);
    bindSplitMenu(tr);
  });
}

/// `fullstack-b-28a`: wire the ⚙ features toggle on a drive row.
/// Click flips the sibling `features-panel` row's `hidden`
/// attribute. First open lazy-loads the persisted state via
/// `get_drive_features` (so a fresh render only pays the IPC cost
/// on drives the user actually inspects). Subsequent
/// checkbox changes call `set_drive_features` with the current
/// pair value; optimistic update + revert on failure.
function bindFeaturesToggle(tr) {
  const path = tr.dataset.path;
  const toggle = tr.querySelector('[data-act="toggle-features"]');
  if (!toggle) return;
  const panel = main.querySelector(`tr.features-panel[data-features-for="${cssEscape(path)}"]`);
  if (!panel) return;

  toggle.addEventListener('click', async (e) => {
    e.stopPropagation();
    const willOpen = panel.hasAttribute('hidden');
    if (willOpen) {
      panel.removeAttribute('hidden');
      toggle.setAttribute('aria-expanded', 'true');
      if (!panel.dataset.loaded) {
        await loadFeaturesInto(panel, path);
        panel.dataset.loaded = '1';
      }
    } else {
      panel.setAttribute('hidden', '');
      toggle.setAttribute('aria-expanded', 'false');
    }
  });

  for (const box of panel.querySelectorAll('input[type="checkbox"][data-feat]')) {
    box.addEventListener('change', async (e) => {
      const target = e.target;
      const checked = target.checked;
      const features = collectFeaturesFromPanel(panel);
      try {
        await invoke('set_drive_features', { path, features });
      } catch (err) {
        // Revert the toggle on failure so the UI matches persisted state.
        target.checked = !checked;
        showError(err);
      }
    });
  }
}

/// Read the persisted feature pair via IPC and reflect it on the
/// panel's checkboxes. Boxes are `disabled` while the IPC is in
/// flight so a fast double-click can't fire a `set_drive_features`
/// before the load resolves.
async function loadFeaturesInto(panel, path) {
  try {
    const features = await invoke('get_drive_features', { path });
    for (const box of panel.querySelectorAll('input[type="checkbox"][data-feat]')) {
      const feat = box.dataset.feat;
      box.checked = Boolean(features && features[feat]);
      box.disabled = false;
    }
  } catch (err) {
    showError(err);
  }
}

/// Snapshot the checkbox pair into the same `{bge, reports}` shape
/// the Rust IPC expects. Missing field defaults to false on the
/// Rust side; defensive default here keeps the contract explicit.
function collectFeaturesFromPanel(panel) {
  const features = { bge: false, reports: false };
  for (const box of panel.querySelectorAll('input[type="checkbox"][data-feat]')) {
    const feat = box.dataset.feat;
    if (feat in features) features[feat] = box.checked;
  }
  return features;
}

/// CSS.escape polyfill for older webviews. Tauri's WKWebView /
/// WebView2 both ship `CSS.escape` today, but the launcher's
/// minimum-supported runtime predates that guarantee; the
/// fallback hand-escapes the ASCII characters that matter for
/// drive paths (`/`, `.`, ` `, `:`).
function cssEscape(s) {
  if (typeof CSS !== 'undefined' && typeof CSS.escape === 'function') {
    return CSS.escape(s);
  }
  return String(s).replace(/[^a-zA-Z0-9_-]/g, (ch) => `\\${ch}`);
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

function escapeHtml(s) {
  return String(s)
    .replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}
function escapeAttr(s) {
  return escapeHtml(s).replaceAll('"', '&quot;');
}

openBtn.addEventListener('click', pickAndAdd);

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
  lastDrivesJson = '';
  refresh().catch(showError);
});

/// Tunnel panel. Hidden until the user clicks "Attach", then
/// rendered inline above the drives table. Two states:
///
///   1. Setup: port input (placeholder "auto") + Start button.
///   2. Listening: bound port + `chan serve` snippet + Stop button.
///
/// A remote drive that registers while the panel is in state 2 is
/// auto-launched via `openUrl`; the panel stays visible so the user
/// can connect more remotes from the same listening session.
let tunnelPanelOpen = false;

async function toggleTunnelPanel() {
  tunnelPanelOpen = !tunnelPanelOpen;
  await renderTunnelPanel();
}

async function renderTunnelPanel() {
  const slot = document.getElementById('tunnel-panel-slot');
  if (!slot) return;
  if (!tunnelPanelOpen) {
    slot.innerHTML = '';
    tunnelBtn.textContent = 'Attach';
    return;
  }
  let status;
  try {
    status = await invoke('tunnel_status');
  } catch (e) {
    showError(e);
    slot.innerHTML = '';
    tunnelPanelOpen = false;
    return;
  }
  tunnelBtn.textContent = status.listening ? 'Hide' : 'Attach';
  slot.innerHTML = renderTunnelPanelHtml(status);
  bindTunnelPanelEvents(status);
}

/// Whether to render the SSH `-R` snippet alongside the `chan serve`
/// snippet. `local` means "the remote chan serve runs on the same
/// machine as this desktop, no SSH needed". `tunnel` means "chan
/// serve lives on a remote host and an SSH reverse-forward bridges
/// to this desktop's loopback port". Persisted in localStorage; the
/// backend doesn't care since both snippets are pre-formatted.
const TUNNEL_MODE_KEY = 'chanDesktopTunnelMode';
function tunnelMode() {
  const v = localStorage.getItem(TUNNEL_MODE_KEY);
  return v === 'local' ? 'local' : 'tunnel';
}
function setTunnelMode(mode) {
  localStorage.setItem(TUNNEL_MODE_KEY, mode === 'local' ? 'local' : 'tunnel');
}

function renderTunnelPanelHtml(status) {
  if (status.listening && status.port != null) {
    const ssh = status.ssh_snippet || '';
    const cmd = status.chan_serve_snippet || '';
    const mode = tunnelMode();
    const isTunnel = mode === 'tunnel';
    const sshBlock = isTunnel
      ? `<p class="muted">SSH from this machine to the remote with a reverse forward:</p>
         <pre class="snippet" data-copy="${escapeAttr(ssh)}" title="click to copy">${escapeHtml(ssh)}</pre>
         <p class="muted">Then on the remote run:</p>`
      : `<p class="muted">On this machine, run:</p>`;
    return `
      ${renderOutboundAttachForm()}
      <section class="tunnel-panel">
        <header>
          <strong>Listening on 127.0.0.1:${status.port}</strong>
          <div class="seg-toggle" role="tablist" aria-label="Where will chan serve run?">
            <button class="seg ${mode === 'local' ? 'on' : ''}" data-mode="local"
                    role="tab" aria-selected="${mode === 'local'}">Local</button>
            <button class="seg ${mode === 'tunnel' ? 'on' : ''}" data-mode="tunnel"
                    role="tab" aria-selected="${mode === 'tunnel'}">Tunnel</button>
          </div>
          <button class="btn danger" data-act="tunnel-stop">Stop</button>
        </header>
        ${sshBlock}
        <pre class="snippet" data-copy="${escapeAttr(cmd)}" title="click to copy">${escapeHtml(cmd)}</pre>
        <p class="muted">Connected drives appear in the list below and open automatically.</p>
      </section>`;
  }
  return `
    ${renderOutboundAttachForm()}
    <section class="tunnel-panel">
      <header><strong>Receive a remote drive</strong></header>
      <p class="muted">Bind a loopback port to accept incoming <code>chan serve --tunnel-url</code> from another machine over an SSH reverse forward.</p>
      <div class="tunnel-row">
        <label>Port
          <input id="tunnel-port-input" type="number" min="0" max="65535" placeholder="auto"
                 value="${status.preferred_port ? status.preferred_port : ''}"/>
        </label>
        <label>Label
          <input id="tunnel-label-input" type="text" maxlength="64"
                 value="${escapeAttr(status.preferred_label || '')}"/>
        </label>
        <label>Drive
          <input id="tunnel-drive-input" type="text" maxlength="32"
                 value="${escapeAttr(status.preferred_drive || '')}"/>
        </label>
        <button class="btn primary" data-act="tunnel-start">Start listening</button>
      </div>
      <p class="muted">Port 0 / empty lets the OS pick. Label appears as the first URL segment. Drive name is lowercase ASCII + hyphens.</p>
    </section>`;
}

function renderOutboundAttachForm() {
  return `
    <section class="tunnel-panel outbound-panel">
      <header><strong>Open by URL</strong></header>
      <div class="tunnel-row outbound-row">
        <label class="outbound-url-field">URL
          <input id="outbound-url-input" type="url" autocomplete="off" spellcheck="false"
                 placeholder="http://127.0.0.1:4000/?t=..."/>
        </label>
        <label>Name
          <input id="outbound-label-input" type="text" maxlength="120" autocomplete="off"/>
        </label>
        <button class="btn primary" data-act="outbound-add">Attach URL</button>
      </div>
    </section>`;
}

function bindTunnelPanelEvents(_status) {
  // Mode toggle (Local | Tunnel). Pure UI state — persisted in
  // localStorage, no backend round-trip. Switching just re-renders
  // the snippet block.
  document.querySelectorAll('.seg-toggle .seg').forEach((btn) => {
    btn.addEventListener('click', async () => {
      setTunnelMode(btn.dataset.mode);
      await renderTunnelPanel();
    });
  });

  const startBtn = document.querySelector('[data-act="tunnel-start"]');
  if (startBtn) {
    startBtn.addEventListener('click', async () => {
      const portInp = document.getElementById('tunnel-port-input');
      const rawPort = (portInp && portInp.value || '').trim();
      const preferred = rawPort === '' ? 0 : Math.max(0, Math.min(65535, Number(rawPort) | 0));
      const label = (document.getElementById('tunnel-label-input').value || '').trim();
      const drive = (document.getElementById('tunnel-drive-input').value || '').trim();
      try {
        await invoke('tunnel_start', { preferredPort: preferred, label, drive });
      } catch (e) {
        showError(e);
        return;
      }
      await renderTunnelPanel();
    });
  }
  const outboundBtn = document.querySelector('[data-act="outbound-add"]');
  if (outboundBtn) {
    outboundBtn.addEventListener('click', attachOutboundUrl);
    for (const input of [
      document.getElementById('outbound-url-input'),
      document.getElementById('outbound-label-input'),
    ]) {
      if (!input) continue;
      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          outboundBtn.click();
        }
      });
    }
  }
  const stopBtn = document.querySelector('[data-act="tunnel-stop"]');
  if (stopBtn) {
    stopBtn.addEventListener('click', async () => {
      try {
        await invoke('tunnel_stop');
      } catch (e) {
        showError(e);
        return;
      }
      await renderTunnelPanel();
      await refresh();
    });
  }
  document.querySelectorAll('.tunnel-panel .snippet[data-copy]').forEach((node) => {
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

async function attachOutboundUrl() {
  const urlInput = document.getElementById('outbound-url-input');
  const labelInput = document.getElementById('outbound-label-input');
  const url = (urlInput && urlInput.value || '').trim();
  const label = (labelInput && labelInput.value || '').trim();
  if (!url) {
    if (urlInput) urlInput.focus();
    showError('Remote URL is required.');
    return;
  }
  try {
    await invoke('add_outbound_drive', { url, label });
  } catch (e) {
    showError(e);
    return;
  }
  if (urlInput) urlInput.value = '';
  if (labelInput) labelInput.value = '';
  await refresh();
}

tunnelBtn.addEventListener('click', toggleTunnelPanel);

// `tunneled-drive-ready` is informational on this side: the Rust
// supervisor already opened the in-app webview window the moment
// the per-tenant listener bound. We just refresh the drive table
// so the new row shows up alongside its URL.
listen('tunneled-drive-ready', () => { refresh().catch(showError); });

listen('tunnel-state-changed', () => { renderTunnelPanel().catch(showError); });

boot().catch(showError);
maybeOfferUpdate().catch((e) => console.warn('update flow error:', e));
