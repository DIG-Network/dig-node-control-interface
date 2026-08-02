//! Typed result payloads for the control methods.
//!
//! Each struct is field-for-field identical to what dig-node emits (snake_case wire fields), so a
//! client deserializes the node's real response and re-serializes the same bytes — the property the
//! conformance KATs pin. Genuinely open/proxied shapes (the updater beacon's status, the peer-pool
//! snapshot, the pairing list) stay [`serde_json::Value`] on the call's `Output` rather than being
//! frozen into a struct that would drift from the proxied source.

use serde::{Deserialize, Serialize};

/// The on-disk content-cache view (`control.cache.get`, and embedded in [`StatusResult`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheView {
    /// The configured cache size cap, in bytes.
    pub cap_bytes: u64,
    /// Bytes currently used on disk.
    pub used_bytes: u64,
    /// The cache directory.
    pub dir: String,
    /// Whether the cache directory is the machine-wide shared cache.
    pub shared: bool,
}

/// The §21 sync availability flag embedded in [`StatusResult`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncAvailability {
    /// Whether authenticated §21 whole-store sync is available on this node.
    pub available: bool,
}

/// `control.status` — a rich node status snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusResult {
    /// Always `true` for a responding node.
    pub running: bool,
    /// The service name (`"dig-node"`).
    pub service: String,
    /// The node binary's semantic version.
    pub version: String,
    /// The git commit the binary was built from (or `"unknown"`).
    pub commit: String,
    /// The DIG read protocol version the node speaks.
    pub protocol: String,
    /// Process uptime in seconds.
    pub uptime_secs: u64,
    /// The loopback `host:port` the node is bound to.
    pub addr: String,
    /// The upstream DIG RPC the node proxies/syncs to.
    pub upstream: String,
    /// The on-disk cache view.
    pub cache: CacheView,
    /// Distinct stores held (from the cache).
    pub hosted_store_count: u64,
    /// Cached capsule count.
    pub cached_capsule_count: u64,
    /// Pinned-store count.
    pub pinned_store_count: u64,
    /// §21 sync availability.
    pub sync: SyncAvailability,
}

/// `control.config.get` — the node's effective configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigResult {
    /// The bound `host:port`.
    pub addr: String,
    /// The bound port, as a string.
    pub port: String,
    /// The effective upstream DIG RPC.
    pub upstream: String,
    /// The persisted upstream override, or `null` when unset.
    pub upstream_override: Option<String>,
    /// The cache directory.
    pub cache_dir: String,
    /// Whether the cache is the machine-wide shared cache.
    pub cache_shared: bool,
    /// The node's config.json path.
    pub config_path: String,
    /// Whether authenticated §21 sync is available.
    pub sync_available: bool,
}

/// `control.config.setUpstream` — the persisted override + a restart hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetUpstreamResult {
    /// The normalized upstream that was persisted.
    pub upstream: String,
    /// Always `true` — the change takes effect on next node start.
    pub requires_restart: bool,
}

/// `control.log.setLevel` — the applied filter directive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetLevelResult {
    /// The EnvFilter directive now in effect.
    pub filter: String,
}

/// `control.cache.setCap` — the applied cap (after the 64 MiB floor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCapResult {
    /// The cache cap now in effect, in bytes.
    pub cap_bytes: u64,
}

/// `control.cache.clear` — the clear acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheClearResult {
    /// Always `true`.
    pub cleared: bool,
}

/// One cached capsule of a store, as listed by the hosted-stores methods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsuleEntry {
    /// The capsule reference (`storeId:rootHash`).
    pub capsule: String,
    /// The capsule root hash.
    pub root: String,
    /// The capsule size on disk, in bytes.
    pub size_bytes: u64,
    /// When the capsule was last served, in unix milliseconds.
    pub last_used_unix_ms: u64,
}

/// One hosted/pinned store (`control.hostedStores.list`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStore {
    /// The canonical lowercase 64-hex store id.
    pub store_id: String,
    /// Whether the operator has pinned this store.
    pub pinned: bool,
    /// The number of cached capsules of this store.
    pub capsule_count: u64,
    /// The total cached bytes across this store's capsules.
    pub total_bytes: u64,
    /// The cached capsules of this store.
    pub capsules: Vec<CapsuleEntry>,
}

/// `control.hostedStores.list` — every held/pinned store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStoresListResult {
    /// The stores, one entry per distinct store id.
    pub stores: Vec<HostedStore>,
}

/// `control.hostedStores.pin` — the pin acknowledgement + the pre-fetch outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinResult {
    /// The store id that was pinned.
    pub store_id: String,
    /// The pinned root, or `null` when pinned at store level.
    pub root: Option<String>,
    /// Always `true`.
    pub pinned: bool,
    /// The in-band pre-fetch outcome (`{status, …}`) — its shape varies with the fetch path.
    pub fetch: serde_json::Value,
}

/// `control.hostedStores.unpin` — the unpin acknowledgement + eviction count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnpinResult {
    /// The store id that was unpinned.
    pub store_id: String,
    /// Whether a pin registry entry was actually removed.
    pub unpinned: bool,
    /// How many cached capsules of the store were evicted.
    pub evicted_capsules: u64,
}

/// `control.hostedStores.status` — per-store pinned flag + cached capsules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStoreStatusResult {
    /// The store id queried.
    pub store_id: String,
    /// Whether the store is pinned.
    pub pinned: bool,
    /// The number of cached capsules.
    pub capsule_count: u64,
    /// The total cached bytes.
    pub total_bytes: u64,
    /// The cached capsules.
    pub capsules: Vec<CapsuleEntry>,
}

/// `control.sync.status` — §21 sync availability + pin coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStatusResult {
    /// Whether authenticated §21 whole-store sync is available.
    pub available: bool,
    /// The sync method name.
    pub method: String,
    /// The number of pinned stores.
    pub pinned_total: u64,
    /// How many pinned stores currently have a cached capsule.
    pub pinned_synced: u64,
    /// Whether whole-store (root-less) sync is supported by this build.
    pub whole_store_trigger_supported: bool,
}

/// `control.sync.trigger` — the synced-capsule outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncTriggerResult {
    /// The store id synced.
    pub store_id: String,
    /// The capsule root synced.
    pub root: String,
    /// The outcome status (`"synced"`).
    pub status: String,
    /// The synced capsule size, in bytes.
    pub size_bytes: u64,
    /// The served root the node verified against.
    pub served_root: String,
}

/// `control.pairing.approve` — the mint acknowledgement + the new token's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingApproveResult {
    /// Always `true`.
    pub approved: bool,
    /// The requesting client's declared name.
    pub client_name: String,
    /// The short id of the minted paired token (used to revoke it).
    pub token_id: String,
}

/// `control.pairing.revoke` — the revoke acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingRevokeResult {
    /// Whether a token was actually removed.
    pub revoked: bool,
    /// The token id that was targeted.
    pub token_id: String,
}

/// `control.peers.connect` — the connected peer's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeersConnectResult {
    /// Always `true` on success.
    pub connected: bool,
    /// The connected peer's id.
    pub peer_id: String,
}

/// `control.peers.disconnect` — the dropped peer's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeersDisconnectResult {
    /// Always `true` (idempotent — dropping an absent peer still succeeds).
    pub disconnected: bool,
    /// The peer id that was targeted (trimmed + lower-cased).
    pub peer_id: String,
}

/// `control.subscribe` — the subscription acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeResult {
    /// Always `true`.
    pub subscribed: bool,
    /// Whether the store was newly added (vs already subscribed).
    pub added: bool,
    /// The canonical persisted store id (trimmed + lower-cased).
    pub store_id: String,
}

/// `control.unsubscribe` — the unsubscription acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeResult {
    /// Always `false`.
    pub subscribed: bool,
    /// Whether the store was actually removed.
    pub removed: bool,
    /// The canonical store id.
    pub store_id: String,
}

/// `control.listSubscriptions` — the node's persisted subscription set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListSubscriptionsResult {
    /// The subscribed store ids.
    pub subscriptions: Vec<String>,
    /// The subscription count.
    pub count: u64,
}

/// `control.wallet.balance` — an address's balance for one asset, as the node's chain read saw it.
///
/// A READ-only result: this reports chain state, it never moves funds. It is a strict SUPERSET of
/// dig-app's frozen `BalanceResponse { balance }` — the node emits the richer shape, and because
/// dig-app's struct does not deny unknown fields it reads [`balance`](Self::balance) losslessly and
/// ignores the rest. That superset relationship is the "no dig-app code change" guarantee, pinned by
/// the conformance KAT.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletBalanceResult {
    /// The CONFIRMED, spendable balance in the asset's base unit (mojos for XCH, base units for DIG).
    /// The only field dig-app 3.x reads.
    pub balance: u64,
    /// Incoming funds seen but not yet confirmed (asset base units); not yet spendable.
    pub pending: u64,
    /// Whether the node's chain view is caught up. When `false`, the figures are STALE.
    pub synced: bool,
    /// The peak block height the reported figures reflect, or `null` when the node has no height yet.
    pub peak_height: Option<u32>,
}

/// `pairing.request` — the pairing handshake bootstrap (OPEN, no token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingRequestResult {
    /// The opaque pairing id to poll with.
    pub pairing_id: String,
    /// A short numeric code the operator compares before approving.
    pub pairing_code: String,
    /// When the pending pairing expires, in unix milliseconds.
    pub expires_ms: u64,
}

/// `pairing.poll` — the pairing poll outcome (OPEN, no token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingPollResult {
    /// The pairing status (`"pending"` / `"approved"` / …).
    pub status: String,
    /// The minted scoped token, present exactly once after approval.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn status_result_round_trips_the_node_shape() {
        let v = json!({
            "running": true, "service": "dig-node", "version": "0.30.0", "commit": "abc",
            "protocol": "21", "uptime_secs": 5, "addr": "127.0.0.1:9256", "upstream": "https://rpc.dig.net",
            "cache": {"cap_bytes": 1024, "used_bytes": 10, "dir": "/c", "shared": false},
            "hosted_store_count": 2, "cached_capsule_count": 3, "pinned_store_count": 1,
            "sync": {"available": true}
        });
        let parsed: StatusResult = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), v);
    }

    #[test]
    fn config_result_keeps_upstream_override_null_when_unset() {
        let parsed = ConfigResult {
            addr: "127.0.0.1:9256".into(),
            port: "9256".into(),
            upstream: "https://rpc.dig.net".into(),
            upstream_override: None,
            cache_dir: "/c".into(),
            cache_shared: false,
            config_path: "/c/config.json".into(),
            sync_available: true,
        };
        let v = serde_json::to_value(&parsed).unwrap();
        assert_eq!(v["upstream_override"], json!(null));
        assert!(v.as_object().unwrap().contains_key("upstream_override"));
    }

    #[test]
    fn pairing_poll_omits_token_until_approved() {
        let pending = PairingPollResult {
            status: "pending".into(),
            token: None,
        };
        let v = serde_json::to_value(&pending).unwrap();
        assert_eq!(v, json!({"status": "pending"}));
        let approved = PairingPollResult {
            status: "approved".into(),
            token: Some("deadbeef".into()),
        };
        assert_eq!(
            serde_json::to_value(&approved).unwrap(),
            json!({"status": "approved", "token": "deadbeef"})
        );
    }
}
