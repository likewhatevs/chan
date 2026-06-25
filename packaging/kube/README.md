# kube

Kubernetes manifests for the chan-gateway stack, plus the local sdme validation path. Images come from `packaging/docker/` (see `packaging/docker/README.md`).

## Two shapes

- **Cluster manifests** (`config.yaml`, `secret.example.yaml`, `postgres.yaml`, `profile.yaml`, `identity.yaml`, `devserver-proxy.yaml`): separate Deployments + Services. Services reach each other by Service DNS name (`chan-gateway-profile:7001`, etc.). This is what you apply to a real cluster.
- **sdme validation pod** (`sdme/gateway-pod.yaml`): the whole stack as ONE Pod. sdme runs a multi-container Pod as a single systemd-nspawn container sharing a network namespace via localhost, so there is no Service DNS; the manifest wires the inter-service URLs to `127.0.0.1`. This is the shape that comes up under sdme on a single host without a real cluster.

The service env-var contract is `gateway/crates/*/packaging/*.env` and `gateway/README.md`. The inter-service trust model (which token each service shares) is in `.agents/gateway.md` "Service-to-service bearers".

## Inter-service wiring

| Edge                                   | Variable                  | Carried by |
|----------------------------------------|---------------------------|------------|
| identity -> profile (service API)      | `PROFILE_AUTH_TOKEN`      | Secret     |
| devserver-proxy -> identity (validate) | `IDENTITY_INTERNAL_TOKEN` | Secret     |
| identity mint / proxy verify (gate)    | `WORKSPACE_GATE_SECRET`   | Secret     |
| identity + profile -> proxy admin      | `WORKSPACE_ADMIN_TOKEN`   | Secret     |
| profile + identity -> Postgres         | `DATABASE_URL`            | Secret     |
| public domain                          | `CHAN_DOMAIN`, `PUBLIC_SCHEME` | ConfigMap |

`IDENTITY_INTERNAL_TOKEN` and `WORKSPACE_GATE_SECRET` MUST match across the two services that share them, or the tunnel handoff fails. identity refuses to start with no OAuth provider, so the Secret carries placeholder GitHub creds for boot.

## Making the images available to sdme

`sdme kube` resolves a short `image:` name through `default_kube_registry` (default `docker.io`). Locally-built images are not in a registry, so make them reachable first. The registry-backed path:

```sh
# 1. Build + export the images (privileged build host; needs a container engine).
packaging/docker/build.sh -t dev

# 2. Run a registry sdme can reach, push the four images, point sdme at it:
#    sudo sdme config set default_kube_registry <registry-host>:5000
#    then docker push <registry-host>:5000/chan-gateway-identity:dev  (etc.)
```

> NOTE (unverified on this host): the exact bridge from a locally-built OCI image to `sdme kube apply` (local registry vs `sdme fs import --oci-mode app` vs the OCI blob cache) was not run here — this host has no container engine and sdme needs root. Confirm the resolution path on the first privileged run and pin it in this section. The manifests themselves are standard k8s and are registry-agnostic (bare image names + `imagePullPolicy: IfNotPresent`).

## D3: bring the stack up under sdme and prove it healthy

All `sdme` commands need root; `<base>` is a systemd-capable base rootfs (`sudo sdme fs import ubuntu docker.io/ubuntu --install-packages=yes`, or set `default_base_fs`).

```sh
# Secrets as an sdme kube secret (loopback DATABASE_URL for the single pod):
sudo sdme kube secret create gateway-secrets \
    --from-literal=DATABASE_URL=postgres://chan:chan@127.0.0.1:5432/chan_gateway \
    --from-literal=POSTGRES_PASSWORD=chan \
    --from-literal=PROFILE_AUTH_TOKEN=$(openssl rand -hex 32) \
    --from-literal=PROFILE_ADMIN_TOKEN=$(openssl rand -hex 32) \
    --from-literal=IDENTITY_INTERNAL_TOKEN=$(openssl rand -hex 32) \
    --from-literal=WORKSPACE_GATE_SECRET=$(openssl rand -hex 32) \
    --from-literal=WORKSPACE_ADMIN_TOKEN=$(openssl rand -hex 32) \
    --from-literal=GITHUB_CLIENT_ID=dev-placeholder \
    --from-literal=GITHUB_CLIENT_SECRET=dev-placeholder

sudo sdme kube apply -f packaging/kube/sdme/gateway-pod.yaml --base-fs <base>

# Health: /healthz on each service, from inside the pod's shared netns.
sudo sdme exec chan-gateway --oci -- sh -c '
  for p in 7000 7001 7002; do
    printf "port %s: " "$p"; curl -fsS "http://127.0.0.1:$p/healthz" && echo;
  done'

# Service-to-service proof: identity reaching profile is exercised by a sign-in;
# for a non-interactive check, confirm devserver-proxy validates against identity
# by watching logs while hitting the apex admin/healthz surface.
sudo sdme logs chan-gateway --oci

sudo sdme kube delete chan-gateway        # teardown
```

A healthy stack answers `ok` on all three `/healthz` ports, and the profile / identity logs show migrations applied (they migrate on boot once Postgres is up).

## D4: headless-Chrome browser upload

Proves a browser is not subject to chan-desktop's Tauri upload ACL. See `test/upload-pod.yaml`.

```sh
sudo sdme kube apply -f packaging/kube/test/upload-pod.yaml --base-fs <base>
sudo sdme logs chan-upload-test --oci      # upload-tester prints PASS: ...
sudo sdme kube delete chan-upload-test
```

The `upload-tester` container drives a real headless Chromium that POSTs a file to `/api/files/upload` from the chan page's origin, then stat's the landed file on the shared `/workspace` volume.

## Apply to a real cluster

```sh
cp packaging/kube/secret.example.yaml packaging/kube/secret.yaml   # replace every value; do not commit
kubectl apply -f packaging/kube/config.yaml -f packaging/kube/secret.yaml \
    -f packaging/kube/postgres.yaml -f packaging/kube/profile.yaml \
    -f packaging/kube/identity.yaml -f packaging/kube/devserver-proxy.yaml
```

A TLS terminator fronts the public services: route `id.<domain>` to `chan-gateway-identity:7000`, `devserver.<domain>` + `*.devserver.<domain>` to `chan-gateway-devserver-proxy:7002`, and grpc/h2c-pass the tunnel register endpoint to `:7100`. Set `COOKIE_SECURE=true` and `PUBLIC_SCHEME=https` behind TLS.
