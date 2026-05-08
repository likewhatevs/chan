# chan-tunnel-proto

Wire types and control frames for chan-tunnel. Pure data: no I/O,
no async. Both `chan-tunnel-client` and `chan-tunnel-server` depend
on this crate.

## What chan-tunnel is

A single HTTP/2 bidirectional stream between `chan serve` (running
on the user's machine) and the public tunnel terminator (e.g.
`tunnel.chan.app`, fronted by nginx). The first message in each
direction is a length-prefixed JSON control frame; after that, both
sides hand the byte stream to yamux and multiplex regular HTTP
substreams over it.

This crate owns the framing, the control frames, and the drive-name
rules that both sides apply identically.

## Public surface

```
control::    Hello, HelloAck, ProtocolVersion
drive_name:: is_valid_drive_name, sanitize_drive_name,
             MAX_DRIVE_NAME_LEN
frame::      encode_frame, decode_frame, FrameError
io::         read_frame, write_frame, IoFrameError
h2_duplex::  H2Duplex (h2 SendStream + RecvStream as one duplex)
```

Constants:

```
TUNNEL_PATH               = "/v1/tunnel"
                            Stable across protocol versions; the
                            version is negotiated inside Hello,
                            not via a path bump.

MAX_CONTROL_FRAME_BYTES   = 64 KiB
                            Upper bound on a single control frame.
                            Guards against a malicious or buggy
                            peer trying to allocate gigabytes
                            before yamux even starts.
```

## Out of scope

- HTTP, TLS, h2 client / server setup.
- yamux multiplexing.
- Token validation, registry of live tunnels.

Those concerns live in `chan-tunnel-client` and
`chan-tunnel-server`.
