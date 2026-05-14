const { invoke } = window.__TAURI__.core;
const { open, ask } = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;
const { check: checkForUpdate } = window.__TAURI__.updater;
const { relaunch } = window.__TAURI__.process;

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
  const drives = await refresh();
  if (!booted && drives.length === 0) {
    booted = true;
    await pickAndAdd();
  } else {
    booted = true;
  }
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
  if (!drives.length) {
    main.innerHTML = `
      <div class="empty">
        <h2>No drives yet</h2>
        <p>A <em>drive</em> is a local folder with your markdown files.
           Pick one to get started.</p>
        <button class="btn primary" id="empty-pick">Open drive</button>
      </div>`;
    document.getElementById('empty-pick').onclick = pickAndAdd;
    return;
  }

  const rows = drives.map((d) => {
    const hasUrl = !!d.url;
    if (d.kind === 'tunneled') {
      // Tunneled row: no On toggle (the remote owns the lifecycle),
      // no Path (it's a remote folder), no Close (the remote drops
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
          data-tunnel-drive="${escapeAttr(d.drive || '')}">
        <td><span class="tag tag-tunnel" title="${escapeAttr(tip)}">tunnel</span></td>
        <td class="path-cell muted">${escapeHtml(d.label || '')}</td>
        <td class="name-cell">${escapeHtml(d.drive || d.name)}</td>
        <td>
          <div class="url-cell">
            <input class="url-input" value="${escapeAttr(d.url)}" placeholder="—" readonly />
            <button class="btn" data-act="launch" ${hasUrl ? '' : 'disabled'}>Launch</button>
          </div>
        </td>
      </tr>`;
    }
    return `
    <tr data-path="${escapeAttr(d.path)}">
      <td>
        <label class="switch">
          <input type="checkbox" data-act="toggle-on" ${d.on ? 'checked' : ''}/>
          <span class="slider"></span>
        </label>
      </td>
      <td class="path-cell" data-act="reveal" title="${escapeAttr(d.path)} — click to open in Finder">${renderPath(d.path)}</td>
      <td class="name-cell" title="set via &#96;chan rename&#96;">${escapeHtml(d.name)}</td>
      <td>
        <div class="url-cell">
          <input class="url-input" value="${escapeAttr(d.url)}" placeholder="—" readonly />
          <button class="btn" data-act="launch" ${hasUrl ? '' : 'disabled'}>Launch</button>
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
          <th style="width:280px">URL</th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>`;

  bindRowEvents();
}

function bindRowEvents() {
  // Tunneled rows: only the Launch button is interactive. No
  // toggle / reveal / remove handlers because there is no
  // desktop-side lifecycle to control — the remote `chan serve`
  // owns it.
  main.querySelectorAll('tr[data-kind="tunneled"]').forEach((tr) => {
    const launch = tr.querySelector('[data-act="launch"]');
    if (launch) {
      launch.addEventListener('click', async () => {
        // Launch reuses the same in-app Tauri webview the
        // supervisor opens on first registration. Going to the
        // system browser would split the editor experience across
        // two surfaces and break the key-bridge shortcuts.
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
      // multi-window per drive is the default. The URL stays in
      // the row's input for users who want to copy it elsewhere.
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
  });

  // Click-to-copy on every URL field (regular + tunneled rows). The
  // input is readonly so a normal click just selects; we copy on
  // click and flash a "Copied" label in place of the URL for ~900ms.
  // `bindRowEvents` runs after each `render(drives)`, so newly added
  // rows get the handler too.
  main.querySelectorAll('.url-input').forEach((inp) => {
    inp.addEventListener('click', () => copyUrlField(inp));
  });
}

async function copyUrlField(input) {
  const url = input.value;
  if (!url || url === 'Copied') return;
  try {
    await navigator.clipboard.writeText(url);
  } catch {
    return;
  }
  input.value = 'Copied';
  input.classList.add('copied');
  setTimeout(() => {
    // Guard against the row being re-rendered while the timeout was
    // pending — `input` may already be detached from the DOM.
    if (!input.isConnected) return;
    input.value = url;
    input.classList.remove('copied');
  }, 900);
}

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

/// Tunnel panel. Hidden until the user clicks "Listen…", then
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
    tunnelBtn.textContent = 'Listen';
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
  tunnelBtn.textContent = status.listening ? 'Hide' : 'Listen';
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
