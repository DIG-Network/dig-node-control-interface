# dig-node-control-interface — normative specification

This document is the authoritative contract for the client ⇄ dig-node **control interface**: how a
client (the CLI `dign`, the browser extension, dig-app, hub) controls and queries a running dig-node.
An independent reimplementation of either side MUST conform to this specification byte-for-byte. Where
this document and the code disagree, the conformance KATs in `src/kats.rs` are the tie-breaker.

Layering: this file is the repo's own contract; the cross-repo interaction map is the superproject
`SYSTEM.md`; the session/transport envelope this catalog rides over is `dig-ipc-protocol`'s `SPEC.md`;
the node↔node peer wire is `dig-rpc-protocol`'s `SPEC.md`. All MUST agree.

## 1. Scope and boundary

`dig-node-control-interface` owns the **client↔node CONTROL method catalog**: the method names,
parameter/result types, and error taxonomy for controlling and querying a running dig-node — status,
configuration, cache, hosted/pinned stores, §21 whole-store sync, the peer network, subscription
lifecycle, the auto-update beacon, live log level, and the control-token pairing handshake.

It does **not** own:
- the node↔node peer wire (dig-rpc-protocol's PublicRead + Peer tiers), or
- the local session/signing handshake and transport a client authenticates over (dig-ipc-protocol).

This crate is transport-agnostic: the same method catalog is carried over the dig-ipc-protocol local
session, or over loopback-mTLS + a signed control token for clients reached via HTTP/WebSocket
(CLAUDE.md §5.3). Consumers select the transport; this crate defines only the payload contract that
rides over it.

## 2. Roles and trust boundary

- **Client:** a user-facing surface (CLI, extension, app, hub) that wants to control or query a
  running dig-node. Authenticates over whichever transport is in play before issuing control calls.
- **Node (server):** the identity-agnostic engine dispatching control calls against its live state
  (config, cache, peer table, subscriptions).

### 2.1 Authorization

- Every `control.*` method is **token-gated**: the caller MUST present the node's local control token
  as the `X-Dig-Control-Token` request header (preferred) or a `params._control_token` field. The
  token is a 64-hex value the node mints at first run into its machine-wide state dir with a
  restrictive ACL; possession of the on-disk token is authorization. A call without a valid token MUST
  be rejected with `UNAUTHORIZED` (-32030). Token comparison MUST be constant-time.
- The two **pairing-bootstrap** methods `pairing.request` / `pairing.poll` are **OPEN** (no token), so
  a token-less client (e.g. an MV3 extension that cannot read a local file) can obtain a scoped token
  after local operator approval.
- The three **pairing-administration** methods (`control.pairing.list` / `.approve` / `.revoke`)
  require the **MASTER** control token specifically, never a scoped paired token — a paired controller
  can drive ordinary mutations but MUST NOT mint more tokens or revoke itself.

## 3. Envelope

Requests and responses are JSON-RPC 2.0.

- **Request:** `{"jsonrpc":"2.0","id":<id>,"method":<name>,"params":<object>}`. `id` is a number,
  string, or null and MUST be echoed on the response. `params` is always an object (`{}` for a
  no-parameter method).
- **Success response:** `{"jsonrpc":"2.0","id":<id>,"result":<value>}` (no `error` key).
- **Error response:** `{"jsonrpc":"2.0","id":<id>,"error":{"code":<int>,"message":<str>,"data":{"code":<SYMBOL>,"origin":<str>}}}`
  (no `result` key). A client MUST branch on `error.data.code` (the stable UPPER_SNAKE symbol), never
  on the human `message`.

## 4. Method catalog

Method names are stable wire contract. `Auth` = requires the control token; `Master` = requires the
master token specifically; `Routing` = how the node resolves it (`owned` by the service shell,
`delegated` to the engine, `open` bootstrap).

| Method | Auth | Routing | Params | Result |
|---|---|---|---|---|
| `control.status` | yes | owned | — | `StatusResult` |
| `control.config.get` | yes | owned | — | `ConfigResult` |
| `control.config.setUpstream` | yes | owned | `{upstream:string}` | `{upstream, requires_restart}` |
| `control.log.setLevel` | yes | owned | `{filter:string}` | `{filter}` |
| `control.cache.get` | yes | owned | — | `CacheView` |
| `control.cache.setCap` | yes | owned | `{cap_bytes:u64}` | `{cap_bytes}` (floored 64 MiB) |
| `control.cache.clear` | yes | owned | — | `{cleared:true}` |
| `control.hostedStores.list` | yes | owned | — | `{stores:[HostedStore]}` |
| `control.hostedStores.pin` | yes | owned | `{store:"storeId[:root]"}` | `{store_id, root, pinned, fetch}` |
| `control.hostedStores.unpin` | yes | owned | `{store}` | `{store_id, unpinned, evicted_capsules}` |
| `control.hostedStores.status` | yes | owned | `{store}` | `{store_id, pinned, capsule_count, total_bytes, capsules}` |
| `control.sync.status` | yes | owned | — | `{available, method, pinned_total, pinned_synced, whole_store_trigger_supported}` |
| `control.sync.trigger` | yes | owned | `{store:"storeId:root"}` | `{store_id, root, status, size_bytes, served_root}` |
| `control.updater.status` | yes | owned | — | (proxied beacon status) |
| `control.updater.setChannel` | yes | owned | `{channel:string}` | (proxied) |
| `control.updater.pause` | yes | owned | `{until?:u64}` | (proxied) |
| `control.updater.resume` | yes | owned | — | (proxied) |
| `control.updater.checkNow` | yes | owned | — | (proxied) |
| `control.pairing.list` | master | owned | — | (pending + issued tokens) |
| `control.pairing.approve` | master | owned | `{pairing_id:string}` | `{approved, client_name, token_id}` |
| `control.pairing.revoke` | master | owned | `{token_id:string}` | `{revoked, token_id}` |
| `control.peerStatus` | yes | delegated | — | (peer-pool snapshot) |
| `control.peers.connect` | yes | delegated | `{peer:string}` | `{connected, peer_id}` |
| `control.peers.disconnect` | yes | delegated | `{peer:string}` | `{disconnected, peer_id}` |
| `control.subscribe` | yes | delegated | `{store_id:string}` | `{subscribed, added, store_id}` |
| `control.unsubscribe` | yes | delegated | `{store_id:string}` | `{subscribed, removed, store_id}` |
| `control.listSubscriptions` | yes | delegated | — | `{subscriptions:[string], count}` |
| `pairing.request` | no | open | `{client_name:string}` | `{pairing_id, pairing_code, expires_ms}` |
| `pairing.poll` | no | open | `{pairing_id:string}` | `{status, token?}` |

### 4.1 Result field definitions

- **`StatusResult`**: `{running:bool, service:string, version:string, commit:string, protocol:string,
  uptime_secs:u64, addr:string, upstream:string, cache:CacheView, hosted_store_count:u64,
  cached_capsule_count:u64, pinned_store_count:u64, sync:{available:bool}}`.
- **`ConfigResult`**: `{addr:string, port:string, upstream:string, upstream_override:string|null,
  cache_dir:string, cache_shared:bool, config_path:string, sync_available:bool}`.
  `upstream_override` MUST be present as `null` when unset (never omitted).
- **`CacheView`**: `{cap_bytes:u64, used_bytes:u64, dir:string, shared:bool}`.
- **`HostedStore`**: `{store_id:string, pinned:bool, capsule_count:u64, total_bytes:u64,
  capsules:[CapsuleEntry]}`.
- **`CapsuleEntry`**: `{capsule:"storeId:root", root:string, size_bytes:u64, last_used_unix_ms:u64}`.
- **`pairing.poll` token**: the `token` field MUST be omitted while `status` is not `approved`, and
  present exactly once after approval.

Proxied results (`control.updater.*`, `control.pairing.list`, `control.peerStatus`) carry the
underlying source's shape verbatim and are modelled as an opaque JSON value; consumers MUST NOT freeze
a struct over them.

## 5. Error taxonomy

The numeric codes are a published wire contract and never change once assigned. `origin` classifies
where the error was minted.

| Code | Symbol | Origin | Meaning |
|---|---|---|---|
| `-32700` | `PARSE_ERROR` | shell | request body was not valid JSON |
| `-32600` | `INVALID_REQUEST` | shell | not a single JSON-RPC object |
| `-32601` | `METHOD_NOT_FOUND` | boundary | control method is not resolved |
| `-32602` | `INVALID_PARAMS` | node | missing/malformed params |
| `-32000` | `DISPATCH_FAILED` | shell | the node failed to dispatch a well-formed call |
| `-32030` | `UNAUTHORIZED` | shell | a `control.*` method called without a valid token |
| `-32031` | `NOT_SUPPORTED` | shell | control op unsupported on this build (e.g. §21 sync with no identity) |
| `-32032` | `CONTROL_ERROR` | shell | a control op failed at runtime |

The `-32020..-32022` band is RESERVED for onion routing (dig-rpc-protocol); the control-plane errors
use `-32030..-32032`.

## 6. Conformance

The golden known-answer tests in `src/kats.rs` are normative: golden request vectors (typed call →
exact envelope), golden response vectors (node JSON decodes into the typed result and re-encodes
byte-identically), the golden error envelope, and an end-to-end route through the node-facing
`ControlHandler` dispatcher proving every method maps to its typed handler. The node side (T7) and
every client side (T8–T10) MUST pin against these vectors. A change that alters a wire shape MUST
fail a KAT.

## 7. Stability

1. `ControlMethod` and `ControlErrorCode` are `#[non_exhaustive]`; adding a method/code is an additive
   MINOR change.
2. Method wire names and error numeric values never change once assigned.
3. Result fields are additive: a new optional field is MINOR; removing/renaming/repurposing a field
   is a BREAKING change.
4. The catalog MUST mirror the live dig-node surface exactly (`dig-node-service/src/control.rs` owned
   methods + the `dig-node-core` delegated peer/subscription methods); a divergence is a drift bug.

## 8. wasm / JS byte-agreement

The catalog types are plain serde structs with no non-wasm dependencies, so a browser/extension client
(T5's `wasm-bindgen` binding) serializes them to identical JSON. The `serde_json::Value`-typed proxied
results and the `#[serde(untagged)]` `RequestId` are the only shapes needing a JS-side check; T5 adds a
Rust↔wasm/JS byte-identical KAT over the vectors in §6.

