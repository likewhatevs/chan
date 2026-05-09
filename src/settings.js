const { invoke } = window.__TAURI__.core;

const devModeEl = document.getElementById('dev-mode');

async function load() {
  const cfg = await invoke('get_config');
  devModeEl.checked = !!cfg.dev_mode;
}

devModeEl.addEventListener('change', async () => {
  await invoke('set_dev_mode', { enabled: devModeEl.checked });
});

load();
