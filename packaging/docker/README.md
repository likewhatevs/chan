# docker

OCI images for chan and the chan-gateway services. Multi-stage builds: a builder stage carries the Rust toolchain plus Node (the web bundles are baked into the binaries by rust-embed at compile time); a thin runtime stage ships only the binary and a CA bundle.

## Images

| Image                            | Dockerfile / target               | Ports        |
|----------------------------------|-----------------------------------|--------------|
| `chan`                           | `chan.Dockerfile`                 | 8787         |
| `chan-gateway-identity`          | `gateway.Dockerfile` `identity`   | 7000         |
| `chan-gateway-profile`           | `gateway.Dockerfile` `profile`    | 7001         |
| `chan-gateway-devserver-proxy`   | `gateway.Dockerfile` `devserver-proxy` | 7002, 7100 |
| `chan-upload-test`               | `test/upload/Dockerfile`          | (test only)  |

The three gateway services share one builder stage in `gateway.Dockerfile` (a single cargo build of all three crates); `--target` selects the runtime.

## Build

The chan and gateway builds use the **repository root** as their context (they run the project's own `make` targets, which need the full tree). Run from the repo root:

```sh
packaging/docker/build.sh                 # all four images, tag :dev
packaging/docker/build.sh -t v0.50.0      # custom tag
packaging/docker/build.sh --model         # chan image with the embedded search model
packaging/docker/build.sh --save          # also export OCI archives to packaging/docker/_out/
```

`build.sh` autodetects docker (BuildKit), podman, or buildah. The equivalent raw commands:

```sh
DOCKER_BUILDKIT=1 docker build -f packaging/docker/chan.Dockerfile -t chan:dev .
docker build -f packaging/docker/gateway.Dockerfile --target identity        -t chan-gateway-identity:dev .
docker build -f packaging/docker/gateway.Dockerfile --target profile         -t chan-gateway-profile:dev .
docker build -f packaging/docker/gateway.Dockerfile --target devserver-proxy -t chan-gateway-devserver-proxy:dev .
docker build -f packaging/docker/test/upload/Dockerfile -t chan-upload-test:dev packaging/docker/test/upload
```

Build-context excludes live in `<dockerfile>.dockerignore` (BuildKit reads them automatically; `build.sh` passes `--ignorefile` for podman/buildah). The builder rebuilds `node_modules` and the web bundles inside the image, so those are excluded from the context.

### chan image, with or without the embedded model

The embedded search model is optional (~130 MB larger image). Default builds omit it and chan downloads the model on demand at runtime:

```sh
docker build -f packaging/docker/chan.Dockerfile -t chan:dev .                       # no model
docker build -f packaging/docker/chan.Dockerfile --build-arg EMBED_MODEL=1 -t chan:model .   # embedded
```

## Run

### Pull from Docker Hub

Every release publishes the four images to `docker.io/fiorix/`. The immutable version tag (`X.Y.Z`) is published on every release and never moves; `latest` is published only on a GA release (skipped for pre-releases), so it always resolves to the newest stable build.

```sh
docker pull fiorix/chan:0.55.0      # immutable, pinned to one release
docker pull fiorix/chan:latest      # newest GA release
```

The gateway services publish the same way and share the tag policy: `fiorix/chan-gateway-identity`, `fiorix/chan-gateway-profile`, `fiorix/chan-gateway-devserver-proxy`. Run a published image just like the local `chan:dev` examples below -- substitute the `fiorix/chan:<tag>` ref:

```sh
docker run --rm -p 8787:8787 -v "$PWD:/workspace" fiorix/chan:0.55.0
```

### chan: serve one workspace

```sh
docker run --rm -p 8787:8787 -v "$PWD:/workspace" chan:dev
```

Serves the mounted folder on `0.0.0.0:8787`. The bearer-token gate stays on; the token is printed on stderr and persisted under the workspace data dir. The subcommands are `open` (one workspace) and `devserver` (many); there is no `chan serve`.

### chan: devserver dialing a gateway tunnel

```sh
docker run --rm -e CHAN_TUNNEL_TOKEN=chan_pat_... chan:dev \
    devserver --bind 0.0.0.0 \
    --tunnel-url https://devserver.example.com/v1/tunnel
```

### gateway services

The service binaries read configuration from environment variables (they do NOT source the systemd `EnvironmentFile`; that is a packaging concern). Each image sets only `BIND_ADDR=0.0.0.0:<port>` (the in-repo default is `127.0.0.1`, which is unreachable across containers). Everything else is injected at runtime; no secrets are baked in. The full variable contract is in `gateway/crates/*/packaging/*.env` and `gateway/README.md`.

For orchestration (Postgres + the three services wired together) and the local sdme validation, use `packaging/kube/` — see `packaging/kube/README.md`.

## Design notes

- **Base images.** Builder `node:20-bookworm`, runtime `debian:bookworm-slim`. Both are Debian bookworm so the binary's glibc requirement never exceeds the runtime's glibc. Pin by digest for reproducible production builds.
- **No runtime deps beyond glibc + ca-certificates.** The gateway is sqlx + rustls (no libpq, no openssl). chan-tunnel-client uses rustls-native-certs, so the chan image needs `ca-certificates` for the outbound tunnel TLS dial.
- **Non-root.** Runtime images create and run as a non-root user (`chan`, `chan-gateway`), mirroring the systemd units' `User=`.
- **Secrets stay out of the image.** Config and secrets arrive as environment variables / mounted files at runtime.
