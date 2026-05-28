-- Drop the refresh-token table introduced in 0002.
--
-- The /v1/token/{refresh,revoke}, /v1/devices and /v1/jwks endpoints
-- were never wired up to a real client. PATs (api_tokens) cover the
-- chan CLI / chan-tunnel use case; entitlement bundles + JWT access
-- tokens come back if/when desktop or mobile clients ship.

DROP TABLE IF EXISTS refresh_tokens;
