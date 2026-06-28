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
