// identity-service public API client. Bearer wrapper, error type,
// and credentialed fetch live in chan-web-common.

import { request } from "chan-web-common/api";

export { HttpError } from "chan-web-common/api";

export type User = {
  id: string;
  email: string;
  display_name: string | null;
  username: string;
  username_edits: number;
  created_at: string;
  updated_at: string;
  avatar_url: string | null;
};

export type Drive = {
  /// Drive slug, used as the path on `{user}.drive.chan.app`.
  drive: string;
  /// Display label. Defaults to the slug until per-tunnel labels
  /// land in the chan-tunnel Hello frame.
  label: string;
  public: boolean;
  /// "online" while the tunnel registration is live. drive-proxy
  /// only reports online drives today; future health states
  /// ("degraded", "offline") land here.
  status: "online";
};

export type Me = {
  user: User;
  /// Live tunnel snapshot for the signed-in user, sourced from
  /// drive-proxy admin. Empty when nothing is connected.
  drives: Drive[];
  /// Resolved feature flags for this user. Map of flag key -> bool.
  /// A flag absent from the map = does not exist in the registry
  /// (treat as off). Fresh deploys ship `share_drives` off.
  flags: Record<string, boolean>;
};

export type UsernameResponse = {
  username: string;
  edits_remaining: number;
};

export type Token = {
  id: string;
  label: string;
  expires_at: string | null;
  created_at: string;
  revoked_at: string | null;
  last_used_at: string | null;
};

export type CreatedToken = Token & { secret: string };

export type AuditEntry = {
  id: number;
  ts: string;
  action: string;
  ip: string | null;
  user_agent: string | null;
};

export type ProvidersResponse = { providers: string[] };

export type DriveGrantRole = "viewer" | "editor";

export type DriveGrant = {
  id: string;
  owner_user_id: string;
  drive_name: string;
  grantee_email: string;
  /// null until the recipient signs in with a verified OAuth email
  /// matching grantee_email. Until then the grant is "pending" and
  /// the recipient cannot open the drive.
  grantee_user_id: string | null;
  role: DriveGrantRole;
  created_at: string;
  accepted_at: string | null;
};

export type OwnedDriveSummary = {
  drive_name: string;
  grant_count: number;
};

export type IncomingShare = {
  grant_id: string;
  owner_user_id: string;
  owner_username: string;
  owner_display_name: string | null;
  owner_avatar_url: string | null;
  drive_name: string;
  role: DriveGrantRole;
  accepted_at: string;
};

export const api = {
  providers: () => request<ProvidersResponse>("/api/providers"),
  me: () => request<Me>("/api/me"),
  logout: () => request<void>("/api/logout", { method: "POST" }),
  deleteAccount: () => request<void>("/api/profile", { method: "DELETE" }),

  updateUsername: (username: string) =>
    request<UsernameResponse>("/api/me/username", {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ username }),
    }),

  listTokens: () => request<Token[]>("/api/tokens"),
  createToken: (label: string, expires_in: number | null) =>
    request<CreatedToken>("/api/tokens", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ label, expires_in }),
    }),
  revokeToken: (id: string) =>
    request<void>(`/api/tokens/${id}`, { method: "DELETE" }),
  tokenAudit: (id: string) =>
    request<AuditEntry[]>(`/api/tokens/${id}/audit`),

  /// Build the entry URL for a drive. The server mints a 30s
  /// drive-gate JWT inside the 303 Location, so we never see the
  /// token here. We hand the URL to the browser via location.assign
  /// (or an anchor href); it follows the 303 to the wildcard
  /// subdomain, drive-proxy validates, sets the session cookie, and
  /// 303s to the clean URL.
  driveOpenUrl: (user: string, drive: string): string => {
    const u = encodeURIComponent(user);
    const d = encodeURIComponent(drive);
    return `/api/drives/open?u=${u}&d=${d}`;
  },

  /// Public, copyable share link. Anyone with this URL who can sign
  /// in via an OAuth provider whose verified email matches a grant
  /// the owner created will be admitted. Hand-distributed (email,
  /// chat, etc.) -- identity-service does not send the message.
  shareUrl: (owner: string, drive: string): string => {
    const o = encodeURIComponent(owner);
    const d = encodeURIComponent(drive);
    // Absolute URL so the copy-button result works after paste
    // anywhere. window.location.origin is the SPA's own origin (the
    // identity service); same hostname that handles /s/:owner/:drive.
    return `${window.location.origin}/s/${o}/${d}`;
  },

  createDrive: (drive_name: string) =>
    request<{
      id: string;
      owner_user_id: string;
      drive_name: string;
      created_at: string;
    }>("/api/drives", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ drive_name }),
    }),
  deleteDrive: (drive: string) =>
    request<void>(`/api/drives/${encodeURIComponent(drive)}`, { method: "DELETE" }),

  listDriveGrants: (drive: string) =>
    request<DriveGrant[]>(`/api/drives/${encodeURIComponent(drive)}/grants`),
  addDriveGrant: (drive: string, grantee_email: string, role: DriveGrantRole) =>
    request<DriveGrant>(`/api/drives/${encodeURIComponent(drive)}/grants`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ grantee_email, role }),
    }),
  deleteDriveGrant: (id: string) =>
    request<void>(`/api/grants/${id}`, { method: "DELETE" }),

  listOwnedDrives: () => request<OwnedDriveSummary[]>("/api/drives/owned"),
  listIncomingShares: () => request<IncomingShare[]>("/api/drives/incoming"),
};
