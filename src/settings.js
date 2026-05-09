const { invoke } = window.__TAURI__.core;
const { revealItemInDir } = window.__TAURI__.opener;
const { ask } = window.__TAURI__.dialog;

const pathEl = document.getElementById('cfg-path');
const revealBtn = document.getElementById('reveal');
const forgetBtn = document.getElementById('forget');

let cfgPath = '';

async function load() {
  cfgPath = await invoke('get_config_path');
  pathEl.textContent = cfgPath;
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
  await invoke('forget_all');
});

load();
