**D1 messaging reframe is shipping now regardless** — web-marketing copy +
README + gateway/README reframed so the SELF-HOSTABLE `gateway/` is the
offering (run your own portable Drive/Docs-equivalent with chan's IDE on it);
the chan-hosted online service is positioned as experimental + disabled by
default. Tunnel stays a CORE chan capability.

Decision needed: how deep to document running your OWN online service this
round. For option (a) or (b), @@LaneE needs your infra specifics — use **[F]
follow-up** to drop:

- DNS provider + wildcard setup
- cert method: certbot **dns-01** wildcard vs **http-01**
- nginx vhost layout for id.chan.app + workspace.chan.app
- how much of your private chan-prod-setup repo to mirror (oauth config, user
  enrollment, workspace sharing)

Not urgent — the reframe lands without this; the deep-dive can follow.
