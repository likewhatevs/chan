// The settings command: opens the configuration surface. Chordless by
// default (the launcher reaches it, and the shortcut-assignment view can
// bind one), so it calls its action directly rather than dispatching
// through App.svelte's runCommand switch. Machine-global config, so it
// stays available in every window, including standalone terminals. See
// state/commands.ts for the Command shape and helpers.

import { registerCommands } from "../commands";
import { openSettings } from "../store.svelte";

registerCommands([
  {
    id: "app.settings.open",
    title: "Settings",
    category: "Global",
    keywords: ["preferences", "configuration", "config", "options"],
    icon: "settings",
    available: () => true,
    run: () => openSettings(),
  },
]);
