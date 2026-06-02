# Item 1 outline (scope-first): prod-like-local gateway guide

For @@Lead review BEFORE writing. Goal (wave-3): a guide to stand up the
FULL gateway stack LOCALLY in a prod-like way (Mac: lima-vm + sdme like
@@Host's prod; Linux: local). EXPLAIN the DNS/cert choices (don't
prescribe), mirror @@Host's nginx vhost layout where sensible, mirror
LITTLE of chan-prod-setup. NOT ngrok, NOT real cloud.

## What already exists (consolidate, dedup)

- `gateway/docs/dev-setup.md` - sdme+Postgres loopback foundation: Lima
  shim, creds (chan/chan, chan_gateway[_test]), running services (-> refs
  scripts/dev/README.md), tests, connection reaper, sdme cheatsheet,
  troubleshooting. Reuses chan-prod-setup container configs already.
- `gateway/docs/testing-on-linux-and-macos.md` - the Postgres CONTAINER
  build (chan-psql.sdme: import base, fs build, create/start, lifecycle).
  Heavy overlap with dev-setup.md's Postgres half.
- `gateway/README.md ## Dev` - the 3-service run (profile :7001, identity
  :7000, workspace-proxy :7002, tunnel :7100), GitHub OAuth app,
  scripts/dev/{setup,run}.sh, frontends/npm.

All three stop at LOOPBACK (127.0.0.1, no front proxy, no TLS, no
wildcard host). That is exactly the prod-like gap to fill.

## Proposed target: ONE consolidated guide (extend dev-setup.md)

Keep dev-setup.md as the home; fold the container-build detail in from
testing-on-linux-and-macos.md (leave it a thin pointer or merge it).
Sections:

1. Overview - why prod-like-local: exercise the FULL topology + the
   cross-subdomain cookie isolation (id_session host-only on id.*,
   workspace_gate host-only + path-scoped on {user}.workspace.*) that
   loopback can't, using the same lima+sdme toolchain as prod.
2. Topology diagram - nginx front -> {identity:7000, profile:7001,
   workspace-proxy:7002 (+ h2c tunnel :7100)} -> Postgres:5432. On Mac,
   all inside the Lima VM.
3. Prerequisites + Postgres (consolidated sdme/Lima + container build).
4. The three services (env, OAuth app, scripts/dev runner).
5. NEW prod-like front layer:
   a. Hostnames + DNS: id.<domain>, <domain> apex, *.workspace.<domain>
      wildcard. Local options vs real DNS.
   b. TLS - EXPLAIN dns-01 (wildcard-capable, REQUIRED for
      *.workspace.*) vs http-01 (no wildcards); local (mkcert/local CA)
      vs prod (Let's Encrypt dns-01). Recommend, don't mandate.
   c. nginx vhosts mirroring @@Host's layout: server_name blocks, the
      /v1/tunnel h2c grpc_pass to :7100, wildcard proxy, the
      X-Forwarded-* hygiene workspace-proxy expects.
   d. End-to-end smoke: sign in at id.*, open-workspace handoff ->
      workspace_gate cookie -> {user}.workspace.<domain>/{workspace}/.
6. From local to a real VPS - ties to gateway/README .deb + systemd;
   what changes is just real DNS wildcard + LE dns-01.
7. Troubleshooting (consolidated).

## What I need from @@Host (tight; confirm/choose, derive the rest)

1. nginx vhosts: SKETCH a representative layout from the documented
   routes (apex /v1/tunnel grpc_pass + admin + healthz; wildcard tenant
   proxy; id.* -> identity) and have you confirm, OR do you want to drop
   in the real chan-prod-setup vhost skeleton? (mirror-LITTLE leans
   sketch+confirm.)
2. Local DNS+cert path to lead with: (a) real domain + LE dns-01
   wildcard, (b) local domain + mkcert/local CA + dnsmasq wildcard, or
   (c) explain both, recommend one. Which?
3. Service topology for prod-like-local: services as host binaries
   (cargo) with only Postgres in Lima (today's shape), OR all as
   systemd units inside Lima/sdme (closer to prod)? And nginx on host or
   in the VM?
4. DNS provider: name a representative one for the dns-01 example, or
   keep it provider-neutral?

If you'd rather I just sketch everything from the documented routes and
you correct it, say so and I'll write the draft with placeholders for
the 4 above.
