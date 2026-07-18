const releaseTagPattern = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?)$/;

export function versionFromTag(tag) {
  const match = String(tag ?? "").match(releaseTagPattern);
  if (!match) {
    throw new Error(`release tag must use vX.Y.Z or vX.Y.Z-rcN: ${tag}`);
  }
  return match[1];
}

export function validateReleaseTag(tag, label = "tag") {
  try {
    versionFromTag(tag);
  } catch {
    throw new Error(`${label} must use vX.Y.Z or vX.Y.Z-rcN`);
  }
}

export function gatewayPackageVersion(version) {
  return version.replace("-", ".");
}

export function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

// Minimal semver compare for X.Y.Z(-prerelease) versions (release tags are
// validated against releaseTagPattern): numeric core; a prerelease sorts
// before the same core without one; prerelease identifiers compare per
// semver (numeric < alphanumeric, numerics numerically, longer list after a
// shared prefix wins).
export function compareVersions(a, b) {
  const [coreA, preA] = splitPrerelease(a);
  const [coreB, preB] = splitPrerelease(b);
  for (let i = 0; i < 3; i += 1) {
    if (coreA[i] !== coreB[i]) return coreA[i] - coreB[i];
  }
  if (!preA && !preB) return 0;
  if (!preA) return 1;
  if (!preB) return -1;
  const idsA = preA.split(".");
  const idsB = preB.split(".");
  for (let i = 0; i < Math.max(idsA.length, idsB.length); i += 1) {
    const idA = idsA[i];
    const idB = idsB[i];
    if (idA === undefined) return -1;
    if (idB === undefined) return 1;
    const numA = /^\d+$/.test(idA);
    const numB = /^\d+$/.test(idB);
    if (numA && numB) {
      const diff = Number(idA) - Number(idB);
      if (diff !== 0) return diff;
    } else if (numA) {
      return -1;
    } else if (numB) {
      return 1;
    } else if (idA !== idB) {
      return idA < idB ? -1 : 1;
    }
  }
  return 0;
}

function splitPrerelease(version) {
  const dash = version.indexOf("-");
  const core = (dash === -1 ? version : version.slice(0, dash)).split(".").map(Number);
  const prerelease = dash === -1 ? "" : version.slice(dash + 1);
  return [core, prerelease];
}
