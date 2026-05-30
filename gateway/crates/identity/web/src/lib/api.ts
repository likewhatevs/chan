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

export type Workspace = {
  /// Workspace slug, used as the path on `{user}.workspace.chan.app`.
  workspace: string;
  /// Display label. Currently always the slug; the chan-tunnel Hello
  /// frame does not carry a per-tunnel label.
  label: string;
  public: boolean;
  /// "online" while the tunnel registration is live. workspace-proxy
  /// only reports online workspaces, so this is the only value.
  status: "online";
};

export type Me = {
  user: User;
  /// Live tunnel snapshot for the signed-in user, sourced from
  /// workspace-proxy admin. Empty when nothing is connected.
  workspaces: Workspace[];
  /// Resolved feature flags for this user. Map of flag key -> bool.
  /// A flag absent from the map = does not exist in the registry
  /// (treat as off). Fresh deploys ship `share_workspaces` off.
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

export type WorkspaceGrantRole = "viewer" | "editor";

export type WorkspaceGrant = {
  id: string;
  owner_user_id: string;
  workspace_name: string;
  grantee_email: string;
  /// null until the recipient signs in with a verified OAuth email
  /// matching grantee_email. Until then the grant is "pending" and
  /// the recipient cannot open the workspace.
  grantee_user_id: string | null;
  role: WorkspaceGrantRole;
  created_at: string;
  accepted_at: string | null;
};

export type OwnedWorkspaceSummary = {
  workspace_name: string;
  grant_count: number;
};

export type IncomingShare = {
  grant_id: string;
  owner_user_id: string;
  owner_username: string;
  owner_display_name: string | null;
  owner_avatar_url: string | null;
  workspace_name: string;
  role: WorkspaceGrantRole;
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

  /// Build the entry URL for a workspace. The server mints a 30s
  /// workspace-gate JWT inside the 303 Location, so we never see the
  /// token here. We hand the URL to the browser via location.assign
  /// (or an anchor href); it follows the 303 to the wildcard
  /// subdomain, workspace-proxy validates, sets the session cookie, and
  /// 303s to the clean URL.
  workspaceOpenUrl: (user: string, workspace: string): string => {
    const u = encodeURIComponent(user);
    const d = encodeURIComponent(workspace);
    return `/api/workspaces/open?u=${u}&d=${d}`;
  },

  /// Public, copyable share link. Anyone with this URL who can sign
  /// in via an OAuth provider whose verified email matches a grant
  /// the owner created will be admitted. Hand-distributed (email,
  /// chat, etc.) -- identity-service does not send the message.
  shareUrl: (owner: string, workspace: string): string => {
    const o = encodeURIComponent(owner);
    const d = encodeURIComponent(workspace);
    // Absolute URL so the copy-button result works after paste
    // anywhere. window.location.origin is the SPA's own origin (the
    // identity service); same hostname that handles /s/:owner/:workspace.
    return `${window.location.origin}/s/${o}/${d}`;
  },

  createWorkspace: (workspace_name: string) =>
    request<{
      id: string;
      owner_user_id: string;
      workspace_name: string;
      created_at: string;
    }>("/api/workspaces", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ workspace_name }),
    }),
  deleteWorkspace: (workspace: string) =>
    request<void>(`/api/workspaces/${encodeURIComponent(workspace)}`, { method: "DELETE" }),

  listWorkspaceGrants: (workspace: string) =>
    request<WorkspaceGrant[]>(`/api/workspaces/${encodeURIComponent(workspace)}/grants`),
  addWorkspaceGrant: (workspace: string, grantee_email: string, role: WorkspaceGrantRole) =>
    request<WorkspaceGrant>(`/api/workspaces/${encodeURIComponent(workspace)}/grants`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ grantee_email, role }),
    }),
  deleteWorkspaceGrant: (id: string) =>
    request<void>(`/api/grants/${id}`, { method: "DELETE" }),

  listOwnedWorkspaces: () => request<OwnedWorkspaceSummary[]>("/api/workspaces/owned"),
  listIncomingShares: () => request<IncomingShare[]>("/api/workspaces/incoming"),
};
