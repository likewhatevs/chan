-- Per-token scope list. workspace-proxy's `Validator::validate` already
-- returns a `scopes: Vec<String>` to chan-tunnel-server, which gates
-- the base "tunnel" scope and now the additive "tunnel.public" scope
-- (anonymous-readable workspace). Before this column the identity service
-- did not carry scopes and workspace-proxy injected `["tunnel"]` for every
-- successfully-validated PAT; the audit moved scope decisions to the
-- token, so the column has to actually exist.
--
-- Default `{tunnel}` keeps freshly-issued tokens at the safe, private
-- behaviour. Grant `tunnel.public` deliberately when minting a token
-- whose user is allowed to host anonymous workspaces.

ALTER TABLE api_tokens
  ADD COLUMN scopes text[] NOT NULL DEFAULT ARRAY['tunnel'];
