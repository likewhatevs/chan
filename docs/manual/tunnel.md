# Tunnel Basics

Local use is the default. Tunnel mode is opt-in.

## Local server

Without tunnel flags, `chan serve` binds to `127.0.0.1` and gates requests
with a per-launch bearer token.

## Tunnel mode

With a tunnel token, the server dials the Chan tunnel service and publishes
the drive under `drive.chan.app`. The local bearer-token gate is disabled in
tunnel mode because the gateway in front of the public route becomes the
trust boundary.

## Public tunnel

Anonymous public access is not the default. Public tunnel behavior must be
selected explicitly with the server flag that makes the tunnel public.
