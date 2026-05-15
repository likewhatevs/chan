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
/// Boot-time preflight result from the Rust side. While `ok=false`,
/// every action that would invoke chan (Open drive, the per-row On
/// toggles, Forget, the tunnel Listen button) is disabled and a
/// persistent banner explains why. `kind` is one of "ok" |
/// "translocated" | "missing"; the renderer keys disabled state off
/// `ok` alone and the banner copy off `reason`.
let chanBinStatus = { ok: true, kind: 'ok', reason: '' };
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
  const drives = await invoke('list_drives');
  const json = JSON.stringify(drives);
  if (json !== lastDrivesJson) {
    lastDrivesJson = json;
    render(drives);
  }
  return drives;
}

/// Render a drive's filesystem path with the user's home folder
/// collapsed to a house glyph. Paths outside the home dir render
/// verbatim. Returns an HTML string; caller injects into a clickable
/// cell that calls `reveal_in_finder` with the full path.
function renderPath(full) {
  if (homeDir && (full === homeDir || full.startsWith(homeDir + '/'))) {
    const rest = full.slice(homeDir.length).replace(/^\//, '');
    // Inline SVG house glyph keeps the rendering self-contained and
    // tints with currentColor for theme switches.
    const house = `<svg class="ic-home" viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-label="home"><path d="M3 11l9-8 9 8"/><path d="M5 10v10h14V10"/></svg>`;
    if (!rest) return house;
    return `${house}<span class="path-sep">/</span>${escapeHtml(rest)}`;
  }
  return escapeHtml(full);
}

async function boot() {
  await checkChanBin();
  const drives = await refresh();
  // When chan is unavailable, suppress the first-run "no drives →
  // open picker" prompt. The picker would either fail outright
  // (translocated / missing binary) or, worse, succeed and leave
  // the user looking at an empty registry they can't use.
  if (!booted && drives.length === 0 && chanBinStatus.ok) {
    booted = true;
    await pickAndAdd();
  } else {
    booted = true;
  }
}

/// Call the Rust preflight, store the result, mirror it onto the
/// chrome (Open drive / Listen / banner). Called once on boot; if
/// future work needs a re-check (e.g. user moved the app and we
/// want to detect it without a restart) the same function is the
/// hook.
async function checkChanBin() {
  try {
    chanBinStatus = await invoke('chan_bin_status');
  } catch (e) {
    chanBinStatus = {
      ok: false,
      kind: 'missing',
      reason: typeof e === 'string' ? e : 'Chan command-line tool is unavailable.',
    };
  }
  applyChanBinStatus();
}

function applyChanBinStatus() {
  const ok = chanBinStatus.ok;
  openBtn.disabled = !ok;
  tunnelBtn.disabled = !ok;
  document.body.classList.toggle('chan-bin-unavailable', !ok);

  let banner = document.getElementById('chan-bin-banner');
  if (ok) {
    if (banner) banner.remove();
    return;
  }
  const msg = chanBinStatus.reason
    || 'Chan command-line tool is unavailable. Drive management is disabled.';
  if (!banner) {
    banner = document.createElement('div');
    banner.id = 'chan-bin-banner';
    banner.className = 'error-banner persistent';
    document.body.insertBefore(banner, document.body.firstChild);
  }
  banner.textContent = msg;
}

async function pickAndAdd() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Select a folder containing markdown files',
  });
  if (typeof selected !== 'string' || !selected.length) return;
  try {
    await invoke('add_drive', { path: selected });
  } catch (e) {
    showError(e);
    return;
  }
  await refresh();
}

function render(drives) {
  // Single source of truth for the row-level disabled attribute.
  // Every control that would spawn chan (toggle, Forget) is keyed
  // off this. Launch and Reveal stay live because they don't
  // depend on the binary — Launch needs a URL (which is gated by
  // the running serve anyway) and Reveal just opens Finder.
  const disabledAttr = chanBinStatus.ok ? '' : 'disabled';

  if (!drives.length) {
    main.innerHTML = `
      <div class="empty">
        <h2>No drives yet</h2>
        <p>A <em>drive</em> is a local folder with your markdown files.
           Pick one to get started.</p>
        <button class="btn primary" id="empty-pick" ${disabledAttr}>Open drive</button>
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
      return `
      <tr data-kind="tunneled"
          data-tunnel-label="${escapeAttr(d.label || '')}"
          data-tunnel-drive="${escapeAttr(d.drive || '')}"
          data-url="${urlAttr}">
        <td><span class="tag tag-tunnel" title="${escapeAttr(tip)}">tunnel</span></td>
        <td class="path-cell muted">${escapeHtml(d.label || '')}</td>
        <td class="name-cell">${escapeHtml(d.drive || d.name)}</td>
        <td>
          <div class="row-actions">
            ${renderOpenSplit({ hasUrl, includeForget: false, disabledAttr })}
          </div>
        </td>
      </tr>`;
    }
    return `
    <tr data-path="${escapeAttr(d.path)}" data-url="${urlAttr}">
      <td>
        <label class="switch">
          <input type="checkbox" data-act="toggle-on" ${d.on ? 'checked' : ''} ${disabledAttr}/>
          <span class="slider"></span>
        </label>
      </td>
      <td class="path-cell" data-act="reveal" title="${escapeAttr(d.path)} — click to open in Finder">${renderPath(d.path)}</td>
      <td class="name-cell" title="set via &#96;chan rename&#96;">${escapeHtml(d.name)}</td>
      <td>
        <div class="row-actions">
          ${renderOpenSplit({ hasUrl, includeForget: true, disabledAttr })}
        </div>
      </td>
    </tr>`;
  }).join('');

  main.innerHTML = `
    <table class="drives">
      <thead>
        <tr>
          <th style="width:60px">On</th>
          <th>Path</th>
          <th style="width:200px">Name</th>
          <th style="width:150px"></th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>`;

  bindRowEvents();
}

/// Per-row "Open" split button: primary action opens the drive in
/// an in-app webview; caret reveals a menu with "Open in Browser"
/// and (for local drives only) "Forget Drive". The primary + caret
/// are both gated by `hasUrl` so a drive that isn't running can't
/// be opened; Forget stays enabled regardless of URL state since
/// it just removes the registry entry.
function renderOpenSplit({ hasUrl, includeForget, disabledAttr }) {
  const openDisabled = hasUrl ? '' : 'disabled';
  const forgetItem = includeForget
    ? `<li><button class="menu-item" data-act="remove" role="menuitem" ${disabledAttr}>Forget Drive</button></li>`
    : '';
  return `
    <div class="split-btn">
      <button class="btn primary" data-act="launch" ${openDisabled}>Open</button>
      <button class="btn primary split-caret" data-act="menu-toggle"
              aria-haspopup="true" aria-expanded="false" aria-label="More actions">
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
          await invoke('remove_drive', { path });
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

// `chan serve` exited before printing the URL banner — the toggle
// would have silently flipped back to off otherwise. Pop a modal
// with the captured stderr so the user can see *why* instead of
// guessing. Serialized so two near-simultaneous failures don't
// stack two dialogs on top of each other.
let serveFailedPending = Promise.resolve();
listen('serve-failed', (e) => {
  serveFailedPending = serveFailedPending.then(() => showServeFailed(e.payload || {}));
});

async function showServeFailed(p) {
  const driveLabel = p.key ? p.key : 'this drive';
  let exitDesc;
  if (typeof p.exit_signal === 'number') {
    exitDesc = `chan was killed by signal ${p.exit_signal}.`;
  } else if (typeof p.exit_code === 'number') {
    exitDesc = `chan exited with code ${p.exit_code}.`;
  } else {
    exitDesc = 'chan exited without reporting a status.';
  }
  // Keep the dialog body bounded: native message dialogs don't
  // scroll on every platform, so trim to the last 20 lines.
  const tailLines = Array.isArray(p.stderr_tail) ? p.stderr_tail.slice(-20) : [];
  const tail = tailLines.length ? tailLines.join('\n') : '(no output captured)';
  const body =
    `Failed to start ${driveLabel}.\n\n` +
    `${exitDesc}\n\n` +
    `Last output:\n${tail}`;
  try {
    await message(body, { title: 'Drive failed to start', kind: 'error' });
  } catch {
    // Dialog plugin not available or denied: fall back to the
    // inline banner so the user still sees something.
    showError(body);
  }
}

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

tunnelBtn.addEventListener('click', toggleTunnelPanel);

// `tunneled-drive-ready` is informational on this side: the Rust
// supervisor already opened the in-app webview window the moment
// the per-tenant listener bound. We just refresh the drive table
// so the new row shows up alongside its URL.
listen('tunneled-drive-ready', () => { refresh().catch(showError); });

listen('tunnel-state-changed', () => { renderTunnelPanel().catch(showError); });

boot().catch(showError);
maybeOfferUpdate().catch((e) => console.warn('update flow error:', e));
