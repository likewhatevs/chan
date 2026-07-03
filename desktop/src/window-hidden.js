// Parameterized notice window. The Rust opener injects a payload global
// (window.__CHAN_NOTICE__) before this script runs, so the window title, body
// sentence, theme, accent, and buttons all come from Rust and no arbitrary
// string ever rides a query string. Dismisses on a button / Return / Escape by
// closing this window; a result-carrying notice (resultId set) also reports the
// chosen button index back to Rust first.

const notice =
  window.__CHAN_NOTICE__ && typeof window.__CHAN_NOTICE__ === 'object'
    ? window.__CHAN_NOTICE__
    : {};

const root = document.documentElement;
// Theme: follow the injected launcher choice; null means follow the OS media
// query (the default). No localStorage: this window is on the Tauri App origin
// and shares no storage with the loopback-served launcher.
if (notice.theme === 'dark' || notice.theme === 'light') {
  root.setAttribute('data-theme', notice.theme);
} else {
  root.removeAttribute('data-theme');
}
// Accent: the triggering window's library colour, applied as the primary
// button + mark tint. Absent leaves the stylesheet's default brand.
if (typeof notice.accent === 'string' && notice.accent) {
  root.style.setProperty('--notice-accent', notice.accent);
}

const titleEl = document.getElementById('notice-title');
if (titleEl) titleEl.textContent = typeof notice.title === 'string' ? notice.title : '';
const bodyEl = document.getElementById('notice-body');
if (bodyEl) bodyEl.textContent = typeof notice.body === 'string' ? notice.body : '';

// Close this notice window. Prefer the Tauri window API; fall back to the DOM
// close (e.g. opened standalone in a browser during dev, no Tauri bridge).
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

// Report the chosen button (when the notice carries a resultId) and dismiss.
function choose(index) {
  const id = typeof notice.resultId === 'string' ? notice.resultId : null;
  const core = window.__TAURI__ && window.__TAURI__.core;
  if (id && core && typeof core.invoke === 'function') {
    core.invoke('notice_result', { id, choice: index }).catch(() => {});
  }
  dismiss();
}

const buttons =
  Array.isArray(notice.buttons) && notice.buttons.length
    ? notice.buttons
    : [{ label: 'OK', primary: true }];

const actions = document.getElementById('notice-actions');
let primaryEl = null;
if (actions) {
  buttons.forEach((b, i) => {
    const el = document.createElement('button');
    el.type = 'button';
    el.className = 'hidden-notice-btn' + (b && b.primary ? ' primary' : '');
    el.textContent = b && typeof b.label === 'string' ? b.label : 'OK';
    el.addEventListener('click', () => choose(i));
    actions.appendChild(el);
    if (b && b.primary && !primaryEl) primaryEl = el;
  });
  const focusTarget = primaryEl || actions.firstElementChild;
  if (focusTarget) focusTarget.focus();
}

// Return confirms the primary button (else the first); Escape picks the last
// (a secondary "Later" / "Cancel", or the same OK when there is only one),
// matching the native alert this replaces.
window.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    e.preventDefault();
    const primaryIndex = buttons.findIndex((b) => b && b.primary);
    choose(primaryIndex >= 0 ? primaryIndex : 0);
  } else if (e.key === 'Escape') {
    e.preventDefault();
    choose(buttons.length - 1);
  }
});
