// Test-server lifecycle: seed a throwaway workspace, launch `chan open`
// on it (devserver handoff disabled), parse the tokenized URL off
// stderr, and tear everything down (process, registry entry, tempdir)
// afterwards. Kills are scoped to the exact child pid so concurrent
// agents on one machine never hit each other's servers.

import { spawn, execFile } from "node:child_process";
import { globSync, mkdtempSync, readdirSync, cpSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFileP = promisify(execFile);
const HERE = dirname(fileURLToPath(import.meta.url));

// 1x1 red PNG; the markdown seeds size it up via the #w= grammar so
// the exported pages carry a visible, deterministic image block.
const RED_DOT_PNG_B64 =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

export function seedWorkspace() {
  const dir = mkdtempSync(join(tmpdir(), "chan-smoke-"));
  cpSync(join(HERE, "..", "seed"), dir, { recursive: true });
  writeFileSync(join(dir, "photo.png"), Buffer.from(RED_DOT_PNG_B64, "base64"));
  return dir;
}

/// Launch `chan open <dir>` and resolve with the tokenized URL. The
/// child is the server (no daemonize); its pid scopes the teardown and
/// the control-socket glob.
export function launchServer(chanBin, workspaceDir, log) {
  const child = spawn(chanBin, ["open", workspaceDir], {
    env: {
      ...process.env,
      CHAN_NO_DEVSERVER_HANDOFF: "1",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  const stderrLines = [];
  let resolved = false;

  const url = new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`chan open: no URL after 60s\n${stderrLines.join("\n")}`)),
      60_000,
    );
    const scan = (chunk) => {
      for (const line of chunk.toString().split("\n")) {
        if (!line.trim()) continue;
        stderrLines.push(line);
        log?.(`[server] ${line}`);
        const m = line.match(/https?:\/\/[^\s]+/);
        if (m && !resolved) {
          resolved = true;
          clearTimeout(timer);
          resolve(m[0]);
        }
      }
    };
    child.stderr.on("data", scan);
    child.stdout.on("data", scan);
    child.on("exit", (code) => {
      if (!resolved) {
        clearTimeout(timer);
        reject(new Error(`chan open exited early (${code})\n${stderrLines.join("\n")}`));
      }
    });
  });

  return { child, url, stderrLines };
}

/// The server's pid-scoped control socket: chan-control-<pid>-<rand>.sock
/// in $XDG_RUNTIME_DIR (or /tmp). Present once the server is up.
export function findControlSocket(pid) {
  const dirs = [process.env.XDG_RUNTIME_DIR, tmpdir(), "/tmp"].filter(Boolean);
  for (const dir of dirs) {
    try {
      const hit = readdirSync(dir).find(
        (name) => name.startsWith(`chan-control-${pid}-`) && name.endsWith(".sock"),
      );
      if (hit) return join(dir, hit);
    } catch {
      // Unreadable candidate dir; try the next one.
    }
  }
  return null;
}

export async function teardownServer(chanBin, child, workspaceDir, log) {
  try {
    child.kill("SIGTERM");
    await new Promise((resolve) => {
      const t = setTimeout(() => {
        try {
          child.kill("SIGKILL");
        } catch {}
        resolve();
      }, 5000);
      child.on("exit", () => {
        clearTimeout(t);
        resolve();
      });
    });
  } catch {}
  try {
    await execFileP(chanBin, ["workspace", "rm", workspaceDir]);
  } catch (e) {
    log?.(`[teardown] workspace rm failed: ${e.message}`);
  }
  try {
    rmSync(workspaceDir, { recursive: true, force: true });
  } catch {}
}

export function defaultChrome() {
  const hits = globSync(
    join(process.env.HOME ?? "", ".cache/puppeteer/chrome/linux-*/chrome-linux64/chrome"),
  ).sort();
  return hits[hits.length - 1] ?? null;
}
