# DNS cutover runbook: chan.app -> GitHub Pages (Cloudflare, DNS-only)

This is a manual procedure for @@Alex to run in the Cloudflare dashboard
plus the GitHub repo settings. It is not automatable from the repo. Run
the steps in order. Because TTL is 300s, a full rollback propagates in
~5 minutes (step 8).

Current live state (verified via dig): Cloudflare nameservers
(`aspen` / `rommy.ns.cloudflare.com`), apex `A 173.208.147.114` (VPS,
**DNS-only** — not a Cloudflare proxy IP), an `AAAA 64:ff9b::add0:9372`
(NAT64 form of the same VPS IP), `www` CNAME -> `chan.app`, TTL 300.
Repo is `fiorix/chan`; Pages host is `fiorix.github.io`; the deploy
workflow already writes `CNAME` = `chan.app` into the artifact.

**Step 0 — Pre-flight: set the custom domain (no DNS change yet)**
- Confirm the Pages workflow is green on `main` (Actions tab) and that
  repo **Settings -> Pages** shows source = GitHub Actions.
- Set the custom domain explicitly: **Settings -> Pages -> Custom
  domain**, enter `chan.app`, Save. This step is required and is easy to
  miss. Under Actions-based publishing the `CNAME` file in the build
  artifact does NOT auto-set the domain; that auto-set only happens with
  legacy branch-based Pages publishing. Until the domain is set here,
  GitHub's edge has no mapping for `chan.app` and serves "Site not found"
  with no certificate.
- Once set, the custom-domain field shows a **DNS check failing** warning.
  That is expected until step 4, since DNS still points at the VPS.
  GitHub issues the Let's Encrypt cert after the DNS check passes.
- Leave TTL at 300 (already low enough).

**Step 1 — In Cloudflare DNS, replace the apex `A` record**
- Delete `A chan.app -> 173.208.147.114`.
- Add four `A` records for `chan.app`, **Proxy status = DNS only (grey
  cloud)**, all pointing at GitHub Pages:
  - `185.199.108.153`
  - `185.199.109.153`
  - `185.199.110.153`
  - `185.199.111.153`

**Step 2 — Replace the apex `AAAA` record**
- Delete `AAAA chan.app -> 64:ff9b::add0:9372` (stale VPS IPv6; leaving
  it would split IPv6 clients back to the VPS).
- Add four `AAAA` records for `chan.app`, **DNS only**:
  - `2606:50c0:8000::153`
  - `2606:50c0:8001::153`
  - `2606:50c0:8002::153`
  - `2606:50c0:8003::153`

**Step 3 — Point `www` at Pages**
- Change `www` from `CNAME -> chan.app` to `CNAME www ->
  fiorix.github.io`, **DNS only**. GitHub auto-creates the apex<->www
  redirect once both resolve to Pages.

**Step 4 — Wait for propagation, then let GitHub issue the cert**
- Verify from your machine:
  ```
  dig +short chan.app A          # expect the four 185.199.108-111.153
  dig +short www.chan.app        # expect fiorix.github.io -> Pages IPs
  ```
- In **Settings -> Pages**, wait for "DNS check successful". GitHub then
  provisions a Let's Encrypt cert (minutes to ~1h). Once issued, the
  **Enforce HTTPS** checkbox un-greys — tick it.
- Do NOT enable the Cloudflare proxy (orange cloud); a proxied record can
  intercept the ACME http-01 challenge and stall cert issuance. DNS-only
  is the chosen mode.

**Step 5 — Verify the site is served by Pages**
- `curl -sI https://chan.app | grep -i server` should show GitHub, not
  the VPS. Browse `https://chan.app/manual/` and click through the index
  links; confirm `https://www.chan.app` redirects to the apex.

**Step 6 (optional hardening) — verify the domain on GitHub**
- GitHub account **Settings -> Pages -> Verified domains** gives a TXT
  record `_github-pages-challenge-fiorix.chan.app`. Adding it (DNS only)
  prevents domain takeover if the repo is ever deleted.

**Step 7 — Decommission VPS web serving (later, once confident)**
- Only after a day or two of clean Pages serving; nothing in this repo
  depends on the VPS afterward.

**Step 8 — Rollback (if anything is wrong before/after cutover)**
- Restore `A chan.app -> 173.208.147.114`, restore the `AAAA`, and set
  `www` CNAME back to `chan.app`. Propagation ~5 min at TTL 300.
