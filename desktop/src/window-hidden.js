// Window Hidden notice. Two jobs: match the launcher theme, and dismiss on
// OK / Return / Escape by closing this window.
//
// The Rust opener injects the formatted body string as a global before this
// script runs (the window's initialization_script), so the arbitrary window
// title never has to ride a query string.

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

const body =
  typeof window.__CHAN_NOTICE_BODY__ === 'string'
    ? window.__CHAN_NOTICE_BODY__
    : '';
const bodyEl = document.getElementById('notice-body');
if (bodyEl) bodyEl.textContent = body;

// Close this notice window. Prefer the Tauri window API; fall back to the DOM
// close (e.g. opened standalone in a browser during dev, where there is no
// Tauri bridge).
function dismiss() {
  try {
    const win = window.__TAURI__ && window.__TAURI__.window;
    if (win && typeof win.getCurrentWindow === 'function') {
      win.getCurrentWindow().close().catch(() => window.close());
      return;
    }
  } catch (_) {
    /* fall through to the DOM close */
  }
  window.close();
}

const ok = document.getElementById('ok');
if (ok) {
  ok.addEventListener('click', dismiss);
  ok.focus();
}

// Return confirms, Escape dismisses — both close, matching the native alert
// this replaces.
window.addEventListener('keydown', (e) => {
  if (e.key === 'Enter' || e.key === 'Escape') {
    e.preventDefault();
    dismiss();
  }
});
