//! The canonical control-method catalog.
//!
//! [`ControlMethod`] enumerates every method a client can send to a running dig-node's CONTROL
//! plane, its stable wire name, whether it requires the local control token, whether it is a
//! pairing-administration method (which requires the MASTER token specifically), and how the node
//! routes it (owned by the service shell, delegated to the embedded node engine, or an open
//! pairing-bootstrap method reachable without a token).
//!
//! This is the SINGLE source of truth for "what can be controlled". The node dispatchers, the
//! client SDKs (CLI `dign`, the extension, dig-app, hub), the OpenRPC/discovery surface, and the
//! conformance KATs all read this one table, so the method set can never drift between them.
//!
//! Mirrors the live dig-node surface: the shell-owned methods in
//! `dig-node-service/src/control.rs` (`CONTROL_METHODS`) plus the peer/subscription methods
//! delegated to `dig-node-core` (`control.peerStatus` / `control.peers.*` / `control.subscribe`
//! / `control.unsubscribe` / `control.listSubscriptions`), and the two OPEN pairing-bootstrap
//! methods (`pairing.request` / `pairing.poll`) a token-less MV3 extension uses to obtain a
//! scoped token after local operator approval.

/// How the node resolves a control method — the routing source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Routing {
    /// Answered by the dig-node service shell itself (config/status/cache/pins/sync/updater/pairing-admin).
    Owned,
    /// Delegated to the embedded dig-node engine's own control surface (peers + subscriptions).
    Delegated,
    /// An OPEN bootstrap method reachable WITHOUT the control token (pairing handshake).
    OpenBootstrap,
}

/// The functional area a control method belongs to — for grouping in UIs and docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    /// Node status snapshot.
    Status,
    /// Node configuration (upstream override).
    Config,
    /// Live log-level control.
    Log,
    /// On-disk content cache.
    Cache,
    /// Hosted/pinned stores.
    HostedStores,
    /// §21 authenticated whole-store sync.
    Sync,
    /// The DIG auto-update beacon proxy.
    Updater,
    /// Control-token pairing lifecycle.
    Pairing,
    /// The L7 peer network: the live pool snapshot, the per-network peer counts, and dial/drop.
    Peers,
    /// The node's subscribed-store set.
    Subscriptions,
    /// Wallet chain transport: the read-only chain views (balance, coins, one coin by id, peak,
    /// sync status) plus the push of an already-signed spend bundle.
    Wallet,
}

/// A dig-node CONTROL method.
///
/// `#[non_exhaustive]` so adding a method in a minor release is additive; downstream matches must
/// carry a `_ => …` arm. Convert to/from the wire name with [`ControlMethod::name`] /
/// [`ControlMethod::from_name`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlMethod {
    // ---- Status / config / log (shell-owned) ----
    /// `control.status` — a rich node status snapshot.
    Status,
    /// `control.config.get` — the node's effective configuration.
    ConfigGet,
    /// `control.config.setUpstream` — persist an upstream-RPC override (effective on restart).
    ConfigSetUpstream,
    /// `control.log.setLevel` — live-swap the running node's tracing level filter.
    LogSetLevel,

    // ---- Cache (shell-owned) ----
    /// `control.cache.get` — the on-disk cache view (cap/used/dir/shared).
    CacheGet,
    /// `control.cache.setCap` — set the cache size cap (floored at 64 MiB).
    CacheSetCap,
    /// `control.cache.clear` — delete all locally cached content.
    CacheClear,

    // ---- Hosted stores (shell-owned) ----
    /// `control.hostedStores.list` — every held/pinned store with its cached capsules.
    HostedStoresList,
    /// `control.hostedStores.pin` — pin a store (and pre-fetch when a root is given).
    HostedStoresPin,
    /// `control.hostedStores.unpin` — unpin a store and evict its cached capsules.
    HostedStoresUnpin,
    /// `control.hostedStores.status` — per-store pinned flag + cached capsules.
    HostedStoresStatus,

    // ---- §21 sync (shell-owned) ----
    /// `control.sync.status` — whether authenticated whole-store sync is available + pin coverage.
    SyncStatus,
    /// `control.sync.trigger` — trigger a §21 sync for one capsule (storeId + root).
    SyncTrigger,

    // ---- Updater beacon proxy (shell-owned) ----
    /// `control.updater.status` — the DIG auto-update beacon's current status.
    UpdaterStatus,
    /// `control.updater.setChannel` — set the beacon's update channel.
    UpdaterSetChannel,
    /// `control.updater.pause` — suspend auto-updates (optionally until a unix time).
    UpdaterPause,
    /// `control.updater.resume` — resume auto-updates.
    UpdaterResume,
    /// `control.updater.checkNow` — force an immediate update check.
    UpdaterCheckNow,

    // ---- Pairing administration (shell-owned, MASTER-token only) ----
    /// `control.pairing.list` — list pending pairing requests + issued paired tokens.
    PairingList,
    /// `control.pairing.approve` — approve a pending pairing, minting a scoped token.
    PairingApprove,
    /// `control.pairing.revoke` — revoke an issued paired token.
    PairingRevoke,

    // ---- Peers (delegated to the engine) ----
    /// `control.peerStatus` — live peer-pool + relay-reservation snapshot.
    PeerStatus,
    /// `control.peerCounts` — how many peers this node holds on EACH network (DIG and Chia).
    PeerCounts,
    /// `control.peers.connect` — dial a peer by address / resolve a connected peer_id.
    PeersConnect,
    /// `control.peers.disconnect` — drop a pooled peer by peer_id.
    PeersDisconnect,

    // ---- Subscriptions (delegated to the engine) ----
    /// `control.subscribe` — subscribe the node to a store (watch + gap-fill).
    Subscribe,
    /// `control.unsubscribe` — stop watching a store.
    Unsubscribe,
    /// `control.listSubscriptions` — the node's persisted subscription set.
    ListSubscriptions,

    // ---- Wallet chain transport (delegated to the engine) ----
    /// `control.wallet.balance` — read an address's confirmed spendable balance for an asset.
    WalletBalance,
    /// `control.wallet.coins` — read an address's spendable coin records for an asset.
    WalletCoins,
    /// `control.wallet.coinById` — read ONE coin record by coin id, spent or unspent.
    WalletCoinById,
    /// `control.wallet.coinSpend` — read the SPEND that spent a coin (puzzle reveal + solution).
    WalletCoinSpend,
    /// `control.wallet.coinsByParent` — read the direct children a coin's spend created (one hop).
    WalletCoinsByParent,
    /// `control.wallet.arrivals` — read confirmed INCOMING funds since a cursor position.
    WalletArrivals,
    /// `control.wallet.peak` — read the node's current chain peak height.
    WalletPeak,
    /// `control.wallet.syncStatus` — read whether the wallet's chain replica is being kept current.
    WalletSyncStatus,
    /// `control.wallet.broadcast` — push an ALREADY-SIGNED spend bundle to the network.
    WalletBroadcast,
    /// `control.wallet.watch` — enrol PUBLIC keys for the node's chain replica to follow.
    WalletWatch,
    /// `control.wallet.unwatch` — deregister enrolled public keys, so the following stops.
    WalletUnwatch,
    /// `control.wallet.watched` — list the public keys currently enrolled.
    WalletWatched,

    // ---- Pairing bootstrap (OPEN — no token) ----
    /// `pairing.request` — request a control-token pairing (returns a code to compare).
    PairingRequest,
    /// `pairing.poll` — poll a pairing; once the operator approves, returns the scoped token once.
    PairingPoll,
}

impl ControlMethod {
    /// The stable JSON-RPC wire name. Never derived from anything else — the published contract.
    pub const fn name(self) -> &'static str {
        match self {
            ControlMethod::Status => "control.status",
            ControlMethod::ConfigGet => "control.config.get",
            ControlMethod::ConfigSetUpstream => "control.config.setUpstream",
            ControlMethod::LogSetLevel => "control.log.setLevel",
            ControlMethod::CacheGet => "control.cache.get",
            ControlMethod::CacheSetCap => "control.cache.setCap",
            ControlMethod::CacheClear => "control.cache.clear",
            ControlMethod::HostedStoresList => "control.hostedStores.list",
            ControlMethod::HostedStoresPin => "control.hostedStores.pin",
            ControlMethod::HostedStoresUnpin => "control.hostedStores.unpin",
            ControlMethod::HostedStoresStatus => "control.hostedStores.status",
            ControlMethod::SyncStatus => "control.sync.status",
            ControlMethod::SyncTrigger => "control.sync.trigger",
            ControlMethod::UpdaterStatus => "control.updater.status",
            ControlMethod::UpdaterSetChannel => "control.updater.setChannel",
            ControlMethod::UpdaterPause => "control.updater.pause",
            ControlMethod::UpdaterResume => "control.updater.resume",
            ControlMethod::UpdaterCheckNow => "control.updater.checkNow",
            ControlMethod::PairingList => "control.pairing.list",
            ControlMethod::PairingApprove => "control.pairing.approve",
            ControlMethod::PairingRevoke => "control.pairing.revoke",
            ControlMethod::PeerStatus => "control.peerStatus",
            ControlMethod::PeerCounts => "control.peerCounts",
            ControlMethod::PeersConnect => "control.peers.connect",
            ControlMethod::PeersDisconnect => "control.peers.disconnect",
            ControlMethod::Subscribe => "control.subscribe",
            ControlMethod::Unsubscribe => "control.unsubscribe",
            ControlMethod::ListSubscriptions => "control.listSubscriptions",
            ControlMethod::WalletBalance => "control.wallet.balance",
            ControlMethod::WalletCoins => "control.wallet.coins",
            ControlMethod::WalletCoinById => "control.wallet.coinById",
            ControlMethod::WalletCoinSpend => "control.wallet.coinSpend",
            ControlMethod::WalletCoinsByParent => "control.wallet.coinsByParent",
            ControlMethod::WalletArrivals => "control.wallet.arrivals",
            ControlMethod::WalletPeak => "control.wallet.peak",
            ControlMethod::WalletSyncStatus => "control.wallet.syncStatus",
            ControlMethod::WalletBroadcast => "control.wallet.broadcast",
            ControlMethod::WalletWatch => "control.wallet.watch",
            ControlMethod::WalletUnwatch => "control.wallet.unwatch",
            ControlMethod::WalletWatched => "control.wallet.watched",
            ControlMethod::PairingRequest => "pairing.request",
            ControlMethod::PairingPoll => "pairing.poll",
        }
    }

    /// Resolve a wire name back to its [`ControlMethod`], or `None` for an unknown name.
    pub fn from_name(name: &str) -> Option<ControlMethod> {
        ControlMethod::ALL
            .iter()
            .copied()
            .find(|m| m.name() == name)
    }

    /// Does calling this method require the local control token?
    ///
    /// Three groups are reachable WITHOUT one, and they are open for two different reasons:
    ///
    /// - the pairing bootstrap (`pairing.request` / `pairing.poll`), so a token-less client can
    ///   obtain a token at all;
    /// - the PEER COUNTS (`control.peerCounts`), which disclose two integers about this node's own
    ///   connectivity and no address, endpoint or secret;
    /// - the wallet CALLER-ADDRESSED CHAIN READS (`control.wallet.balance` / `.coins` /
    ///   `.coinById` / `.coinSpend` / `.coinsByParent`) and the node's own chain POSITION
    ///   (`.peak` / `.syncStatus`), because each needs only PUBLIC chain data the CALLER already
    ///   named — an address, or a coin id; never a seed, a key, or a signature — and dig-node has
    ///   served `control.wallet.balance` open since #1851. A person whose node runs as a service
    ///   with an unreadable token file can still see their own money.
    ///
    /// Two wallet methods are deliberately NOT in that second group.
    /// `control.wallet.broadcast` puts bytes on the network, so the token is what stands between a
    /// local process and a broadcast. `control.wallet.arrivals` names the wallet's OWN watched
    /// puzzle hashes back to a caller that supplied nothing — see
    /// [`ControlMethod::is_open_read`]. On both, `UNAUTHORIZED` genuinely means *unauthorized*.
    pub const fn requires_auth(self) -> bool {
        !self.is_open_read()
            && !matches!(
                self,
                ControlMethod::PairingRequest | ControlMethod::PairingPoll
            )
    }

    /// Is this an OPEN READ — served without a control token?
    ///
    /// Two kinds of method qualify, and they are open for different reasons:
    ///
    /// - the wallet CHAIN READS (`control.wallet.balance` / `.coins` / `.coinById` / `.coinSpend` /
    ///   `.coinsByParent` / `.peak` / `.syncStatus`), which need only PUBLIC chain data — an
    ///   address, or a coin id; never a seed, a key, or a signature. On the first five the CALLER
    ///   supplies the address or coin id, so the node relays a public fact and discloses no
    ///   association with itself; the last two name the node's own chain position and no address
    ///   at all;
    /// - `control.peerCounts`, which is NOT a chain read: it discloses two integers about this
    ///   node's own connectivity, and no address, endpoint, peer identity or secret. The identity
    ///   and topology half of the same subject stays gated behind `control.peerStatus`.
    ///
    /// Naming both reasons matters more than it looks. The test for membership is *does this
    /// disclose only data that is already public, or a bare count of this node's own state?* — NOT
    /// *is it a chain read?* A future method judged against the narrower phrasing, and found to
    /// contradict a member that was already there, invites widening the predicate by analogy rather
    /// than against the rule.
    ///
    /// `control.wallet.arrivals` is the worked example, and it was briefly a member. It passes the
    /// narrower phrasing — every field it returns is a public chain fact — and fails the rule: the
    /// caller supplies NOTHING, so the node volunteers its OWN watched puzzle hashes together with
    /// the full receive history behind them. The individual facts are public; the ASSOCIATION
    /// between this node and those addresses is not, and that association is the whole answer. A
    /// token-less caller could then feed those addresses back into the caller-addressed reads.
    /// Membership turns on *who names the address*, never on whether the bytes are on chain.
    ///
    /// Stated on the contract rather than discovered by calling, because the two refusals a client
    /// can get here demand OPPOSITE remedies. On an open read, `UNAUTHORIZED` can only come from a
    /// node build that predates the method and gates it generically, so the remedy is an upgrade.
    /// On a gated method — the push — `UNAUTHORIZED` means exactly what it says, and the remedy is
    /// the token. A client that maps the two the same way sends somebody to fix the wrong thing.
    pub const fn is_open_read(self) -> bool {
        matches!(
            self,
            ControlMethod::WalletBalance
                | ControlMethod::WalletCoins
                | ControlMethod::WalletCoinById
                | ControlMethod::WalletCoinSpend
                | ControlMethod::WalletCoinsByParent
                | ControlMethod::WalletPeak
                | ControlMethod::WalletSyncStatus
                | ControlMethod::PeerCounts
        )
    }

    /// Is this a PAIRING-ADMINISTRATION method that requires the MASTER control token specifically?
    ///
    /// A paired (scoped) token can drive ordinary `control.*` mutations but MUST NOT mint more
    /// tokens or revoke itself — so listing/approving/revoking pairings requires the master token
    /// (a local file read), never a paired token.
    pub const fn is_pairing_admin(self) -> bool {
        matches!(
            self,
            ControlMethod::PairingList
                | ControlMethod::PairingApprove
                | ControlMethod::PairingRevoke
        )
    }

    /// How the node routes this method (shell-owned, engine-delegated, or open bootstrap).
    pub const fn routing(self) -> Routing {
        match self {
            ControlMethod::PeerStatus
            | ControlMethod::PeerCounts
            | ControlMethod::PeersConnect
            | ControlMethod::PeersDisconnect
            | ControlMethod::Subscribe
            | ControlMethod::Unsubscribe
            | ControlMethod::ListSubscriptions
            | ControlMethod::WalletBalance
            | ControlMethod::WalletCoins
            | ControlMethod::WalletCoinById
            | ControlMethod::WalletCoinSpend
            | ControlMethod::WalletCoinsByParent
            | ControlMethod::WalletArrivals
            | ControlMethod::WalletPeak
            | ControlMethod::WalletSyncStatus
            | ControlMethod::WalletBroadcast
            | ControlMethod::WalletWatch
            | ControlMethod::WalletUnwatch
            | ControlMethod::WalletWatched => Routing::Delegated,
            ControlMethod::PairingRequest | ControlMethod::PairingPoll => Routing::OpenBootstrap,
            _ => Routing::Owned,
        }
    }

    /// The functional area this method belongs to.
    pub const fn category(self) -> Category {
        match self {
            ControlMethod::Status => Category::Status,
            ControlMethod::ConfigGet | ControlMethod::ConfigSetUpstream => Category::Config,
            ControlMethod::LogSetLevel => Category::Log,
            ControlMethod::CacheGet | ControlMethod::CacheSetCap | ControlMethod::CacheClear => {
                Category::Cache
            }
            ControlMethod::HostedStoresList
            | ControlMethod::HostedStoresPin
            | ControlMethod::HostedStoresUnpin
            | ControlMethod::HostedStoresStatus => Category::HostedStores,
            ControlMethod::SyncStatus | ControlMethod::SyncTrigger => Category::Sync,
            ControlMethod::UpdaterStatus
            | ControlMethod::UpdaterSetChannel
            | ControlMethod::UpdaterPause
            | ControlMethod::UpdaterResume
            | ControlMethod::UpdaterCheckNow => Category::Updater,
            ControlMethod::PairingList
            | ControlMethod::PairingApprove
            | ControlMethod::PairingRevoke
            | ControlMethod::PairingRequest
            | ControlMethod::PairingPoll => Category::Pairing,
            ControlMethod::PeerStatus
            | ControlMethod::PeerCounts
            | ControlMethod::PeersConnect
            | ControlMethod::PeersDisconnect => Category::Peers,
            ControlMethod::Subscribe
            | ControlMethod::Unsubscribe
            | ControlMethod::ListSubscriptions => Category::Subscriptions,
            ControlMethod::WalletBalance
            | ControlMethod::WalletCoins
            | ControlMethod::WalletCoinById
            | ControlMethod::WalletCoinSpend
            | ControlMethod::WalletCoinsByParent
            | ControlMethod::WalletArrivals
            | ControlMethod::WalletPeak
            | ControlMethod::WalletSyncStatus
            | ControlMethod::WalletBroadcast
            | ControlMethod::WalletWatch
            | ControlMethod::WalletUnwatch
            | ControlMethod::WalletWatched => Category::Wallet,
        }
    }

    /// A one-line human/agent description for the discovery catalogue.
    pub const fn summary(self) -> &'static str {
        match self {
            ControlMethod::Status => "A rich node status snapshot (version, uptime, addr, cache, hosted/pinned counts, sync availability).",
            ControlMethod::ConfigGet => "The node's effective configuration (addr/port, upstream + override, cache dir/shared, config path, sync availability).",
            ControlMethod::ConfigSetUpstream => "Persist an upstream-RPC override; takes effect on next node start (requires_restart).",
            ControlMethod::LogSetLevel => "Live-swap the running node's tracing EnvFilter directive (not persisted).",
            ControlMethod::CacheGet => "The on-disk content-cache view: cap_bytes, used_bytes, dir, shared.",
            ControlMethod::CacheSetCap => "Set the on-disk cache size cap in bytes (floored at 64 MiB).",
            ControlMethod::CacheClear => "Delete all locally cached DIG content.",
            ControlMethod::HostedStoresList => "Every held/pinned store, merged, with each store's cached capsules and a pinned flag.",
            ControlMethod::HostedStoresPin => "Pin a store (storeId[:rootHash]); pre-fetches the capsule when a root is given and §21 sync is available.",
            ControlMethod::HostedStoresUnpin => "Unpin a store and evict its cached capsules.",
            ControlMethod::HostedStoresStatus => "Per-store status: pinned flag, cached capsules, total bytes.",
            ControlMethod::SyncStatus => "Whether authenticated §21 whole-store sync is available, plus pinned-store cache coverage.",
            ControlMethod::SyncTrigger => "Trigger a §21 sync for one capsule (storeId + root).",
            ControlMethod::UpdaterStatus => "The DIG auto-update beacon's current status (proxied from dig-updater).",
            ControlMethod::UpdaterSetChannel => "Set the beacon's update channel (\"nightly\" | \"stable\").",
            ControlMethod::UpdaterPause => "Suspend the beacon's auto-updates (optionally until a unix time).",
            ControlMethod::UpdaterResume => "Resume the beacon's auto-updates.",
            ControlMethod::UpdaterCheckNow => "Force an immediate beacon update check.",
            ControlMethod::PairingList => "List pending pairing requests and issued paired tokens (MASTER token only).",
            ControlMethod::PairingApprove => "Approve a pending pairing, minting a scoped token (MASTER token only).",
            ControlMethod::PairingRevoke => "Revoke an issued paired token by token_id (MASTER token only).",
            ControlMethod::PeerStatus => "Live peer-pool + relay-reservation snapshot, including the per-peer connected array; each entry carries an always-present `software` field (the peer's advertised build). Its `relay.peer_count` counts peers connected to THE RELAY, not to this node, and is never the answer to \"how many peers does this node have\" -- that is control.peerCounts.",
            ControlMethod::PeerCounts => "READ-only: how many peers this node holds on EACH network -- dig_peer_count (DIG content/gossip, port 9445) and chia_peer_count (Chia full nodes serving the wallet chain sync). Two unrelated numbers, each named for its network.",
            ControlMethod::PeersConnect => "Dial a peer by address, or resolve an already-connected peer_id, via the live gossip pool.",
            ControlMethod::PeersDisconnect => "Drop a pooled peer by peer_id, closing its mTLS link (idempotent).",
            ControlMethod::Subscribe => "Subscribe the node to a store it actively watches and gap-fills.",
            ControlMethod::Unsubscribe => "Stop watching a store.",
            ControlMethod::ListSubscriptions => "The node's persisted subscription set + count.",
            ControlMethod::WalletCoins => "READ-only: the spendable coin records for an address + asset, with the tier that answered and the height they reflect.",
            ControlMethod::WalletCoinById => "READ-only: ONE coin record by coin id, spent or unspent, with no address and no asset scope; `coin: null` means the chain holds no such coin.",
            ControlMethod::WalletCoinSpend => "READ-only: the SPEND that spent a coin -- its puzzle reveal, its solution and the coin itself -- named by the coin's own id. `spend: null` means the consulted chain shows that coin as unspent or unknown; it NEVER means the chain could not be reached, which is an error.",
            ControlMethod::WalletCoinsByParent => "READ-only: the DIRECT children created by spending one coin, named by that parent's coin id. ONE hop, never a recursive walk: an empty list means the parent created no known children, and a caller wanting a lineage composes hops itself.",
            ControlMethod::WalletArrivals => "READ-only: confirmed INCOMING funds recorded since a cursor position, oldest first -- the answer to `was I just paid?`, which no balance or coin list can give. Each row is a coin the node determined ARRIVED: confirmed on chain, above the wallet's arrival baseline, not previously reported, and not the wallet's own change. Resume from `cursor` (the last row you were handed), never from `latest`.",
            ControlMethod::WalletPeak => "READ-only: the node's current chain peak height, independent of any address.",
            ControlMethod::WalletSyncStatus => "READ-only: whether the wallet's CHAIN replica is being kept current (not_started/syncing/synced/no_wallet_enrolled/wallet_not_unlocked), the replica's own height, and its CHIA full-node peer count -- unrelated to control.sync.status (DIG stores) and to control.peerStatus (DIG peers).",
            ControlMethod::WalletBroadcast => "Push an ALREADY-SIGNED spend bundle to the network; the node never signs. TOKEN-GATED.",
            ControlMethod::WalletBalance => "READ-only: the confirmed spendable balance for an address + asset (plus pending, sync freshness, and the peak height it reflects).",
            ControlMethod::WalletWatch => "Enrol PUBLIC keys (48-byte G1, lowercase 96-hex) for the node's chain replica to follow, so their addresses are synced and readable. IDEMPOTENT: re-enrolling a key already enrolled succeeds and changes nothing. Keys, never puzzle hashes -- the node derives the addresses itself, so one derivation serves every client. TOKEN-GATED.",
            ControlMethod::WalletUnwatch => "Deregister enrolled public keys, so the node stops following their addresses. IDEMPOTENT: a key that was never enrolled is not an error. TOKEN-GATED.",
            ControlMethod::WalletWatched => "READ-only: the public keys currently enrolled, so a client can reconcile what it asked for against what the node holds. TOKEN-GATED although it is a read -- the caller supplies nothing, so the answer is this node's OWN key set.",
            ControlMethod::PairingRequest => "OPEN: request a control-token pairing; returns a pairing_id + pairing_code to compare.",
            ControlMethod::PairingPoll => "OPEN: poll a pairing by id; once the operator approves, returns the scoped token once.",
        }
    }

    /// Every catalogued method, in a stable order — the enumeration a machine reads to discover the
    /// full control surface, and the anchor the conformance KATs pin against.
    pub const ALL: &'static [ControlMethod] = &[
        ControlMethod::Status,
        ControlMethod::ConfigGet,
        ControlMethod::ConfigSetUpstream,
        ControlMethod::LogSetLevel,
        ControlMethod::CacheGet,
        ControlMethod::CacheSetCap,
        ControlMethod::CacheClear,
        ControlMethod::HostedStoresList,
        ControlMethod::HostedStoresPin,
        ControlMethod::HostedStoresUnpin,
        ControlMethod::HostedStoresStatus,
        ControlMethod::SyncStatus,
        ControlMethod::SyncTrigger,
        ControlMethod::UpdaterStatus,
        ControlMethod::UpdaterSetChannel,
        ControlMethod::UpdaterPause,
        ControlMethod::UpdaterResume,
        ControlMethod::UpdaterCheckNow,
        ControlMethod::PairingList,
        ControlMethod::PairingApprove,
        ControlMethod::PairingRevoke,
        ControlMethod::PeerStatus,
        ControlMethod::PeerCounts,
        ControlMethod::PeersConnect,
        ControlMethod::PeersDisconnect,
        ControlMethod::Subscribe,
        ControlMethod::Unsubscribe,
        ControlMethod::ListSubscriptions,
        ControlMethod::WalletBalance,
        ControlMethod::WalletCoins,
        ControlMethod::WalletCoinById,
        ControlMethod::WalletCoinSpend,
        ControlMethod::WalletCoinsByParent,
        ControlMethod::WalletArrivals,
        ControlMethod::WalletPeak,
        ControlMethod::WalletSyncStatus,
        ControlMethod::WalletBroadcast,
        ControlMethod::WalletWatch,
        ControlMethod::WalletUnwatch,
        ControlMethod::WalletWatched,
        ControlMethod::PairingRequest,
        ControlMethod::PairingPoll,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_method_has_a_unique_wire_name() {
        let names: BTreeSet<&str> = ControlMethod::ALL.iter().map(|m| m.name()).collect();
        assert_eq!(
            names.len(),
            ControlMethod::ALL.len(),
            "duplicate or missing wire names in the catalog"
        );
    }

    #[test]
    fn from_name_round_trips_every_method() {
        for &m in ControlMethod::ALL {
            assert_eq!(ControlMethod::from_name(m.name()), Some(m));
        }
        assert_eq!(ControlMethod::from_name("control.nope"), None);
        assert_eq!(ControlMethod::from_name(""), None);
    }

    #[test]
    fn the_token_less_surface_is_exactly_the_bootstrap_plus_the_chain_reads() {
        // Written out rather than derived from `is_open_read`, so this pins the SET and not the
        // implementation's opinion of itself. A method added to the open surface must be added
        // here deliberately -- which is the review step a broadcast must never slip past.
        let expected_open: BTreeSet<&str> = [
            "pairing.request",
            "pairing.poll",
            "control.wallet.balance",
            "control.wallet.coins",
            "control.wallet.coinById",
            "control.wallet.coinSpend",
            "control.wallet.coinsByParent",
            "control.wallet.peak",
            "control.wallet.syncStatus",
            "control.peerCounts",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            expected_open.len(),
            10,
            "the open surface is ten named methods"
        );
        let actual_open: BTreeSet<&str> = ControlMethod::ALL
            .iter()
            .filter(|m| !m.requires_auth())
            .map(|m| m.name())
            .collect();
        assert_eq!(actual_open, expected_open);
    }

    /// **The gated wallet methods are the push, the arrival cursor, and the three enrolment
    /// methods.** The fixture varies one thing -- which wallet method is asked -- against a category
    /// whose other members ARE open, so both nearest wrong implementations fail here: one that opens
    /// the whole category (the state this crate shipped in at `1190a18`) and one that gates it
    /// wholesale.
    ///
    /// Written out in catalog order rather than derived, so a method joining the gated side is a
    /// deliberate edit here -- the review step a broadcast, or an enrolment, must never slip past.
    #[test]
    fn the_gated_wallet_methods_are_the_push_the_cursor_and_enrolment() {
        let gated: Vec<&str> = ControlMethod::ALL
            .iter()
            .filter(|m| m.category() == Category::Wallet && m.requires_auth())
            .map(|m| m.name())
            .collect();
        assert_eq!(
            gated,
            vec![
                "control.wallet.arrivals",
                "control.wallet.broadcast",
                "control.wallet.watch",
                "control.wallet.unwatch",
                "control.wallet.watched",
            ]
        );
        assert!(!ControlMethod::WalletBroadcast.is_open_read());
    }

    /// **The arrival cursor is NOT an open read, and the reason is not "is it a chain read?".**
    ///
    /// The rule is *who names the address*. `control.wallet.arrivals` takes only a cursor, so the
    /// node volunteers its OWN watched puzzle hashes and the receive history behind them -- the
    /// node-to-address association, which is not public, and which a token-less caller could then
    /// replay into the caller-addressed reads.
    ///
    /// The control keeps `control.wallet.coinById` in the same assertion: it is the neighbour the
    /// analogy was drawn from, it is still open, and it stays open because its CALLER supplies the
    /// coin id. Without that control this test would also pass on a wholesale gating of the wallet
    /// category, which is a different (and wrong) implementation.
    #[test]
    fn the_arrival_cursor_is_not_an_open_read() {
        assert!(
            !ControlMethod::WalletArrivals.is_open_read(),
            "control.wallet.arrivals discloses this node's OWN watched puzzle hashes to a caller \
             that supplied nothing, so it MUST NOT be served token-less"
        );
        assert!(ControlMethod::WalletArrivals.requires_auth());
        assert!(
            ControlMethod::WalletCoinById.is_open_read(),
            "the caller-addressed reads stay open -- the fix is the membership rule, not gating \
             the wallet category"
        );
    }

    /// **The control plane names every chain primitive `ChainSource` needs.**
    ///
    /// The list is written out rather than derived, because the property under test is a claim about
    /// ANOTHER crate's trait (`dig-chainsource-interface`'s `ChainSource`) that no compiler here can
    /// check. Five of its seven methods need a control method of their own. The other two need none:
    /// `parent_spend` is a trait DEFAULT composed from `coin_record` + `coin_spend`, and
    /// `resolve_singleton_lineage` is composed CLIENT-side from the primitives below rather than
    /// served as a walk the node performs.
    ///
    /// `block_timestamp` is deliberately ABSENT from the control plane. dig-node's light client
    /// (`chia-peer`'s `ChiaPeerProvider`) does not index block timestamps and answers `Unsupported`,
    /// so a control method for it could only ever be refused — a surface that looks live and does
    /// nothing. A consumer mirrors that refusal honestly; if one ever genuinely needs the value, the
    /// method is an additive minor at that point.
    ///
    /// A missing name here is not a cosmetic gap: a client that cannot answer one of these cannot
    /// implement the trait at all, which is what made a dig-profile mint structurally impossible
    /// through the node before these two were added (dig_ecosystem#2572).
    #[test]
    fn the_catalog_serves_every_chain_source_primitive() {
        for wire in [
            "control.wallet.coinById",      // coin_record
            "control.wallet.coins",         // coin_records_by_puzzle_hash
            "control.wallet.peak",          // peak_height
            "control.wallet.coinsByParent", // coin_records_by_parent
            "control.wallet.coinSpend",     // coin_spend
        ] {
            assert!(
                ControlMethod::from_name(wire).is_some(),
                "{wire} is required to implement ChainSource over the control plane"
            );
        }
    }

    /// **The two chain primitives are `coinById`'s neighbours, not `arrivals`'.**
    ///
    /// Each takes a caller-supplied coin id and returns a deterministic public chain fact,
    /// so the membership rule — *who names the subject* — puts them on the open side. The gated
    /// control in the same assertion is what makes the test load-bearing: without it, a wholesale
    /// opening of the wallet category would pass, and that is a different (and wrong) implementation.
    #[test]
    fn the_chain_primitives_are_caller_named_open_reads() {
        for method in [
            ControlMethod::WalletCoinSpend,
            ControlMethod::WalletCoinsByParent,
        ] {
            assert!(
                method.is_open_read(),
                "{} names its subject in the request and discloses no node-to-address \
                 association, exactly like control.wallet.coinById",
                method.name()
            );
            assert!(!method.requires_auth());
        }
        assert!(
            ControlMethod::WalletArrivals.requires_auth(),
            "the caller-supplies-nothing read stays gated -- the rule is who names the subject, \
             not whether the bytes are on chain"
        );
        assert!(ControlMethod::WalletBroadcast.requires_auth());
    }

    /// **All three enrolment methods are gated — including the one that only reads.**
    ///
    /// `control.wallet.watch` and `.unwatch` aim what the node follows, so they are mutations and the
    /// question barely arises. `control.wallet.watched` is the one a future reader will be tempted to
    /// open, because it returns nothing but public keys and every other wallet READ in this catalog is
    /// open. It stays gated under the SAME rule that gates `control.wallet.arrivals`: the caller
    /// supplies nothing, so the node volunteers its OWN enrolled keys — the node-to-key association,
    /// which is not public, and which a token-less caller could replay straight into the
    /// caller-addressed reads.
    ///
    /// The control keeps `control.wallet.coinById` open in the same assertion. Without it this test
    /// would also pass on a wholesale gating of the wallet category, which is a different (and wrong)
    /// implementation.
    #[test]
    fn the_enrolment_methods_are_gated_including_the_read() {
        for wire in [
            "control.wallet.watch",
            "control.wallet.unwatch",
            "control.wallet.watched",
        ] {
            let method = ControlMethod::from_name(wire)
                .unwrap_or_else(|| panic!("{wire} must be in the catalog"));
            assert!(
                !method.is_open_read(),
                "{wire} either aims this node's subscriptions or names the keys it already \
                 follows, so it MUST NOT be served token-less"
            );
            assert!(method.requires_auth(), "{wire} must require the token");
            assert_eq!(method.category(), Category::Wallet);
            assert_eq!(method.routing(), Routing::Delegated);
        }
        assert!(
            ControlMethod::WalletCoinById.is_open_read(),
            "the caller-addressed reads stay open -- enrolment is gated by the membership rule, \
             not by gating the wallet category"
        );
    }

    #[test]
    fn only_pairing_bootstrap_is_open_bootstrap_routed() {
        for &m in ControlMethod::ALL {
            let open_bootstrap = matches!(
                m,
                ControlMethod::PairingRequest | ControlMethod::PairingPoll
            );
            assert_eq!(
                m.routing() == Routing::OpenBootstrap,
                open_bootstrap,
                "{} routing mismatch",
                m.name()
            );
        }
    }

    #[test]
    fn pairing_admin_methods_are_exactly_three() {
        let admin: Vec<&str> = ControlMethod::ALL
            .iter()
            .filter(|m| m.is_pairing_admin())
            .map(|m| m.name())
            .collect();
        assert_eq!(
            admin,
            vec![
                "control.pairing.list",
                "control.pairing.approve",
                "control.pairing.revoke"
            ]
        );
    }

    #[test]
    fn delegated_set_matches_the_engine_surface() {
        let delegated: BTreeSet<&str> = ControlMethod::ALL
            .iter()
            .filter(|m| m.routing() == Routing::Delegated)
            .map(|m| m.name())
            .collect();
        let expected: BTreeSet<&str> = [
            "control.wallet.coins",
            "control.wallet.coinById",
            "control.wallet.coinSpend",
            "control.wallet.coinsByParent",
            "control.wallet.arrivals",
            "control.wallet.peak",
            "control.wallet.syncStatus",
            "control.wallet.broadcast",
            "control.wallet.watch",
            "control.wallet.unwatch",
            "control.wallet.watched",
            "control.peerStatus",
            "control.peerCounts",
            "control.peers.connect",
            "control.peers.disconnect",
            "control.subscribe",
            "control.unsubscribe",
            "control.listSubscriptions",
            "control.wallet.balance",
        ]
        .into_iter()
        .collect();
        assert_eq!(delegated, expected);
    }

    #[test]
    fn every_method_has_a_nonempty_summary() {
        for &m in ControlMethod::ALL {
            assert!(!m.summary().is_empty(), "{} has no summary", m.name());
        }
    }
}
