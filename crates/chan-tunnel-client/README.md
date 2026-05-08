# chan-tunnel-client

Client library for chan-tunnel. Embedded into `chan serve` (and
into future native clients) so a user-running `chan` can register
its drive at the public tunnel host and receive forwarded HTTP
requests over a single h2/TLS connection.

## What it does

1. Dials the public tunnel host over h2/TLS at `TUNNEL_PATH`.
2. Sends a `Hello` control frame with the user token and
   drive name; reads `HelloAck` on success.
3. Hands the resulting bidirectional stream to yamux.
4. For every incoming yamux substream, runs hyper's HTTP/1
   server with a user-supplied `tower::Service` (typically an
   `axum::Router`).

Everything past the handshake is conventional HTTP between yamux
substreams: the public side opens a substream per request, the
local side serves it.

## Public surface

```
dial(...)                       open the h2 stream + run handshake
ClientConfig                    URL, token, drive name, timeouts
Registration                    drive name accepted by the server
TunnelEvent                     substream open / accept / error
ClientError                     uniffi-friendly error variants

handshake<S>(...)               free function over any tokio duplex
                                (used by wire tests)
serve_substreams<S>(...)        run an axum::Router on yamux subs
run(cfg, router)                end-to-end: dial + handshake + serve
```

## Notes

- The eventual consumer is `chan serve --tunnel-url ...
  --tunnel-token ...`; this crate is the library form.
- `handshake` and `serve_substreams` are exposed as free functions
  so unit tests can exercise the protocol over an in-memory tokio
  duplex without spinning up TLS.
