# Testing the gateway on Linux and macOS

The gateway's Postgres setup, the sdme container build, the macOS Lima
shim, and the test commands are now covered in one place:
[`dev-setup.md`](dev-setup.md). See its "Prerequisites + Postgres",
"macOS only: Lima shim", and "Running tests" sections.

CI runs the same gate on `gateway/**` changes via
[`gateway-ci.yml`](../../.github/workflows/gateway-ci.yml) with a
`postgres:16` service on `ubuntu-latest` (x86_64), the canonical lane;
local sdme is the fast loop.
