-- User avatar URL.
--
-- Surfaced from the OAuth provider's userinfo on sign-in: GitHub's
-- avatar_url, Google's picture, GitLab's avatar_url. Microsoft's
-- /me userinfo doesn't carry a picture URL (Graph /me/photo/$value
-- needs an extra Bearer call); left NULL until that lands.
--
-- Pure URL string, no caching or proxying. The browser fetches the
-- image directly from the provider CDN. Stored nullable because
-- providers (and individual users) can hide avatars; the SPA falls
-- back to an initial-circle placeholder when the column is NULL.

ALTER TABLE users ADD COLUMN avatar_url text;
