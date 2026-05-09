const { listen } = window.__TAURI__.event;

const logEl = document.getElementById('log');
const clearBtn = document.getElementById('clear');

const MAX_LINES = 5000;

function shortName(p) {
  const i = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
  return i >= 0 ? p.slice(i + 1) : p;
}

function append({ path, line }) {
  // Auto-scroll only when the user is already pinned to the bottom.
  const pinned = logEl.scrollTop + logEl.clientHeight >= logEl.scrollHeight - 8;

  const tag = document.createElement('span');
  tag.className = 'log-tag';
  tag.textContent = `[${shortName(path)}] `;

  const body = document.createTextNode(line + '\n');

  logEl.appendChild(tag);
  logEl.appendChild(body);

  // Cap memory: drop the oldest line-pair when over the limit.
  while (logEl.childNodes.length > MAX_LINES * 2) {
    logEl.removeChild(logEl.firstChild);
  }

  if (pinned) logEl.scrollTop = logEl.scrollHeight;
}

clearBtn.addEventListener('click', () => {
  logEl.textContent = '';
});

listen('chan-log', (e) => append(e.payload));
