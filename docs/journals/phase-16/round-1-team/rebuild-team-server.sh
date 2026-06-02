#!/usr/bin/env bash
# Rebuild + reinstall the chan/cs binaries on the latest merged code, then
# (manually) restart the team server. RUN FROM A NATIVE TERMINAL OUTSIDE THE
# CHAN WINDOW - restarting the server kills every tab it hosts (incl @@Host's
# and @@Lead's), respawning the whole team.
#
# Pre-req before you trigger the restart: @@Lead must make round-1-bootstrap.md
# RESUME-AWARE so respawned lanes resume wave-3 (round-1-wave-3.md) instead of
# re-doing wave-1. Ask @@Lead to confirm that's done.
set -euo pipefail
cd ~/dev/github.com/fiorix/chan

echo "== 1. rebuild + reinstall CLI (release chan + web bundle; cs symlink follows) =="
make install PREFIX="$HOME/.local"   # -> ~/.local/bin/chan (cs -> chan)
~/.local/bin/cs terminal scrollback --help >/dev/null 2>&1 \
  && echo "   cs has the new commands (scrollback present)" \
  || echo "   WARN: new cs commands not present - check the build"

echo
echo "== 2. restart the team server (RESPAWNS the team) =="
echo "   This part is YOUR setup-specific. Steps:"
echo "   a) stop the current 'chan serve' that hosts the team window;"
echo "   b) relaunch it on the new binary against the same workspace:"
echo "        ~/.local/bin/chan serve <your-workspace-path>   # backgrounded as you launch it"
echo "   c) re-spawn the team. !! IMPORTANT (window-id fix A caveat, @@LaneA) !!"
echo "      Fix A binds CHAN_WINDOW_ID only when the team is spawned from a"
echo "      WINDOWED chan terminal. So re-spawn the team via the SPA Team dialog"
echo "      OR run 'cs terminal team load docs/journals/phase-16/round-1-team/"
echo "      config.toml' FROM A CHAN TERMINAL IN THE OPEN SPA WINDOW - NOT from"
echo "      this native terminal (a native re-spawn leaves agents unbound, so"
echo "      cs pane/open/survey stay broken for them). Open the SPA first."
echo "   The respawned lanes read round-1-bootstrap.md -> resume from round-1-"
echo "   status.md + round-1-wave-3.md. (16 slices already merged are safe in git.)"
echo
echo "Not auto-running step 2 - it ends this very session if run inside chan."
