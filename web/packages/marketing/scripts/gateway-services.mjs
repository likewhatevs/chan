// Single source of truth for the gateway service names, parsed from the
// Makefile's GATEWAY_RELEASE_CRATES. The gateway ships one .deb per service per
// arch; the deb name is `chan-gateway-<service>` and the install-page download
// id is `gateway-<service>-deb-<arch>`. Deriving the list from the Makefile (the
// same source the release build + release.yml use) means a service rename only
// touches the Makefile and can't drift the release-asset scripts or the install
// page off the real deb names.

import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
  "..",
  "..",
);

const makefile = readFileSync(path.join(repoRoot, "Makefile"), "utf8");
const match = makefile.match(/^GATEWAY_RELEASE_CRATES\s*:?=\s*(.+)$/m);
if (!match) {
  throw new Error(
    "GATEWAY_RELEASE_CRATES not found in Makefile; the marketing " +
      "release-asset scripts single-source the gateway service names from it.",
  );
}

/** Gateway service names, e.g. ["profile", "identity", "devserver-proxy", "admin"]. */
export const gatewayServices = match[1]
  .split("#")[0]
  .trim()
  .split(/\s+/)
  .filter(Boolean);
