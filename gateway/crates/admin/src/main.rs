//! chan-gateway-admin: command-line admin for the chan.app gateway.
//!
//! Talks to profile-service's `/v1/admin/*` tree (and the small
//! handful of non-admin routes that already exist for cross-service
//! reads). Authenticates with `CHAN_ADMIN_TOKEN`, which must match
//! `PROFILE_ADMIN_TOKEN` on profile-service.
//!
//! Output is shell-friendly: human-readable tables on a TTY,
//! `--json` everywhere for piping into jq. Exit codes:
//!
//!   0  success
//!   1  upstream / network / config error
//!   2  user input error (bad uuid, missing arg, etc.)
//!   3  not found  (no row for the user/token id)
//!
//! Tunnel ps / kill / watch land in a follow-up that wires the
//! drive-proxy admin endpoint.

use std::process::ExitCode;

use anyhow::{anyhow, Context};
use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand};
use comfy_table::{presets::NOTHING, Cell, Table};
use reqwest::{header, Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const EXIT_INPUT: u8 = 2;
const EXIT_NOT_FOUND: u8 = 3;

#[derive(Parser)]
#[command(
    name = "chan-gateway-admin",
    version,
    about = "Admin CLI for chan-gateway (users, tokens, audit).",
    propagate_version = true
)]
struct Cli {
    /// HTTP URL of profile-service. Defaults to
    /// CHAN_ADMIN_PROFILE_URL or http://127.0.0.1:7001.
    #[arg(long, global = true, env = "CHAN_ADMIN_PROFILE_URL")]
    profile_url: Option<String>,

    /// HTTP URL of drive-proxy (used by `tunnel` subcommands).
    /// Defaults to CHAN_ADMIN_DRIVE_URL or http://127.0.0.1:7002.
    #[arg(long, global = true, env = "CHAN_ADMIN_DRIVE_URL")]
    drive_url: Option<String>,

    /// Bearer matching profile-service's PROFILE_ADMIN_TOKEN and
    /// drive-proxy's DRIVE_ADMIN_TOKEN. Single-token deployments
    /// share one secret across both services; deployments that
    /// rotate them independently can override per-call with the
    /// dedicated env vars.
    #[arg(long, global = true, env = "CHAN_ADMIN_TOKEN")]
    token: Option<String>,

    /// Emit JSON instead of a human-readable table.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Manage users.
    User {
        #[command(subcommand)]
        cmd: UserCmd,
    },
    /// Manage personal access tokens.
    Token {
        #[command(subcommand)]
        cmd: TokenCmd,
    },
    /// Inspect and kill live tunnels (live drive-proxy registry).
    Tunnel {
        #[command(subcommand)]
        cmd: TunnelCmd,
    },
    /// Manage feature flags (default + per-user overrides).
    Flag {
        #[command(subcommand)]
        cmd: FlagCmd,
    },
}

#[derive(Subcommand)]
enum FlagCmd {
    /// List every registered flag with its override count.
    List,
    /// Create or update a flag. Re-issuing for the same key bumps
    /// `default_enabled` and (optionally) the description.
    Create {
        key: String,
        /// Default the flag to ON for every user. Mutually exclusive
        /// with --default-off; if neither is given, defaults to OFF.
        #[arg(long, conflicts_with = "default_off")]
        default_on: bool,
        #[arg(long)]
        default_off: bool,
        #[arg(long)]
        description: Option<String>,
    },
    /// Delete a flag and (via FK cascade) every override on it.
    Delete {
        key: String,
        #[arg(long, help = "skip the y/N prompt")]
        yes: bool,
    },
    /// Grant a flag to one user. <ident> is a uuid, email, or
    /// username. Defaults to --enabled; pass --disabled to record
    /// an explicit "deny" override when the flag default is on.
    Grant {
        key: String,
        ident: String,
        #[arg(long, conflicts_with = "disabled")]
        enabled: bool,
        #[arg(long)]
        disabled: bool,
    },
    /// Clear the per-user override on a flag. After this the user
    /// resolves the flag from its default again.
    Revoke { key: String, ident: String },
    /// List per-user overrides on a flag.
    Overrides { key: String },
}

#[derive(Subcommand)]
enum TunnelCmd {
    /// Snapshot every registered tunnel (`ps`-style).
    Ps {
        /// Filter to one user.
        #[arg(long)]
        user: Option<String>,
    },
    /// Force a tunnel offline by (user, drive). The chan serve
    /// peer is free to reconnect.
    Kill { user: String, drive: String },
    /// Live snapshot stream (SSE). Re-renders the table every
    /// second until Ctrl-C.
    Watch {
        /// Filter to one user.
        #[arg(long)]
        user: Option<String>,
    },
}

#[derive(Subcommand)]
enum UserCmd {
    /// List users with optional filters.
    List(UserListArgs),
    /// Show one user. <ident> is a uuid, email, or username.
    Get { ident: String },
    /// Create a user (provisioned without OAuth identities).
    Create {
        #[arg(long)]
        email: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// Update display name on a user. Email rewrite has its own
    /// admin-only subcommand (`change-email`) because it pivots the
    /// identity-linking key.
    Update {
        ident: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// Rewrite a user's email (admin only). Logged in auth_audit.
    /// Required because email is the identity-linking key in
    /// upsert_by_identity, so this is treated as a privileged op.
    ChangeEmail {
        ident: String,
        #[arg(long)]
        email: String,
        #[arg(long, help = "skip the y/N prompt")]
        yes: bool,
    },
    /// Rename a user's public handle (consumes one of their cap-4
    /// rename slots).
    Rename { ident: String, username: String },
    /// Hard-delete a user (cascades identities + tokens + audit).
    Delete {
        ident: String,
        #[arg(long, help = "skip the y/N prompt")]
        yes: bool,
    },
    /// Block a user: revokes all live tokens, refuses fresh logins,
    /// evicts every live tunnel they had registered.
    Block {
        ident: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Unblock. Existing tokens stay revoked; reissue if needed.
    Unblock { ident: String },
    /// Show login / logout / block events for a user.
    Audit {
        ident: String,
        #[arg(long, default_value_t = 50)]
        limit: i64,
    },
    /// List a user's tokens.
    Tokens { ident: String },
}

#[derive(Args)]
struct UserListArgs {
    /// Substring filter on email (case-insensitive).
    #[arg(long)]
    email: Option<String>,
    /// Exact match on username.
    #[arg(long)]
    username: Option<String>,
    /// Show only blocked accounts.
    #[arg(long, conflicts_with = "active")]
    blocked: bool,
    /// Show only non-blocked accounts.
    #[arg(long)]
    active: bool,
    #[arg(long, default_value_t = 100)]
    limit: i64,
    #[arg(long, default_value_t = 0)]
    offset: i64,
}

#[derive(Subcommand)]
enum TokenCmd {
    /// List tokens for a user.
    List { ident: String },
    /// Revoke a token by id.
    Revoke { token_id: Uuid },
    /// Show audit log for a token.
    Audit {
        token_id: Uuid,
        #[arg(long, default_value_t = 100)]
        limit: i64,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // Walk the chain so the operator sees the upstream
            // status alongside the operation that failed.
            eprintln!("error: {e:#}");
            ExitCode::from(exit_code_for(&e))
        }
    }
}

fn exit_code_for(e: &anyhow::Error) -> u8 {
    if let Some(ce) = e.downcast_ref::<ClientError>() {
        return match ce {
            ClientError::NotFound => EXIT_NOT_FOUND,
            ClientError::BadInput(_) => EXIT_INPUT,
            _ => 1,
        };
    }
    1
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let token = cli
        .token
        .clone()
        .ok_or_else(|| anyhow!("CHAN_ADMIN_TOKEN not set; pass --token or export it"))?;
    let json = cli.json;
    match cli.cmd {
        Cmd::User {
            cmd: UserCmd::Block { ident, reason },
        } => {
            // Block needs both clients: profile holds the canonical
            // block + token revoke, drive-proxy holds live tunnel
            // registrations that must be severed so the cookie-
            // session bypass (existing tunnels surviving an admin
            // block) is closed. Profile first so a drive-proxy
            // outage doesn't leave the user un-blocked.
            let profile = build_profile_client(cli.profile_url.as_deref(), &token)?;
            let u = profile.resolve_user(&ident).await?;
            let blocked = profile.block_user(u.id, reason.as_deref()).await?;
            render_users(std::slice::from_ref(&blocked), json);
            match build_drive_client(cli.drive_url.as_deref(), &token) {
                Ok(drive) => match drive.kill_user_tunnels(&blocked.username).await {
                    Ok(killed) if killed > 0 => {
                        eprintln!("evicted {killed} live tunnel(s)");
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!(
                        "warning: profile block applied but tunnel eviction failed: {e:#}"
                    ),
                },
                Err(e) => {
                    eprintln!("warning: profile block applied but drive client unavailable: {e:#}")
                }
            }
            Ok(())
        }
        Cmd::User { cmd } => {
            let client = build_profile_client(cli.profile_url.as_deref(), &token)?;
            user(&client, json, cmd).await
        }
        Cmd::Token { cmd } => {
            let client = build_profile_client(cli.profile_url.as_deref(), &token)?;
            token_cmd(&client, json, cmd).await
        }
        Cmd::Tunnel { cmd } => {
            let client = build_drive_client(cli.drive_url.as_deref(), &token)?;
            tunnel_cmd(&client, json, cmd).await
        }
        Cmd::Flag { cmd } => {
            let client = build_profile_client(cli.profile_url.as_deref(), &token)?;
            flag_cmd(&client, json, cmd).await
        }
    }
}

fn build_profile_client(url: Option<&str>, token: &str) -> anyhow::Result<AdminClient> {
    let url = url
        .map(|s| s.to_string())
        .unwrap_or_else(|| "http://127.0.0.1:7001".to_string());
    AdminClient::new(url, token.to_string()).context("build profile admin client")
}

fn build_drive_client(url: Option<&str>, token: &str) -> anyhow::Result<DriveClient> {
    let url = url
        .map(|s| s.to_string())
        .unwrap_or_else(|| "http://127.0.0.1:7002".to_string());
    DriveClient::new(url, token.to_string()).context("build drive admin client")
}

// ---------------------------------------------------------------------------
// Subcommand handlers
// ---------------------------------------------------------------------------

async fn user(c: &AdminClient, json: bool, cmd: UserCmd) -> anyhow::Result<()> {
    match cmd {
        UserCmd::List(args) => {
            let blocked = if args.blocked {
                Some(true)
            } else if args.active {
                Some(false)
            } else {
                None
            };
            let users = c
                .list_users(
                    args.email.as_deref(),
                    args.username.as_deref(),
                    blocked,
                    args.limit,
                    args.offset,
                )
                .await?;
            render_users(&users, json);
        }
        UserCmd::Get { ident } => {
            let u = c.resolve_user(&ident).await?;
            render_users(std::slice::from_ref(&u), json);
        }
        UserCmd::Create { email, name } => {
            let u = c.create_user(&email, name.as_deref()).await?;
            render_users(std::slice::from_ref(&u), json);
        }
        UserCmd::Update { ident, name } => {
            if name.is_none() {
                return Err(anyhow!("nothing to update; pass --name"));
            }
            let u = c.resolve_user(&ident).await?;
            let updated = c.update_user(u.id, name.as_deref()).await?;
            render_users(std::slice::from_ref(&updated), json);
        }
        UserCmd::ChangeEmail { ident, email, yes } => {
            let u = c.resolve_user(&ident).await?;
            if !yes
                && !confirm(&format!(
                    "rewrite email for {} <{}> to <{email}>?",
                    u.username, u.email
                ))?
            {
                return Err(anyhow!("aborted"));
            }
            let updated = c.change_email(u.id, &email).await?;
            render_users(std::slice::from_ref(&updated), json);
        }
        UserCmd::Rename { ident, username } => {
            let u = c.resolve_user(&ident).await?;
            let renamed = c.update_username(u.id, &username).await?;
            render_users(std::slice::from_ref(&renamed), json);
        }
        UserCmd::Delete { ident, yes } => {
            let u = c.resolve_user(&ident).await?;
            if !yes && !confirm(&format!("delete user {} <{}>?", u.username, u.email))? {
                return Err(anyhow!("aborted"));
            }
            c.delete_user(u.id).await?;
            eprintln!("deleted {}", u.id);
        }
        UserCmd::Block { .. } => {
            // Handled in `run` so it can use both profile + drive
            // clients (profile.block_user followed by drive.kill_
            // user_tunnels). Reaching this arm means the dispatch
            // forgot to intercept; fail loudly.
            unreachable!("UserCmd::Block must be intercepted in run()");
        }
        UserCmd::Unblock { ident } => {
            let u = c.resolve_user(&ident).await?;
            let unblocked = c.unblock_user(u.id).await?;
            render_users(std::slice::from_ref(&unblocked), json);
        }
        UserCmd::Audit { ident, limit } => {
            let u = c.resolve_user(&ident).await?;
            let audit = c.user_audit(u.id, limit).await?;
            render_audit(&audit, json);
        }
        UserCmd::Tokens { ident } => {
            let u = c.resolve_user(&ident).await?;
            let tokens = c.user_tokens(u.id).await?;
            render_tokens(&tokens, json);
        }
    }
    Ok(())
}

async fn flag_cmd(c: &AdminClient, json: bool, cmd: FlagCmd) -> anyhow::Result<()> {
    match cmd {
        FlagCmd::List => {
            let rows = c.list_flags().await?;
            render_flags(&rows, json);
        }
        FlagCmd::Create {
            key,
            default_on,
            default_off: _, // mutually exclusive with default_on; clap enforces
            description,
        } => {
            // Default is OFF (closed allowlist) when neither flag is
            // given; --default-on flips it. Mutual exclusion is wired
            // at the clap layer above so we don't have to defend
            // against both being true here.
            let default_enabled = default_on;
            let row = c
                .upsert_flag(&key, description.as_deref(), default_enabled)
                .await?;
            render_flags(
                &[FeatureFlagSummary {
                    key: row.key,
                    description: row.description,
                    default_enabled: row.default_enabled,
                    override_count: 0,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                }],
                json,
            );
        }
        FlagCmd::Delete { key, yes } => {
            if !yes && !confirm(&format!("delete flag {key} and every override on it?"))? {
                return Err(anyhow!("aborted"));
            }
            c.delete_flag(&key).await?;
            eprintln!("deleted {key}");
        }
        FlagCmd::Grant {
            key,
            ident,
            enabled: _,
            disabled,
        } => {
            // --enabled is the default when neither flag is passed.
            // --disabled records an explicit "deny" override against
            // a default-on flag.
            let enabled = !disabled;
            let user = c.resolve_user(&ident).await?;
            let row = c.upsert_flag_override(&key, user.id, enabled).await?;
            render_overrides(&[row], json);
        }
        FlagCmd::Revoke { key, ident } => {
            let user = c.resolve_user(&ident).await?;
            c.delete_flag_override(&key, user.id).await?;
            eprintln!("cleared override on {key} for {}", user.username);
        }
        FlagCmd::Overrides { key } => {
            let rows = c.list_flag_overrides(&key).await?;
            render_overrides(&rows, json);
        }
    }
    Ok(())
}

async fn token_cmd(c: &AdminClient, json: bool, cmd: TokenCmd) -> anyhow::Result<()> {
    match cmd {
        TokenCmd::List { ident } => {
            let u = c.resolve_user(&ident).await?;
            let tokens = c.user_tokens(u.id).await?;
            render_tokens(&tokens, json);
        }
        TokenCmd::Revoke { token_id } => {
            c.revoke_token(token_id).await?;
            eprintln!("revoked {token_id}");
        }
        TokenCmd::Audit { token_id, limit } => {
            let entries = c.token_audit(token_id, limit).await?;
            render_token_audit(&entries, json);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AdminClient {
    base: url::Url,
    http: Client,
    token: String,
}

#[derive(Debug)]
enum ClientError {
    BadInput(String),
    NotFound,
    Upstream { status: StatusCode, body: String },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::BadInput(m) => write!(f, "{m}"),
            ClientError::NotFound => write!(f, "not found"),
            ClientError::Upstream { status, body } => write!(f, "upstream {status}: {body}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl AdminClient {
    fn new(base_url: String, token: String) -> anyhow::Result<Self> {
        let base =
            url::Url::parse(&base_url).with_context(|| format!("parse profile url: {base_url}"))?;
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(2))
            .user_agent(concat!("chan-gateway-admin/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { base, http, token })
    }

    fn url(&self, path: &str) -> url::Url {
        let mut u = self.base.clone();
        u.set_path(path);
        u.set_query(None);
        u
    }

    fn req(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, self.url(path))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
    }

    /// Resolve <ident> -> User. Order: uuid, email substring (must
    /// match exactly one row), username exact match. The list
    /// endpoint enforces case-insensitivity for both fields.
    async fn resolve_user(&self, ident: &str) -> anyhow::Result<User> {
        if let Ok(id) = Uuid::parse_str(ident) {
            return self.get_user(id).await;
        }
        if ident.contains('@') {
            let mut hits = self
                .list_users(Some(ident), None, None, 5, 0)
                .await?
                .into_iter()
                .filter(|u| u.email.eq_ignore_ascii_case(ident))
                .collect::<Vec<_>>();
            return match hits.len() {
                1 => Ok(hits.remove(0)),
                0 => Err(ClientError::NotFound.into()),
                _ => Err(anyhow!("ambiguous email: {} matches", hits.len())),
            };
        }
        let mut hits = self.list_users(None, Some(ident), None, 2, 0).await?;
        match hits.len() {
            1 => Ok(hits.remove(0)),
            0 => Err(ClientError::NotFound.into()),
            _ => Err(anyhow!("ambiguous username; multiple users match")),
        }
    }

    async fn get_user(&self, id: Uuid) -> anyhow::Result<User> {
        let res = self
            .req(Method::GET, &format!("/v1/users/{id}"))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn list_users(
        &self,
        email: Option<&str>,
        username: Option<&str>,
        blocked: Option<bool>,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<User>> {
        let mut url = self.url("/v1/admin/users");
        {
            let mut q = url.query_pairs_mut();
            if let Some(e) = email {
                q.append_pair("email", e);
            }
            if let Some(u) = username {
                q.append_pair("username", u);
            }
            if let Some(b) = blocked {
                q.append_pair("blocked", &b.to_string());
            }
            q.append_pair("limit", &limit.to_string());
            q.append_pair("offset", &offset.to_string());
        }
        let res = self
            .http
            .get(url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn create_user(&self, email: &str, name: Option<&str>) -> anyhow::Result<User> {
        let res = self
            .req(Method::POST, "/v1/users")
            .json(&serde_json::json!({"email": email, "display_name": name}))
            .send()
            .await?;
        match res.status() {
            StatusCode::CREATED => Ok(res.json().await?),
            StatusCode::CONFLICT => Err(ClientError::BadInput("email already taken".into()).into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn update_user(&self, id: Uuid, name: Option<&str>) -> anyhow::Result<User> {
        let res = self
            .req(Method::PATCH, &format!("/v1/users/{id}"))
            .json(&serde_json::json!({"display_name": name}))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    /// Admin-only email rewrite. Profile-service logs an
    /// `email_changed` auth_audit row server-side; we just need to
    /// surface the resulting User row.
    async fn change_email(&self, id: Uuid, email: &str) -> anyhow::Result<User> {
        let res = self
            .req(Method::POST, &format!("/v1/admin/users/{id}/email"))
            .json(&serde_json::json!({"email": email}))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            StatusCode::BAD_REQUEST => Err(ClientError::BadInput(read_body(res).await).into()),
            StatusCode::CONFLICT => Err(ClientError::BadInput(read_body(res).await).into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn update_username(&self, id: Uuid, username: &str) -> anyhow::Result<User> {
        let res = self
            .req(Method::PATCH, &format!("/v1/users/{id}/username"))
            .json(&serde_json::json!({"username": username}))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            StatusCode::BAD_REQUEST => Err(ClientError::BadInput(read_body(res).await).into()),
            StatusCode::CONFLICT => Err(ClientError::BadInput(read_body(res).await).into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn delete_user(&self, id: Uuid) -> anyhow::Result<()> {
        let res = self
            .req(Method::DELETE, &format!("/v1/users/{id}"))
            .send()
            .await?;
        match res.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn block_user(&self, id: Uuid, reason: Option<&str>) -> anyhow::Result<User> {
        let res = self
            .req(Method::POST, &format!("/v1/admin/users/{id}/block"))
            .json(&serde_json::json!({"reason": reason}))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn unblock_user(&self, id: Uuid) -> anyhow::Result<User> {
        let res = self
            .req(Method::POST, &format!("/v1/admin/users/{id}/unblock"))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn user_audit(&self, id: Uuid, limit: i64) -> anyhow::Result<Vec<AuthAudit>> {
        let mut url = self.url(&format!("/v1/admin/users/{id}/auth-audit"));
        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string());
        let res = self
            .http
            .get(url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn user_tokens(&self, id: Uuid) -> anyhow::Result<Vec<TokenView>> {
        let res = self
            .req(Method::GET, &format!("/v1/admin/users/{id}/tokens"))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn revoke_token(&self, id: Uuid) -> anyhow::Result<()> {
        let res = self
            .req(Method::POST, &format!("/v1/admin/tokens/{id}/revoke"))
            .send()
            .await?;
        match res.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn list_flags(&self) -> anyhow::Result<Vec<FeatureFlagSummary>> {
        let res = self.req(Method::GET, "/v1/admin/flags").send().await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn upsert_flag(
        &self,
        key: &str,
        description: Option<&str>,
        default_enabled: bool,
    ) -> anyhow::Result<FeatureFlag> {
        let res = self
            .req(Method::POST, "/v1/admin/flags")
            .json(&serde_json::json!({
                "key": key,
                "description": description,
                "default_enabled": default_enabled,
            }))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK | StatusCode::CREATED => Ok(res.json().await?),
            StatusCode::BAD_REQUEST => Err(ClientError::BadInput(read_body(res).await).into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn delete_flag(&self, key: &str) -> anyhow::Result<()> {
        let res = self
            .req(Method::DELETE, &format!("/v1/admin/flags/{key}"))
            .send()
            .await?;
        match res.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn list_flag_overrides(&self, key: &str) -> anyhow::Result<Vec<FeatureFlagOverride>> {
        let res = self
            .req(Method::GET, &format!("/v1/admin/flags/{key}/overrides"))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn upsert_flag_override(
        &self,
        key: &str,
        user_id: Uuid,
        enabled: bool,
    ) -> anyhow::Result<FeatureFlagOverride> {
        let res = self
            .req(Method::POST, &format!("/v1/admin/flags/{key}/overrides"))
            .json(&serde_json::json!({"user_id": user_id, "enabled": enabled}))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn delete_flag_override(&self, key: &str, user_id: Uuid) -> anyhow::Result<()> {
        let res = self
            .req(
                Method::DELETE,
                &format!("/v1/admin/flags/{key}/overrides/{user_id}"),
            )
            .send()
            .await?;
        match res.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn token_audit(&self, id: Uuid, limit: i64) -> anyhow::Result<Vec<TokenAudit>> {
        let mut url = self.url(&format!("/v1/admin/tokens/{id}/audit"));
        url.query_pairs_mut()
            .append_pair("limit", &limit.to_string());
        let res = self
            .http
            .get(url)
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }
}

async fn upstream(status: StatusCode, res: reqwest::Response) -> ClientError {
    let body = read_body(res).await;
    ClientError::Upstream { status, body }
}

async fn read_body(res: reqwest::Response) -> String {
    res.text()
        .await
        .unwrap_or_else(|e| format!("<read error: {e}>"))
}

// ---------------------------------------------------------------------------
// Drive (tunnel) admin client
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DriveClient {
    base: url::Url,
    http: Client,
    token: String,
}

impl DriveClient {
    fn new(base_url: String, token: String) -> anyhow::Result<Self> {
        let base =
            url::Url::parse(&base_url).with_context(|| format!("parse drive url: {base_url}"))?;
        let http = Client::builder()
            // Watch streams idle between snapshots; disable the
            // global timeout for it. Per-call timeouts are still
            // enforced on the request builder. connect_timeout
            // applies to the TCP handshake only, so it remains safe
            // for the long-lived SSE stream.
            .connect_timeout(std::time::Duration::from_secs(2))
            .user_agent(concat!("chan-gateway-admin/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { base, http, token })
    }

    fn url(&self, path: &str) -> url::Url {
        let mut u = self.base.clone();
        u.set_path(path);
        u.set_query(None);
        u
    }

    async fn list(&self) -> anyhow::Result<Vec<TunnelView>> {
        let res = self
            .http
            .get(self.url("/admin/v1/tunnels"))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => Ok(res.json().await?),
            s => Err(upstream(s, res).await.into()),
        }
    }

    async fn kill(&self, user: &str, drive: &str) -> anyhow::Result<()> {
        let path = format!(
            "/admin/v1/tunnels/{}/{}/kill",
            urlencoding::encode_path(user),
            urlencoding::encode_path(drive),
        );
        let res = self
            .http
            .post(self.url(&path))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;
        match res.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::NOT_FOUND => Err(ClientError::NotFound.into()),
            s => Err(upstream(s, res).await.into()),
        }
    }

    /// Bulk-evict every tunnel for a user. "Nothing to kill" is
    /// success with `killed: 0`; idempotent retry is safe.
    async fn kill_user_tunnels(&self, user: &str) -> anyhow::Result<usize> {
        let path = format!(
            "/admin/v1/users/{}/tunnels/kill",
            urlencoding::encode_path(user),
        );
        let res = self
            .http
            .post(self.url(&path))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;
        match res.status() {
            StatusCode::OK => {
                let body: KillUserTunnelsResponse = res.json().await?;
                Ok(body.killed)
            }
            StatusCode::NO_CONTENT => Ok(0),
            s => Err(upstream(s, res).await.into()),
        }
    }

    /// SSE stream of `event: snapshot` frames. Yields parsed
    /// `Vec<TunnelView>` per event; ignores malformed events.
    async fn watch(&self) -> anyhow::Result<reqwest::Response> {
        let res = self
            .http
            .get(self.url("/admin/v1/tunnels/watch"))
            .header(header::AUTHORIZATION, format!("Bearer {}", self.token))
            .header(header::ACCEPT, "text/event-stream")
            .send()
            .await?;
        if !res.status().is_success() {
            let status = res.status();
            return Err(upstream(status, res).await.into());
        }
        Ok(res)
    }
}

/// Tiny helper: percent-encode path segments without pulling in a
/// real urlencoding crate. Limits the alphabet to what a username
/// or drive slug can contain (`[a-z0-9-]` plus `_` and `.` for
/// drive names) so the typical path needs no escaping.
mod urlencoding {
    pub fn encode_path(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            match b {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
}

async fn tunnel_cmd(c: &DriveClient, json: bool, cmd: TunnelCmd) -> anyhow::Result<()> {
    match cmd {
        TunnelCmd::Ps { user } => {
            let mut tunnels = c.list().await?;
            if let Some(u) = user.as_deref() {
                tunnels.retain(|t| t.user == u);
            }
            render_tunnels(&tunnels, json);
        }
        TunnelCmd::Kill { user, drive } => {
            c.kill(&user, &drive).await?;
            eprintln!("killed {user}/{drive}");
        }
        TunnelCmd::Watch { user } => {
            watch_loop(c, user.as_deref(), json).await?;
        }
    }
    Ok(())
}

/// Read the SSE stream and re-render on every `snapshot` event.
/// Plain text mode clears the screen between renders so the
/// output looks like `top`; --json mode emits one JSON line per
/// event so it pipes into jq cleanly.
async fn watch_loop(c: &DriveClient, user_filter: Option<&str>, json: bool) -> anyhow::Result<()> {
    use std::io::Write;
    use tokio_stream::StreamExt;

    let res = c.watch().await?;
    let mut stream = res.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buf.extend_from_slice(&chunk);
        // SSE event = "\n\n"-delimited block. Process each.
        while let Some(pos) = find_subseq(&buf, b"\n\n") {
            let block = buf.drain(..pos + 2).collect::<Vec<_>>();
            if let Some(payload) = parse_sse_data(&block) {
                let mut tunnels: Vec<TunnelView> = match serde_json::from_slice(&payload) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if let Some(u) = user_filter {
                    tunnels.retain(|t| t.user == u);
                }
                if json {
                    print_json(&tunnels);
                    let _ = std::io::stdout().flush();
                } else {
                    // ANSI clear screen + home; mirrors `watch -n1`.
                    print!("\x1b[2J\x1b[H");
                    println!(
                        "chan-gateway-admin tunnel watch  ({})",
                        chrono::Local::now().format("%H:%M:%S")
                    );
                    render_tunnels(&tunnels, false);
                    let _ = std::io::stdout().flush();
                }
            }
        }
    }
    Ok(())
}

fn find_subseq(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Pull the concatenated `data:` field out of one SSE event block.
fn parse_sse_data(block: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(block).ok()?;
    let mut out = Vec::new();
    for line in s.split('\n') {
        if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !out.is_empty() {
                out.push(b'\n');
            }
            out.extend_from_slice(rest.as_bytes());
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
struct User {
    id: Uuid,
    email: String,
    display_name: Option<String>,
    username: String,
    username_edits: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[serde(default)]
    blocked_at: Option<DateTime<Utc>>,
    #[serde(default)]
    block_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AuthAudit {
    id: i64,
    user_id: Uuid,
    ts: DateTime<Utc>,
    action: String,
    ip: Option<String>,
    user_agent: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenView {
    id: Uuid,
    user_id: Uuid,
    label: String,
    expires_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenAudit {
    id: i64,
    token_id: Uuid,
    ts: DateTime<Utc>,
    action: String,
    ip: Option<String>,
    user_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
struct KillUserTunnelsResponse {
    killed: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FeatureFlag {
    key: String,
    description: String,
    default_enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FeatureFlagSummary {
    key: String,
    description: String,
    default_enabled: bool,
    override_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct FeatureFlagOverride {
    flag_key: String,
    user_id: Uuid,
    enabled: bool,
    set_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TunnelView {
    user: String,
    drive: String,
    public: bool,
    peer_addr: Option<String>,
    connected_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn render_users(rows: &[User], json: bool) {
    if json {
        print_json(rows);
        return;
    }
    let mut t = make_table();
    t.set_header(["ID", "USERNAME", "EMAIL", "NAME", "STATE", "CREATED"]);
    for u in rows {
        let state = if u.blocked_at.is_some() {
            match &u.block_reason {
                Some(r) => format!("blocked ({r})"),
                None => "blocked".to_string(),
            }
        } else {
            "active".to_string()
        };
        t.add_row([
            Cell::new(short_uuid(&u.id)),
            Cell::new(&u.username),
            Cell::new(&u.email),
            Cell::new(u.display_name.as_deref().unwrap_or("-")),
            Cell::new(state),
            Cell::new(u.created_at.format("%Y-%m-%d").to_string()),
        ]);
    }
    println!("{t}");
}

fn render_tokens(rows: &[TokenView], json: bool) {
    if json {
        print_json(rows);
        return;
    }
    let mut t = make_table();
    t.set_header(["ID", "LABEL", "STATE", "LAST USED", "EXPIRES", "CREATED"]);
    for r in rows {
        let state = if r.revoked_at.is_some() {
            "revoked"
        } else if r.expires_at.map(|e| e <= Utc::now()).unwrap_or(false) {
            "expired"
        } else {
            "active"
        };
        t.add_row([
            Cell::new(short_uuid(&r.id)),
            Cell::new(&r.label),
            Cell::new(state),
            Cell::new(fmt_dt_opt(r.last_used_at)),
            Cell::new(fmt_dt_opt(r.expires_at)),
            Cell::new(r.created_at.format("%Y-%m-%d").to_string()),
        ]);
    }
    println!("{t}");
}

fn render_audit(rows: &[AuthAudit], json: bool) {
    if json {
        print_json(rows);
        return;
    }
    let mut t = make_table();
    t.set_header(["WHEN", "ACTION", "IP", "AGENT", "NOTE"]);
    for r in rows {
        t.add_row([
            Cell::new(fmt_dt(r.ts)),
            Cell::new(&r.action),
            Cell::new(r.ip.as_deref().unwrap_or("-")),
            Cell::new(truncate(r.user_agent.as_deref().unwrap_or("-"), 32)),
            Cell::new(r.note.as_deref().unwrap_or("-")),
        ]);
    }
    println!("{t}");
}

fn render_tunnels(rows: &[TunnelView], json: bool) {
    if json {
        print_json(rows);
        return;
    }
    let mut t = make_table();
    t.set_header(["USER", "DRIVE", "PUBLIC", "PEER", "UPTIME", "CONNECTED"]);
    let now = Utc::now();
    for r in rows {
        let uptime = now
            .signed_duration_since(r.connected_at)
            .to_std()
            .map(format_duration)
            .unwrap_or_else(|_| "-".to_string());
        t.add_row([
            Cell::new(&r.user),
            Cell::new(&r.drive),
            Cell::new(if r.public { "yes" } else { "no" }),
            Cell::new(r.peer_addr.as_deref().unwrap_or("-")),
            Cell::new(uptime),
            Cell::new(fmt_dt(r.connected_at)),
        ]);
    }
    println!("{t}");
}

fn format_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m{}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d{}h", secs / 86400, (secs % 86400) / 3600)
    }
}

fn render_flags(rows: &[FeatureFlagSummary], json: bool) {
    if json {
        print_json(rows);
        return;
    }
    let mut t = make_table();
    t.set_header(["KEY", "DEFAULT", "OVERRIDES", "DESCRIPTION", "UPDATED"]);
    for r in rows {
        t.add_row([
            Cell::new(&r.key),
            Cell::new(if r.default_enabled { "on" } else { "off" }),
            Cell::new(r.override_count),
            Cell::new(truncate(&r.description, 40)),
            Cell::new(r.updated_at.format("%Y-%m-%d %H:%M").to_string()),
        ]);
    }
    println!("{t}");
}

fn render_overrides(rows: &[FeatureFlagOverride], json: bool) {
    if json {
        print_json(rows);
        return;
    }
    let mut t = make_table();
    t.set_header(["FLAG", "USER", "ENABLED", "SET AT"]);
    for r in rows {
        t.add_row([
            Cell::new(&r.flag_key),
            Cell::new(short_uuid(&r.user_id)),
            Cell::new(if r.enabled { "on" } else { "off" }),
            Cell::new(fmt_dt(r.set_at)),
        ]);
    }
    println!("{t}");
}

fn render_token_audit(rows: &[TokenAudit], json: bool) {
    if json {
        print_json(rows);
        return;
    }
    let mut t = make_table();
    t.set_header(["WHEN", "ACTION", "IP", "AGENT"]);
    for r in rows {
        t.add_row([
            Cell::new(fmt_dt(r.ts)),
            Cell::new(&r.action),
            Cell::new(r.ip.as_deref().unwrap_or("-")),
            Cell::new(truncate(r.user_agent.as_deref().unwrap_or("-"), 40)),
        ]);
    }
    println!("{t}");
}

fn make_table() -> Table {
    let mut t = Table::new();
    t.load_preset(NOTHING);
    t
}

fn print_json<T: Serialize + ?Sized>(v: &T) {
    match serde_json::to_string_pretty(v) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("error: serialize: {e}"),
    }
}

fn short_uuid(id: &Uuid) -> String {
    let s = id.simple().to_string();
    s[..8].to_string()
}

fn fmt_dt(d: DateTime<Utc>) -> String {
    d.format("%Y-%m-%d %H:%M").to_string()
}

fn fmt_dt_opt(d: Option<DateTime<Utc>>) -> String {
    match d {
        Some(t) => fmt_dt(t),
        None => "-".to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn confirm(prompt: &str) -> anyhow::Result<bool> {
    use std::io::{stdin, stdout, Write};
    eprint!("{prompt} [y/N] ");
    stdout().flush().ok();
    let mut buf = String::new();
    stdin().read_line(&mut buf)?;
    Ok(matches!(buf.trim(), "y" | "Y" | "yes"))
}
