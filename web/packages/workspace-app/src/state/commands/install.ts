// Load every command module for its registration side effect. Import
// this once (the launcher does) before reading the catalog, so all
// categories are present. Each module calls registerCommands at load.

import "./core";
import "./global";
import "./workspace";
import "./search";
import "./diagram";
import "./editor";
import "./terminal";
import "./dashboard";
import "./graph";
import "./panes";
