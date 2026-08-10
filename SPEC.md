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

- Every `control.*` method is **token-gated** EXCEPT the open surface, which is whatever the §4
  method table marks `no` in its Token column — that table is authoritative and this sentence must
  never restate it as a count. Today it is the pairing bootstrap (`pairing.request`,
  `pairing.poll`), the wallet chain reads (`control.wallet.balance` / `.coins` / `.coinById` /
  `.peak` / `.syncStatus`), which need only public chain data, and `control.peerCounts`, which
  discloses two integers about this node's own connectivity. For every other method the caller MUST
  present the node's local control token
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
| `control.peerStatus` | yes | delegated | — | (peer-pool snapshot; each peer entry carries `software`) |
| `control.peerCounts` | no | delegated | — | `{dig_peer_count:u32\|null, chia_peer_count:u32\|null}` |
| `control.peers.connect` | yes | delegated | `{peer:string}` | `{connected, peer_id}` |
| `control.peers.disconnect` | yes | delegated | `{peer:string}` | `{disconnected, peer_id}` |
| `control.subscribe` | yes | delegated | `{store_id:string}` | `{subscribed, added, store_id}` |
| `control.unsubscribe` | yes | delegated | `{store_id:string}` | `{subscribed, removed, store_id}` |
| `control.listSubscriptions` | yes | delegated | — | `{subscriptions:[string], count}` |
| `control.wallet.balance` | no | delegated | `{address:string, asset:"xch"\|"dig"}` | `{balance, pending, source, synced, peak_height}` |
| `control.wallet.coins` | no | delegated | `{address:string, asset:"xch"\|"dig"}` | `WalletCoinsResult` |
| `control.wallet.coinById` | no | delegated | `{coin_id:string}` | `WalletCoinByIdResult` |
| `control.wallet.arrivals` | yes | delegated | `{after_seq:u64=0, limit?:u32}` | `WalletArrivalsResult` |
| `control.wallet.peak` | no | delegated | — | `{peak_height:u32\|null, synced:bool}` |
| `control.wallet.syncStatus` | no | delegated | — | `{phase:"not_started"\|"syncing"\|"synced", peak_height:u32\|null, chia_peer_count:u32\|null}` |
| `control.wallet.broadcast` | yes | delegated | `{signed_bundle_hex:string}` | `WalletBroadcastResult` |
| `pairing.request` | no | open | `{client_name:string}` | `{pairing_id, pairing_code, expires_ms}` |
| `pairing.poll` | no | open | `{pairing_id:string}` | `{status, token?}` |

The wallet CHAIN READS (`control.wallet.balance` / `.coins` / `.coinById` / `.peak` /
`.syncStatus`)
are served WITHOUT a control token, because each needs only public chain data — an address or a coin
id, never a seed, a key, or a signature. `control.peerCounts` is open for a second reason: it
discloses two integers about this node's own connectivity and no address, endpoint or secret. The
`Token` column above is authoritative; the open set is deliberately named here rather than counted,
so that adding a method cannot leave a stale number behind. Two wallet methods are token-gated.
`control.wallet.broadcast` puts bytes on the network. `control.wallet.arrivals` takes only a cursor,
so its answer names this node's OWN watched puzzle hashes and the receive history behind them: the
chain facts are public, the association between this node and those addresses is not. Membership of
the open set turns on WHO NAMES THE ADDRESS, not on whether the data is on chain. That difference is normative for clients, because the two refusals demand
opposite remedies — see §4.2.

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
- **`WalletCoinsResult`**: `{coins:[WalletCoinRecord], source:"db"|"fallback"|null, synced:bool,
  peak_height:u32|null}`. `source`/`synced`/`peak_height` carry exactly the meanings defined for
  `WalletBalanceResult` below. `coins` MUST list the address's spendable coins for the requested
  asset (XCH coins sit AT the puzzle hash; CAT coins are HINTED to it).

  `coins:[]` MUST mean the node consulted a chain and the address holds nothing. A node that could
  NOT consult a chain MUST return the matching §5 wallet error instead — never an empty list. This
  is normative and not a quality-of-implementation note: an empty list on an unreachable chain tells
  a holder of funds that they hold nothing, and a spend built on that answer refuses with a
  shortfall that is not true.
- **`WalletCoinRecord`**: `{coin_id:string, asset:"xch"|"dig"|null, amount:u64,
  parent_coin_info:string, puzzle_hash:string, created_height:u32|null, spent_height:u32|null}`. All
  hashes are lowercase 64-hex, unprefixed. `created_height:null` means the coin is known only from
  the mempool; `spent_height:null` means unspent. The first three fields are a strict SUPERSET of
  dig-app's frozen `CoinRecord`.

  `asset:null` MUST mean THE READ DID NOT CLASSIFY THE COIN. It MUST NOT be read as "no asset" and
  MUST NOT be defaulted to XCH. A singleton, a CAT and a plain XCH coin are indistinguishable from a
  coin id alone — telling them apart requires inspecting the puzzle, which a coin-record read does
  not do. `control.wallet.coins` MUST report the concrete asset it was SCOPED to and MUST NOT emit
  `asset:null`: dig-app's frozen `CoinRecord` requires a non-null asset there, so `null` is a hard
  deserialization failure, not a degraded read. `control.wallet.coinById` MUST report `null`.
- **`WalletCoinByIdResult`**: `{coin:WalletCoinRecord|null, source:"db"|"fallback"|null, synced:bool,
  peak_height:u32|null}`. ONE coin, named by its own id, SPENT OR UNSPENT.

  `coin:null` MUST mean the node consulted a chain and it holds no coin with that id. A node that
  could NOT consult a chain MUST return the matching §5 wallet error instead — never `coin:null`.
  These MUST NOT be collapsed: the first says stop waiting, the second says retry the read. A caller
  that conflates them reports a mint whose coin does not exist as pending forever, with the funds
  already spent.

  The `coin` key MUST be present on every response; `null` is a verdict and MUST NOT be conveyed by
  omitting the field.

  `coin_id` MUST be lowercase 64-hex; a `0x` prefix MUST be accepted on input and MUST NOT be
  emitted. Any other value MUST be refused as `-32602 INVALID_PARAMS` BEFORE any chain is consulted,
  so that an unanswerable question never wears the shape of an answer. There is no `asset`
  parameter: a coin id is not asset-scoped.

  `source` names which tier answered, and every freshness field describes THAT tier, exactly as for
  `WalletBalanceResult` below: a `"fallback"` answer MUST report `synced:false` and
  `peak_height:null`; a `"db"` answer means the node's own replica answered and MUST report
  `synced:true` and that replica's peak. A caller needing a height to bound a confirmation against
  reads `control.wallet.peak`.

  **A node MUST NOT answer `coin:null` from a view that could not have held the coin.** A replica
  that is not caught up, or a local index that is address-scoped rather than a full chain view, has
  not established absence — only its own inability to see. Such a node MUST return `-32040
  WALLET_NO_CHAIN_SOURCE` or `-32042 WALLET_READ_FAILED`. This is normative and load-bearing: the two
  coins this method exists to observe are a created coin sitting at no wallet address and a funding
  coin already spent, both of which an address-scoped view is GUARANTEED to miss, so a `coin:null`
  from one would report a mint that really happened as never having happened, with the funds gone.
  `-32041 WALLET_NOT_SYNCED` is not used here — it names a wallet-scoped branch this method does not
  have — but a node that is not synced still MUST NOT manufacture a negative answer; it errors.

  This method is how a pushed spend becomes OBSERVABLE. `control.wallet.broadcast`'s `accepted:true`
  reports mempool admission only; only a buried confirmation of the created coin is evidence.
  `control.wallet.coins` cannot supply it — it answers by ADDRESS and lists UNSPENT coins only, so it
  sees neither a created coin sitting at no wallet address nor a funding coin the spend consumed.
- **`WalletPeakResult`**: `{peak_height:u32|null, synced:bool}`. The node's chain peak, independent
  of any address. `peak_height:null` means the node tracks NO height — it MUST NOT be read as height
  zero, which every block is trivially above. A caller bounding a claimed confirmation MUST treat
  `null` as unknown and fail closed.

  `synced` here reports ONLY that the replica's initial catch-up completed; it MUST NOT be read as
  "the wallet is caught up AND connected", which is `WalletSyncStatusResult.phase == "synced"` and
  is strictly stronger. Neither is a freshness guarantee.
- **`PeerCountsResult`**: `{dig_peer_count:u32|null, chia_peer_count:u32|null}`. How many peers
  this node holds on EACH network. The two networks are unrelated and their counts move
  independently.

  `dig_peer_count` MUST be the count of peers on the DIG content/gossip network (port 9445) — the
  node's `connected_peers`, the same figure `control.peerStatus` reports. `chia_peer_count` MUST be
  the count of CHIA full-node peers the wallet's chain sync holds, and MUST be the SAME observation
  `WalletSyncStatusResult.chia_peer_count` reports: a conforming node MUST serve both from ONE
  source, and the two MUST agree within a single node's view.

  Neither field may be named `peers`, `connected_peers` or `peer_count`. A name that does not state
  its network forces a consumer to know which number it is holding, and a consumer that guesses
  wrong fails SILENTLY — a plausible integer in a right-looking place.

  For each count, `0` MUST mean the node observed that network and found nothing connected, and
  `null` MUST mean the node cannot observe the count. A network that is not running is UNKNOWN and
  MUST NOT be reported as `0`, which would assert that nothing is connected to a network never asked.

  `control.peerStatus`'s `relay.peer_count` counts peers connected to THE RELAY, not to this node.
  It is frequently the only non-zero number on a node connected to nothing, and it is NEVER the
  answer to "how many peers does this node have" — `dig_peer_count` is.
- **`WalletSyncStatusResult`**: `{phase:"not_started"|"syncing"|"synced", peak_height:u32|null,
  chia_peer_count:u32|null}`. Whether the node's WALLET CHAIN replica is being kept current, how far
  it has got, and how many CHIA full-node peers its sync is using. This is NOT `control.sync.status`,
  which reports §21 DIG store sync and is unrelated.

  `phase` MUST be one of the three tokens above, spelled exactly. `"synced"` MUST require BOTH that
  the initial catch-up completed AND that at least one Chia peer connection is live at the time of
  the read; a wallet that caught up earlier and has since lost every peer MUST report `"syncing"`.
  `"synced"` is therefore STRICTLY STRONGER than `WalletPeakResult.synced`, which reflects only the
  completed-catch-up flag: `"synced"` implies that flag, the flag does not imply `"synced"`. A
  boolean MUST NOT be substituted for the three phases — "never started" and "synced at height 0"
  are different facts and MUST NOT render the same.

  `peak_height` MUST be the node's OWN replica's height, or `null` when it has none. It MUST NOT
  fall back to the coinset oracle, which `control.wallet.peak` deliberately does: that method answers
  what height the chain is at, this field answers how far this replica has got, and an oracle's
  number here would report a caller's own progress using a height the replica never reached.
  `null` MUST NOT be read as height zero.

  `chia_peer_count` counts CHIA FULL-NODE peers the wallet's chain sync is connected to. `0` is an
  OBSERVED zero and is not a fourth phase: a running sync connected to nothing reports `"syncing"`
  with `0`, which a consumer SHOULD render as "syncing — no peers". `null` means the node cannot
  observe the count and licenses no claim about connectivity. This count is NOT the DIG
  gossip/content peer count from `control.peerStatus` (`connected_peers` / `relay_peer_count`); the
  two are unrelated numbers, and a surface labelling either one bare "peers" beside a wallet sync
  status asserts something false. A caller wanting both networks' counts reads
  `control.peerCounts`, whose `chia_peer_count` is this same observation under the same key and MUST
  agree with it. The field is duplicated across the two methods rather than moved, because
  `chia_peer_count:0` beside `"syncing"` is the honest "syncing — no peers" state and a phase
  separated from its count reads as a contradiction; a DIG content-network count, by contrast, is not
  a wallet fact and MUST NOT be added here for symmetry.

  `"synced"` MUST NOT be read as a guarantee that the replica's data is FRESH. A live connection to
  a stalled or lagging peer satisfies the predicate while the replica goes stale; the phase reports
  that catch-up finished and a peer is attached, i.e. that nothing KNOWN is preventing the replica
  from being kept current.

  **Field combinations.** `{phase:"synced", peak_height:null}` MUST NOT be emitted: a node records
  its peak before it marks the initial catch-up complete, so a completed catch-up always has a
  height behind it, and the pair describes a state no conforming node can be in.

  `{phase:"not_started", peak_height:<n>}` is NOT a contradiction and MUST be permitted. The height
  is persisted in the wallet database while the phase describes whether a sync is running IN THIS
  PROCESS, so a node that synced earlier and has just RESTARTED reports this pair truthfully: here
  is the height I reached, and no sync is running right now. A node MUST NOT suppress the height or
  fabricate a phase to avoid emitting it. `{phase:"not_started", peak_height:null}` is equally
  legitimate and means a wallet that has never synced.

  The height reported is the height of the LAST EXISTING block the peer view reported
  (`NewPeakWallet.height` / `RespondPuzzleState.height` from a real full node). This surface performs
  no confirmation-depth arithmetic; a consumer computing depth MUST floor its own input rather than
  assume a convention, because `peak_height` means the NEXT height on a simulator and the last
  existing block on a full node.
- **`WalletBroadcastResult`**: `{accepted:bool, transaction_id:string|null, rejection:string|null}`.
  The node pushes an ALREADY-SIGNED bundle; it MUST NOT sign, and MUST NOT accept any parameter it
  could sign with (§4.3).

  A mempool that examined the bundle and refused it is a SUCCESSFUL call reporting
  `accepted:false` with a `rejection` reason. Failing to REACH a mempool MUST be a §5 error instead.
  These MUST NOT be collapsed: the first says build a different bundle, the second says retry this
  one. `accepted:true` reports mempool admission ONLY — it is NOT evidence that anything reached a
  block, and a caller MUST NOT record an outcome from it. Only a buried confirmation of the created
  coin is evidence.
- **`WalletBalanceResult`**: `{balance:u64, pending:u64, source:"db"|"fallback"|null, synced:bool,
  peak_height:u32|null}`. A
  READ-only chain read over the loopback control plane — it reports state, never moves funds. `balance`
  is the CONFIRMED spendable amount in the asset's base unit (mojos for XCH, base units for DIG);
  `pending` is incoming-unconfirmed.

  `source` names the TIER that produced the figures, and every freshness field describes THAT tier:
  `"db"` is the node's own chain replica (`synced:true`, `peak_height` = the replica's peak);
  `"fallback"` is a third-party coinset HTTP oracle, which MUST report `synced:false` and
  `peak_height:null` however caught-up the node's own replica is, because the replica neither
  produced that figure nor bounds its freshness. A `"fallback"` answer also means the queried address
  WAS DISCLOSED off-node. `source` is ABSENT/`null` only from a node predating tier disclosure — a
  third state meaning "tier unknown", never a defaulted tier; consumers MUST NOT treat it as either.
  `synced:false` means the figures are STALE or fallback-served; `peak_height` is the block height
  the figures reflect (present as `null`, never omitted, when no height applies).
  The `asset` request field is the lowercase wire token `"xch"`/`"dig"`. This result is a strict
  SUPERSET of dig-app's `BalanceResponse {balance}`: a consumer reading only `{balance}` deserializes
  it losslessly (unknown fields ignored), which is the no-consumer-change guarantee pinned by a KAT.

Proxied results (`control.updater.*`, `control.pairing.list`, `control.peerStatus`) carry the
underlying source's shape verbatim and are modelled as an opaque JSON value; consumers MUST NOT freeze
a struct over them.

- **`PeerSoftware`** — a peer's advertised SOFTWARE build, the one member of the otherwise-proxied
  `control.peerStatus` snapshot whose shape this contract owns. Every entry of the snapshot's
  `connected` array MUST carry a `software` member; a peer entry that omits it is a serialization
  defect, NOT a peer of unknown build. Two forms, tagged by `kind`:

  ```json
  {"kind": "unknown"}
  {"kind": "reported", "product": "dig-node", "version": "0.99.1", "raw": "dig-node/0.99.1"}
  ```

  `unknown` MUST carry no `version` member — never `"0.0.0"`, never `""`, never `null`.

  The node derives it from the peer's gossip `Handshake.software_version` string. The mapping is
  normative:

  | Advertised string | Result |
  |---|---|
  | `product/semver`, both parts non-empty, version parsing as semver | `reported` |
  | `""` (the peer advertised nothing, or coarsened its build off) | `unknown` |
  | any advertisement whose version is VERSION ZERO — the LEGACY SENTINEL | `unknown` |
  | anything else unparseable | `unknown` |

  The product/version split is at the LAST `/`, so a product name may itself contain one.
  Surrounding whitespace is trimmed before parsing.

  **Version zero is a CLASS, not a string.** The rule MUST be applied to the parsed
  major/minor/patch triple, ignoring pre-release and build metadata: the bare `0.0.0`, a
  product-qualified `dig-node/0.0.0`, and every decorated form (`0.0.0-rc.1`, `0.0.0+build`,
  `0.0.0-0`) are all `unknown`. A string comparison would let the decorated forms through as real
  builds at version zero.

  **Why version zero is `unknown` and not a version.** Every dig-node built before this contract
  advertises the literal `"0.0.0"`: three of dig-gossip's four handshake send sites hardcoded it. A
  reader that treated it as a version would classify the entire live network as running software
  0.0.0, and any `>=` comparison would call all of it ancient.

  **Version zero MUST NEVER BE ADVERTISED.** It is a value received from a legacy peer, never one a
  conforming node sends — see `SoftwareVersionDetail` below for the one place that constraint
  binds.

  **`PeerSoftware` MUST NOT implement `Ord`, `PartialOrd`, or `Default`.** `unknown` has no position
  on a version line, and most peers are `unknown` today; a comparison is reachable only after
  destructuring `reported`, which forces a caller to decide what `unknown` means for its question.

  **Privacy.** Reporting a peer's exact build is a fingerprinting aid — it identifies which peers run
  a version with a publicly disclosed defect. Accepted for the diagnostic value on a pre-release
  network. A node that declines to advertise sends an empty string, which reads as `unknown` here and
  is indistinguishable from a build predating the field.

- **`SoftwareVersionDetail`** — how much of its own build a node reveals when it advertises. Wire
  tokens `"full"` (default) | `"minor"` | `"off"`, rendering:

  | Mode | Advertised for version `0.99.1` | Read back as |
  |---|---|---|
  | `full` | `dig-node/0.99.1` | `reported`, exact |
  | `minor` | `dig-node/0.99.0` | `reported`, patch level hidden |
  | `off` | `""` | `unknown` |
  | `minor` of a `0.0.x` build | `""` | `unknown` |

  `minor` MUST render `MAJOR.MINOR.0`, never a bare `MAJOR.MINOR`: a two-part version is not valid
  semver, so the coarse setting would be read as `unknown` and become a confusing second spelling of
  `off`. For the same reason, `minor` of a `0.0.x` build MUST render the EMPTY STRING: its
  coarsening is version zero, which is the `unknown` sentinel, and there is no coarser representable
  value — so it advertises nothing rather than advertising the sentinel as if it were a report.

  The binding invariant: **every rendering is either the empty string or a value that reads back as
  `reported`.** Coarsening reduces precision; it never yields a value that reads as `unknown` while
  looking like a report. `minor` MUST also strip pre-release and build metadata — a nightly identifier is more
  precisely identifying than the patch number beside it, so retaining it would coarsen nothing for
  exactly the builds that most want it. A coarsened `1.4.0` is indistinguishable from a genuine
  `1.4.0`; that is the purpose of coarsening, not a defect in it.

  Rendering is specified here, beside the parsing, because they are two halves of one format. A node
  MUST NOT hand-roll its own `product/version` string.

- **`StatusResult.version`** already reports THIS node's own build; there is no separate method for
  it, and `control.peerStatus` covers both the point lookup ("what is that peer running") and the
  census (a group-by over the returned array).

### 4.2 `UNAUTHORIZED` on an open read means an OLD NODE, not a permission problem

A client MUST branch on which method it called:

- On an OPEN read — every method the §4 table marks `no`, today `control.wallet.balance` / `.coins` /
  `.coinById` / `.peak` / `.syncStatus` and `control.peerCounts` — `-32030 UNAUTHORIZED` can only
  come from a node build that predates the method and gates the whole `control.*` namespace. The
  truth is "this node cannot do that yet" and the remedy is an UPGRADE.
- On `control.wallet.broadcast`, `-32030 UNAUTHORIZED` means exactly what it says, and the remedy is
  the CONTROL TOKEN.

A client that maps both to the same outcome sends a person to fix the wrong thing. `-32601
METHOD_NOT_FOUND` always means the method is absent, on either.

### 4.3 The custody boundary (§908)

The node holds no user key and produces no signature. `control.wallet.broadcast` carries signed bytes
and nothing else: there is no key, seed, phrase, or unsigned-spend-plus-key parameter in this catalog,
and none may be added. The node's role on the money path is to read chain state and to push what
somebody else signed.

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
| `-32040` | `WALLET_NO_CHAIN_SOURCE` | node | a wallet chain read had no live chain source to answer |
| `-32041` | `WALLET_NOT_SYNCED` | node | a wallet chain read of the wallet's own address is still syncing, with no fallback |
| `-32042` | `WALLET_READ_FAILED` | node | a wallet chain read failed at the DB / chain-source layer |
| `-32043` | `WALLET_RATE_LIMITED` | node | a wallet chain read was refused: the open fallback rate bound is spent |

The `-3204x` band is the wallet's. All four wallet codes mean the answer is UNKNOWN. A client MUST
NOT degrade any of them into an empty or zero result, and MUST NOT report a mint, a spend or a
balance as having failed on their strength alone — they say the node could not look, not that the
chain said no.

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
