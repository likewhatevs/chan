const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

const main = document.getElementById('main');
const openBtn = document.getElementById('open-drive');
const settingsBtn = document.getElementById('open-settings');

let booted = false;

async function boot() {
  const cfg = await invoke('get_config');
  render(cfg);
  if (!booted && cfg.drives.length === 0) {
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
  if (typeof selected === 'string' && selected.length) {
    const cfg = await invoke('add_drive', { path: selected });
    render(cfg);
  }
}

function render(cfg) {
  if (!cfg.drives.length) {
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

  const rows = cfg.drives.map((d) => `
    <tr data-path="${escapeAttr(d.path)}">
      <td>
        <label class="switch">
          <input type="checkbox" data-act="toggle-on" ${d.on ? 'checked' : ''}/>
          <span class="slider"></span>
        </label>
      </td>
      <td class="path-cell" title="${escapeAttr(d.path)}">${escapeHtml(d.path)}</td>
      <td>
        <input class="name-input" data-act="rename" value="${escapeAttr(d.name)}" />
      </td>
      <td>
        <span class="vis" role="group" aria-label="Visibility">
          <button data-act="vis" data-private="true"  aria-pressed="${d.private ? 'true' : 'false'}">private</button>
          <button data-act="vis" data-private="false" aria-pressed="${d.private ? 'false' : 'true'}">public</button>
        </span>
      </td>
      <td>
        <input class="url-input" value="${escapeAttr(d.url || '—')}" readonly />
      </td>
      <td>
        <div class="row-actions">
          <button class="btn danger" data-act="remove">Close</button>
        </div>
      </td>
    </tr>`).join('');

  main.innerHTML = `
    <table class="drives">
      <thead>
        <tr>
          <th style="width:60px">On</th>
          <th>Path</th>
          <th style="width:200px">Name</th>
          <th style="width:160px">Visibility</th>
          <th style="width:200px">URL</th>
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
      const cfg = await invoke('update_drive', { update: { path, on: e.target.checked } });
      render(cfg);
    });

    const nameInput = tr.querySelector('[data-act="rename"]');
    nameInput.addEventListener('change', async () => {
      const cfg = await invoke('update_drive', { update: { path, name: nameInput.value } });
      render(cfg);
    });
    nameInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') nameInput.blur();
    });

    tr.querySelectorAll('[data-act="vis"]').forEach((b) => {
      b.addEventListener('click', async () => {
        const isPrivate = b.dataset.private === 'true';
        const cfg = await invoke('update_drive', { update: { path, private: isPrivate } });
        render(cfg);
      });
    });

    tr.querySelector('[data-act="remove"]').addEventListener('click', async () => {
      const cfg = await invoke('remove_drive', { path });
      render(cfg);
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

openBtn.addEventListener('click', pickAndAdd);
settingsBtn.addEventListener('click', () => invoke('show_settings'));

boot().catch((e) => {
  main.innerHTML = `<div class="empty"><h2>Error</h2><p>${escapeHtml(String(e))}</p></div>`;
});
