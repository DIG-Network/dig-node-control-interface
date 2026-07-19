# dig-node-control-interface

The canonical **client ⇄ dig-node CONTROL interface** contract. One ecosystem definition of the
management/query surface a client (the CLI `dign`, the browser extension, dig-app, hub) uses to
control and query a running dig-node — config, cache config, peer status, subscriptions, peer
connect/disconnect, and other node-control operations — so the client side and the node side can
never silently drift. Transport-agnostic: rides the `dig-ipc-protocol` local session, or
loopback-mTLS + a signed control token over HTTP/WebSocket for clients that aren't on the local pipe.

- **License:** Apache-2.0 OR MIT
- **Spec:** [`SPEC.md`](./SPEC.md) (normative) — this README is the at-a-glance interface reference.

```toml
[dependencies]
dig-node-control-interface = "0.1"
```

> **Status: skeleton (epic #1110, T1).** This is the provisioning commit — the repository, CI/release
> plumbing, and module layout only. The full method catalog (this section is where every control
> method, its params/result types, and its error code would be listed for LLM/agent lookup, per the
> ecosystem's dig-`<x>`-protocol convention) lands in **T2** (tracked as `dig_ecosystem#1147`). Check
> back there, or re-read this README once T2 merges, for the real interface reference.

## Boundary

- `dig-rpc-protocol` — node ⇄ node peer wire.
- `dig-ipc-protocol` — app ⇄ node local session/signing envelope (the transport this catalog rides
  over, for local clients).
- `dig-node-control-interface` (this crate) — the control METHOD CATALOG carried inside that
  authenticated channel, or over loopback-mTLS for remote-transport clients.

See `SPEC.md` §1 for the full boundary statement, and the superproject `SYSTEM.md` for how this crate
fits the wider cross-repo contract map.

## Consumers (planned)

dig-node (server-side dispatch), the CLI `dign`, dig-chrome-extension, dig-app, hub — all consuming
this one control contract once T2 lands, per epic #1110.
