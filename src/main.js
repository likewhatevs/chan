const { invoke } = window.__TAURI__.core;
const { open, ask } = window.__TAURI__.dialog;
const { openUrl } = window.__TAURI__.opener;
const { listen } = window.__TAURI__.event;
const { check: checkForUpdate } = window.__TAURI__.updater;
const { relaunch } = window.__TAURI__.process;

const main = document.getElementById('main');
const openBtn = document.getElementById('open-drive');

let booted = false;

async function refresh() {
  const drives = await invoke('list_drives');
  render(drives);
  return drives;
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
    return `
    <tr data-path="${escapeAttr(d.path)}">
      <td>
        <label class="switch">
          <input type="checkbox" data-act="toggle-on" ${d.on ? 'checked' : ''}/>
          <span class="slider"></span>
        </label>
      </td>
      <td class="path-cell" title="${escapeAttr(d.path)}">${escapeHtml(d.path)}</td>
      <td class="name-cell" title="set via &#96;chan rename&#96;">${escapeHtml(d.name)}</td>
      <td>
        <div class="url-cell">
          <input class="url-input" value="${escapeAttr(d.url)}" placeholder="—" readonly />
          <button class="btn" data-act="launch" ${hasUrl ? '' : 'disabled'}>Launch</button>
        </div>
      </td>
      <td>
        <div class="row-actions">
          <button class="btn danger" data-act="remove">Close</button>
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
          <th style="width:90px"></th>
        </tr>
      </thead>
      <tbody>${rows}</tbody>
    </table>`;

  bindRowEvents();
}

function bindRowEvents() {
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
      const url = tr.querySelector('.url-input').value.trim();
      if (url) await openUrl(url);
    });

    tr.querySelector('[data-act="remove"]').addEventListener('click', async () => {
      try {
        await invoke('remove_drive', { path });
      } catch (err) {
        showError(err);
      }
      await refresh();
    });
  });
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

boot().catch(showError);
maybeOfferUpdate().catch((e) => console.warn('update flow error:', e));
