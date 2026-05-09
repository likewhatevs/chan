const { invoke } = window.__TAURI__.core;
const { revealItemInDir } = window.__TAURI__.opener;
const { ask, message } = window.__TAURI__.dialog;

const regPathEl = document.getElementById('reg-path');
const cfgPathEl = document.getElementById('cfg-path');
const revealRegBtn = document.getElementById('reveal-reg');
const revealCfgBtn = document.getElementById('reveal-cfg');
const forgetBtn = document.getElementById('forget');
const devModeEl = document.getElementById('dev-mode');

let regPath = '';
let cfgPath = '';

async function load() {
  [regPath, cfgPath] = await Promise.all([
    invoke('get_registry_path'),
    invoke('get_config_path'),
  ]);
  regPathEl.textContent = regPath;
  cfgPathEl.textContent = cfgPath;
  const cfg = await invoke('get_config');
  devModeEl.checked = !!cfg.dev_mode;
}

async function reveal(path) {
  if (!path) return;
  try { await revealItemInDir(path); } catch (e) {}
}

revealRegBtn.addEventListener('click', () => reveal(regPath));
revealCfgBtn.addEventListener('click', () => reveal(cfgPath));

forgetBtn.addEventListener('click', async () => {
  const ok = await ask(
    'Run `chan remove` on every registered drive and clear the desktop sidecar config?',
    {
      title: 'Forget all drives',
      kind: 'warning',
      okLabel: 'Forget',
      cancelLabel: 'Cancel',
    },
  );
  if (!ok) return;
  try {
    await invoke('forget_all');
  } catch (e) {
    await message(String(e), { title: 'Forget all drives', kind: 'error' });
  }
});

devModeEl.addEventListener('change', async () => {
  await invoke('set_dev_mode', { enabled: devModeEl.checked });
});

load();
