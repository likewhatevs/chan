const { invoke } = window.__TAURI__.core;
const { revealItemInDir } = window.__TAURI__.opener;
const { ask } = window.__TAURI__.dialog;

const pathEl = document.getElementById('cfg-path');
const revealBtn = document.getElementById('reveal');
const forgetBtn = document.getElementById('forget');
const devModeEl = document.getElementById('dev-mode');

let cfgPath = '';

async function load() {
  cfgPath = await invoke('get_config_path');
  pathEl.textContent = cfgPath;
  const cfg = await invoke('get_config');
  devModeEl.checked = !!cfg.dev_mode;
}

revealBtn.addEventListener('click', async () => {
  if (cfgPath) {
    try { await revealItemInDir(cfgPath); } catch (e) {}
  }
});

forgetBtn.addEventListener('click', async () => {
  const ok = await ask('Forget all drives and delete the config file?', {
    title: 'Forget all drives',
    kind: 'warning',
    okLabel: 'Forget',
    cancelLabel: 'Cancel',
  });
  if (!ok) return;
  const cfg = await invoke('forget_all');
  devModeEl.checked = !!cfg.dev_mode;
});

devModeEl.addEventListener('change', async () => {
  await invoke('set_dev_mode', { enabled: devModeEl.checked });
});

load();
