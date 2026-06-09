// About window logic. Three small jobs:
//   1. Apply the launcher theme so the window matches (same `tauri://`
//      origin as the launcher, so localStorage.chanDesktopTheme is shared).
//   2. Show the desktop version, passed in as the `?v=` query param by the
//      Rust opener (avoids needing an `app`-plugin capability for getVersion).
//   3. Open external links in the system browser via the opener plugin
//      (a plain <a> would navigate the About webview itself).
const { openUrl } = window.__TAURI__.opener;

const THEME_KEY = 'chanDesktopTheme';
function applyTheme() {
  const saved = localStorage.getItem(THEME_KEY);
  const root = document.documentElement;
  if (saved === 'dark' || saved === 'light') {
    root.setAttribute('data-theme', saved);
  } else {
    root.removeAttribute('data-theme');
  }
}
applyTheme();
window
  .matchMedia('(prefers-color-scheme: light)')
  .addEventListener('change', applyTheme);

const version = new URLSearchParams(location.search).get('v');
if (version) {
  document.getElementById('version').textContent = version;
}

// Route every external link through the opener plugin. preventDefault stops
// the webview from trying to navigate to the href itself.
for (const a of document.querySelectorAll('a.ext')) {
  a.addEventListener('click', (e) => {
    e.preventDefault();
    const href = a.getAttribute('href');
    if (href) openUrl(href).catch((err) => console.warn('openUrl failed:', err));
  });
}
