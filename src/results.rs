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

/// How much of its own build a node reveals when it advertises (dig_ecosystem#2215).
///
/// Advertising an exact build is a fingerprinting aid — it tells an observer precisely which peers
/// run a version with a publicly disclosed defect. This is the operator's dial between that cost
/// and the diagnostic value of knowing what the network is running.
///
/// It lives here, beside [`PeerSoftware`], because rendering and parsing are two halves of one
/// format: a node that hand-rolled its own `product/version` string would be re-implementing half
/// the contract, and the two halves would drift. A node picks a mode; this type renders it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoftwareVersionDetail {
    /// Advertise the exact build, e.g. `dig-node/0.99.1`. The default: the diagnostic value is why
    /// the field exists, and an operator who disagrees opts down explicitly.
    #[default]
    Full,
    /// Advertise only the major and minor level, e.g. `dig-node/0.99.0`. Hides the patch level, and
    /// any pre-release or build metadata, while remaining READABLE at the far end.
    Minor,
    /// Advertise nothing. Indistinguishable from a peer built before this field existed, and reads
    /// as [`PeerSoftware::Unknown`].
    Off,
}

impl SoftwareVersionDetail {
    /// Render the advertisement a node with this setting puts on its handshake.
    ///
    /// The result is ALWAYS either the empty string or a value [`PeerSoftware::parse`] reads back
    /// as `Reported`. Coarsening reduces PRECISION; it never produces a value that reads as
    /// Unknown while pretending to be a report. Two consequences follow, and both are tested:
    ///
    /// - [`Minor`](SoftwareVersionDetail::Minor) renders `MAJOR.MINOR.0`, never a bare
    ///   `MAJOR.MINOR` — two-part versions are not valid semver, so that spelling would read as
    ///   Unknown and become a second, confusing spelling of [`Off`](SoftwareVersionDetail::Off).
    /// - `Minor` of a `0.0.x` build renders the EMPTY STRING, because its coarsening is version
    ///   zero and version zero is the "unknown" sentinel. There is no coarser representable value,
    ///   so it advertises nothing rather than advertising the sentinel as if it were a report.
    ///
    /// A coarsened `1.4.0` is indistinguishable from a genuine `1.4.0`. That is the point of
    /// coarsening, not a defect in it.
    pub fn render(self, product: &str, version: &semver::Version) -> String {
        match self {
            Self::Full => format!("{product}/{version}"),
            // A pre-release identifier (`-nightly.20260805`) is more precisely identifying than the
            // patch number beside it, so a "coarse" advertisement that kept it would coarsen
            // nothing for exactly the builds that most want it. `Version::new` drops both it and
            // any build metadata.
            Self::Minor => {
                let coarsened = semver::Version::new(version.major, version.minor, 0);
                // Hiding the patch of a `0.0.x` build leaves version zero, which the wire reserves
                // as the "unknown" sentinel. There is no coarser representable value, so advertise
                // nothing rather than advertise the sentinel dressed up as a report. (This differs
                // from the rejected two-part `MAJOR.MINOR` spelling: there a representable coarse
                // value existed and the wrong one was chosen; here none exists.)
                if is_version_zero(&coarsened) {
                    return String::new();
                }
                format!("{product}/{coarsened}")
            }
            Self::Off => String::new(),
        }
    }
}

/// A peer's advertised SOFTWARE build, as read from the gossip handshake (dig_ecosystem#2215).
///
/// dig-gossip carries the peer's `Handshake.software_version` as an opaque sanitized string and
/// deliberately does not interpret it. This type is where that string becomes meaning, once, at the
/// control boundary — so the interpretation is defined in one place and every client agrees.
///
/// # This is NOT the protocol version
///
/// Wire compatibility is a separate field that dig-gossip gates connections on. Two peers can speak
/// the same protocol while running builds months apart; this type reports the latter. It MUST NOT
/// be used to decide whether to talk to a peer.
///
/// # Why there is no `Ord` and no `Default`
///
/// [`Unknown`](PeerSoftware::Unknown) has no position on a version line: it is the absence of a
/// measurement, not a low value. Deriving `Ord` would place it somewhere — and every peer built
/// before #2215 is Unknown, so "somewhere" would silently become a verdict about most of the live
/// network. Comparison is therefore reachable only by destructuring
/// [`Reported`](PeerSoftware::Reported), which forces the caller to say what Unknown means for
/// their question. There is no `Default` for the same reason: a defaulted Unknown that appears from
/// nowhere is a different fact from one that was measured, and the two must not be confusable.
///
/// # JSON
///
/// ```json
/// {"kind": "unknown"}
/// {"kind": "reported", "product": "dig-node", "version": "0.99.1", "raw": "dig-node/0.99.1"}
/// ```
///
/// Unknown carries no `version` member at all — never version zero, never `""`, never `null` in a
/// field a consumer might read as a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PeerSoftware {
    /// The peer's build is not known: it advertised nothing, advertised VERSION ZERO — the legacy
    /// sentinel, in any decoration (`0.0.0`, `0.0.0-rc.1`, `0.0.0+build`) — or advertised something
    /// this contract cannot parse. See [`PeerSoftware::parse`] for why those three are one case.
    Unknown,
    /// The peer advertised a well-formed `product/semver` build.
    Reported {
        /// The product name, e.g. `dig-node`. Everything before the LAST `/`.
        product: String,
        /// The parsed semantic version, e.g. `0.99.1`. Serializes as its string form.
        version: semver::Version,
        /// Exactly what the peer advertised, after trimming.
        ///
        /// **Currently reconstructible, deliberately kept.** The grammar this parser accepts is
        /// lossless — `semver::Version` re-renders every string it accepts byte-identically — so
        /// today `raw` always equals `format!("{product}/{version}")`, and no test can distinguish
        /// this field from that expression. It is retained as the honest source: the moment the
        /// grammar accepts anything non-canonical (a `v` prefix, a two-part version, a vendor
        /// suffix), a diagnostic reader must see what the peer actually sent rather than this
        /// parser's opinion of it, and callers that already read `raw` will not need to change.
        raw: String,
    },
}

/// Is this VERSION ZERO — the legacy "no version" sentinel, whatever it is dressed in?
///
/// Three of dig-gossip's four handshake send sites hardcoded `"0.0.0"` before dig_ecosystem#2215,
/// so version zero is not a hypothetical value: it is what the live fleet is sending right now. It
/// means "this build predates the field", which is [`PeerSoftware::Unknown`]; mapping it to a
/// *version* would make the whole existing network read as ancient.
///
/// The test is over the major/minor/patch TRIPLE, ignoring any pre-release or build metadata. A
/// peer advertising `0.0.0-rc.1` is no more versioned than one advertising `0.0.0`, and matching
/// the bare string would let the decorated forms through as real builds at version zero.
fn is_version_zero(version: &semver::Version) -> bool {
    version.major == 0 && version.minor == 0 && version.patch == 0
}

/// The separator between the product and the version in a `product/semver` advertisement.
const PRODUCT_VERSION_SEPARATOR: char = '/';

impl PeerSoftware {
    /// Interpret a peer's advertised `software_version` string.
    ///
    /// Returns [`Unknown`](PeerSoftware::Unknown) for an empty or blank string, for anything that
    /// is not `product/semver` with both parts non-empty and the version parsing as semver, and for
    /// any advertisement whose version is VERSION ZERO.
    ///
    /// Version zero is the legacy sentinel and is matched as a CLASS, not as a string: the bare
    /// `0.0.0`, a product-qualified `dig-node/0.0.0`, and every decorated form (`0.0.0-rc.1`,
    /// `0.0.0+build`, `0.0.0-0`) all mean "unversioned". A peer advertising `0.0.0-rc.1` is no more
    /// versioned than one advertising `0.0.0`.
    ///
    /// A product name may contain `/`; the split is at the LAST separator.
    pub fn parse(advertised: &str) -> Self {
        let raw = advertised.trim();

        // No separator at all: an empty advertisement, a bare version, a product with no version,
        // or a bare version-zero sentinel (`0.0.0`, `0.0.0-rc.1`) — none of which contain a `/`, so
        // they land here rather than needing a clause of their own. None of them name a build.
        let Some((product, version)) = raw.rsplit_once(PRODUCT_VERSION_SEPARATOR) else {
            return Self::Unknown;
        };
        if product.is_empty() {
            return Self::Unknown;
        }
        let Ok(version) = version.parse::<semver::Version>() else {
            return Self::Unknown;
        };
        // The sentinel is VERSION ZERO, a class — not the three-character string. Comparing the
        // PARSED version is what makes `0.0.0+build`, `0.0.0-rc.1`, and `0.0.0-0` Unknown too; a
        // string comparison would report each of them as a real build at version zero.
        if is_version_zero(&version) {
            return Self::Unknown;
        }

        Self::Reported {
            product: product.to_string(),
            version,
            raw: raw.to_string(),
        }
    }
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
    /// Which tier produced these figures, or `None` from a node too old to disclose it.
    ///
    /// See [`WalletReadSource`]. Absent (`null` / omitted) is a THIRD state, not a default tier:
    /// it means the answering node predates tier disclosure, so the caller knows the tier is
    /// unknown rather than being told a tier that was never reported.
    ///
    /// The [`Option`] carries the backwards compatibility on its own — serde treats a missing
    /// `Option` field as `None` — so no `#[serde(default)]` is needed and none is written; a
    /// REQUIRED field here would reject an older node's payload outright.
    pub source: Option<WalletReadSource>,
    /// Whether THESE figures reflect a caught-up local view. When `false`, they are STALE or came
    /// from the fallback tier.
    ///
    /// This describes the ANSWER, not the node: a [`WalletReadSource::Fallback`] answer is always
    /// `false`, however caught-up the node's own replica happens to be.
    pub synced: bool,
    /// The peak block height the reported figures reflect, or `null` when no height applies —
    /// including every [`WalletReadSource::Fallback`] answer, whose figures came from the oracle's
    /// chain view rather than the node's.
    pub peak_height: Option<u32>,
}

/// Which tier answered a wallet read (dig_ecosystem#2233).
///
/// A node serves a wallet read either from its own chain replica or from a third-party HTTP
/// oracle, and the two are not interchangeable to a caller: the oracle path is a network round
/// trip that **discloses the queried address off-node**, which a user on a metered or private
/// connection has a legitimate interest in knowing about. Reporting the tier is also what makes
/// "the node answered from its own chain state" a falsifiable claim — a sync-progress flag is not,
/// since a flag can flip while the oracle keeps answering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WalletReadSource {
    /// The node's own local chain replica. No third party was consulted.
    Db,
    /// A third-party coinset HTTP oracle. The queried address was disclosed off-node.
    Fallback,
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

    // ---- PeerSoftware (dig_ecosystem#2215) ----

    /// The mapping that matters most. Every peer built before #2215 advertises the LITERAL
    /// `"0.0.0"` — three of dig-gossip's four handshake send sites hardcoded it. A parser that
    /// maps only `""` to Unknown would therefore read the entire live fleet as "software version
    /// 0.0.0", which any later `>=` comparison treats as ancient. `""`, `"0.0.0"`, and anything
    /// unparseable must all be Unknown, and this test is that mapping's guard.
    #[test]
    fn unknown_covers_empty_the_legacy_sentinel_and_garbage() {
        for raw in [
            "",                       // a peer advertising nothing, or `off` coarsening
            "0.0.0",                  // the pre-#2215 legacy sentinel
            "   ",                    // whitespace only
            "dig-node",               // no version part
            "dig-node/",              // empty version part
            "dig-node/not-a-version", // unparseable version
            "/1.2.3",                 // empty product part
            "1.2.3",                  // bare version, no product
            "dig-node/0.0.0",         // the sentinel, however it is dressed up
        ] {
            assert_eq!(
                PeerSoftware::parse(raw),
                PeerSoftware::Unknown,
                "{raw:?} must map to Unknown"
            );
        }
    }

    /// A well-formed `product/semver` advertisement is reported with all three parts, and `raw`
    /// preserves exactly what the peer sent so a diagnostic reader is never shown a value the peer
    /// did not actually advertise.
    #[test]
    fn reported_carries_product_version_and_the_raw_advertisement() {
        let parsed = PeerSoftware::parse("dig-node/0.99.1");
        let PeerSoftware::Reported {
            product,
            version,
            raw,
        } = parsed
        else {
            panic!("a well-formed advertisement must be Reported");
        };
        assert_eq!(product, "dig-node");
        assert_eq!(version, semver::Version::new(0, 99, 1));
        assert_eq!(raw, "dig-node/0.99.1");
    }

    /// A product name may itself contain a `/`; only the LAST separator splits product from
    /// version. Pinning this stops a future reader from switching to a first-separator split,
    /// which would silently reclassify such a peer as Unknown.
    #[test]
    fn product_is_split_at_the_last_separator() {
        let PeerSoftware::Reported {
            product, version, ..
        } = PeerSoftware::parse("acme/dig-node/1.2.3")
        else {
            panic!("expected Reported");
        };
        assert_eq!(product, "acme/dig-node");
        assert_eq!(version, semver::Version::new(1, 2, 3));
    }

    /// Surrounding whitespace is trimmed before parsing, and `raw` records the TRIMMED
    /// advertisement. CON-008 sanitization strips Unicode Cc/Cf from the wire value but not
    /// spaces, so a padded advertisement reaches this parser intact and must not be classified as
    /// unparseable merely for having been padded.
    #[test]
    fn surrounding_whitespace_is_trimmed_before_parsing() {
        let PeerSoftware::Reported {
            product,
            version,
            raw,
        } = PeerSoftware::parse("  dig-node/1.2.3	")
        else {
            panic!("a padded advertisement must still be Reported");
        };
        assert_eq!(product, "dig-node");
        assert_eq!(version, semver::Version::new(1, 2, 3));
        assert_eq!(raw, "dig-node/1.2.3", "raw must record the trimmed value");
    }

    /// A pre-release/build-metadata semver survives intact, because that is what a nightly build
    /// advertises and dropping it would make every nightly indistinguishable from its release.
    #[test]
    fn prerelease_versions_are_preserved() {
        let PeerSoftware::Reported { version, raw, .. } =
            PeerSoftware::parse("dig-node/1.0.0-nightly.20260805")
        else {
            panic!("expected Reported");
        };
        assert_eq!(version.to_string(), "1.0.0-nightly.20260805");
        assert_eq!(raw, "dig-node/1.0.0-nightly.20260805");
    }

    /// Unknown's JSON is a tagged object — never `"0.0.0"`, never `""`, never a null sitting in a
    /// version field where a consumer might read it as a number.
    #[test]
    fn unknown_serializes_as_a_tagged_object_with_no_version_field() {
        let v = serde_json::to_value(PeerSoftware::Unknown).unwrap();
        assert_eq!(v, json!({"kind": "unknown"}));
        assert!(
            v.get("version").is_none(),
            "Unknown must not carry a version field at all"
        );
    }

    /// Both variants round-trip byte-identically, which is what lets a client re-encode a node's
    /// response unchanged.
    #[test]
    fn both_variants_round_trip_byte_identically() {
        for wire in [
            json!({"kind": "unknown"}),
            json!({
                "kind": "reported",
                "product": "dig-node",
                "version": "0.99.1",
                "raw": "dig-node/0.99.1"
            }),
        ] {
            let parsed: PeerSoftware = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
        }
    }

    /// Parsing a wire string and serializing the result produces the documented JSON, so the two
    /// halves of the contract cannot drift from each other.
    #[test]
    fn parse_then_serialize_matches_the_documented_json() {
        assert_eq!(
            serde_json::to_value(PeerSoftware::parse("dig-node/0.99.1")).unwrap(),
            json!({
                "kind": "reported",
                "product": "dig-node",
                "version": "0.99.1",
                "raw": "dig-node/0.99.1"
            })
        );
        assert_eq!(
            serde_json::to_value(PeerSoftware::parse("0.0.0")).unwrap(),
            json!({"kind": "unknown"})
        );
    }

    // ---- Trait-absence probes (dig_ecosystem#2215) ----
    //
    // `PeerSoftware` deriving `Ord` or `Default` would be a silent correctness regression rather
    // than a compile error anywhere, so it is pinned here. The probe exploits inherent-impl
    // precedence: `Probe::<T>::has_it()` resolves to the inherent impl (returning `true`) only when
    // `T` satisfies the bound, and otherwise falls back to the blanket trait impl (`false`).
    //
    // Each probe carries a CONTROL on a type that DOES implement the trait. Without the control, a
    // probe broken so that it always answers `false` would pass while proving nothing.

    struct Probe<T>(core::marker::PhantomData<T>);

    trait ProbeFallback {
        fn is_ord() -> bool {
            false
        }
    }
    impl<T> ProbeFallback for Probe<T> {}

    impl<T: Ord> Probe<T> {
        fn is_ord() -> bool {
            true
        }
    }

    struct PartialOrdProbe<T>(core::marker::PhantomData<T>);
    trait PartialOrdFallback {
        fn is_partial_ord() -> bool {
            false
        }
    }
    impl<T> PartialOrdFallback for PartialOrdProbe<T> {}
    impl<T: PartialOrd> PartialOrdProbe<T> {
        fn is_partial_ord() -> bool {
            true
        }
    }

    /// A version comparison must be unreachable without first destructuring `Reported`, so that a
    /// caller cannot order `Unknown` against a real version — which, since every pre-#2215 peer is
    /// Unknown, would quietly become a verdict about most of the live network.
    #[test]
    fn peer_software_is_not_ordered() {
        assert!(
            Probe::<u32>::is_ord(),
            "control: the probe must detect a type that IS Ord, or it proves nothing"
        );
        assert!(
            !Probe::<PeerSoftware>::is_ord(),
            "PeerSoftware must not implement Ord — comparison belongs after destructuring Reported"
        );
    }

    /// A defaulted `Unknown` appearing from nowhere is a different fact from a measured one, and
    /// `Default` would make the two indistinguishable at the point of construction.
    #[test]
    fn peer_software_has_no_default() {
        struct DefaultProbe<T>(core::marker::PhantomData<T>);
        trait DefaultFallback {
            fn is_default() -> bool {
                false
            }
        }
        impl<T> DefaultFallback for DefaultProbe<T> {}
        impl<T: Default> DefaultProbe<T> {
            fn is_default() -> bool {
                true
            }
        }

        assert!(
            DefaultProbe::<String>::is_default(),
            "control: the probe must detect a type that IS Default, or it proves nothing"
        );
        assert!(
            !DefaultProbe::<PeerSoftware>::is_default(),
            "PeerSoftware must not implement Default"
        );
    }

    // ---- SoftwareVersionDetail (dig_ecosystem#2215) ----

    /// Each mode renders a value the PARSER reads back at the intended level of detail.
    ///
    /// The fixture uses a version with a non-zero minor AND a non-zero patch, because that is the
    /// only shape where `Full` and `Minor` differ — a `1.0.0` fixture would let a renderer that
    /// ignores the mode entirely pass.
    #[test]
    fn each_detail_mode_round_trips_to_the_intended_precision() {
        let v = semver::Version::new(0, 99, 1);

        let full = SoftwareVersionDetail::Full.render("dig-node", &v);
        assert_eq!(full, "dig-node/0.99.1");
        assert_eq!(
            PeerSoftware::parse(&full),
            PeerSoftware::parse("dig-node/0.99.1")
        );

        let minor = SoftwareVersionDetail::Minor.render("dig-node", &v);
        assert_ne!(minor, full, "Minor must actually coarsen");
        let PeerSoftware::Reported { version, .. } = PeerSoftware::parse(&minor) else {
            panic!("a coarsened advertisement must still be READABLE, not Unknown");
        };
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 99);
        assert_eq!(version.patch, 0, "the patch level is what Minor hides");

        let off = SoftwareVersionDetail::Off.render("dig-node", &v);
        assert_eq!(off, "");
        assert_eq!(PeerSoftware::parse(&off), PeerSoftware::Unknown);
    }

    /// `Minor` renders `MAJOR.MINOR.0`, NOT `MAJOR.MINOR`.
    ///
    /// A bare two-part `0.99` is not valid semver, so the parser would classify a peer that
    /// coarsened its build as Unknown — turning "tell them less" into "tell them nothing", which
    /// is what `Off` is for. This test is the guard on that distinction.
    #[test]
    fn minor_mode_stays_valid_semver_rather_than_collapsing_to_unknown() {
        let rendered =
            SoftwareVersionDetail::Minor.render("dig-node", &semver::Version::new(1, 4, 7));
        assert_eq!(rendered, "dig-node/1.4.0");
        assert_ne!(
            PeerSoftware::parse(&rendered),
            PeerSoftware::Unknown,
            "a coarsened build must remain readable; `product/1.4` would not be"
        );
    }

    /// Coarsening strips pre-release and build metadata. A nightly's identifier is more precisely
    /// identifying than the patch number it accompanies, so leaving it in place would make `Minor`
    /// coarsen nothing at all for exactly the builds that most want it.
    #[test]
    fn minor_mode_strips_prerelease_and_build_metadata() {
        let v: semver::Version = "1.0.0-nightly.20260805+sha.abc123".parse().unwrap();
        let rendered = SoftwareVersionDetail::Minor.render("dig-node", &v);
        assert_eq!(rendered, "dig-node/1.0.0");
        assert!(
            !rendered.contains("nightly"),
            "the nightly identifier must not survive coarsening"
        );
        assert!(
            !rendered.contains("abc123"),
            "build metadata must not survive coarsening"
        );
    }

    /// `Off` renders the empty string for ANY version, which is what makes it indistinguishable
    /// from a peer built before the field existed.
    #[test]
    fn off_mode_reveals_nothing_for_any_version() {
        for v in ["0.0.1", "1.2.3", "99.99.99-rc.1"] {
            let rendered = SoftwareVersionDetail::Off.render("dig-node", &v.parse().unwrap());
            assert_eq!(
                rendered, "",
                "Off must reveal nothing, including the product name"
            );
        }
    }

    /// The default is the most informative setting: the diagnostic value is the reason the field
    /// exists, and an operator who disagrees opts down explicitly.
    #[test]
    fn detail_defaults_to_full() {
        assert_eq!(
            SoftwareVersionDetail::default(),
            SoftwareVersionDetail::Full
        );
    }

    /// The wire tokens are the lowercase words an operator writes in a config file, and they are a
    /// published contract once a config carries them.
    #[test]
    fn detail_uses_lowercase_wire_tokens() {
        for (mode, token) in [
            (SoftwareVersionDetail::Full, "\"full\""),
            (SoftwareVersionDetail::Minor, "\"minor\""),
            (SoftwareVersionDetail::Off, "\"off\""),
        ] {
            assert_eq!(serde_json::to_string(&mode).unwrap(), token);
            assert_eq!(
                serde_json::from_str::<SoftwareVersionDetail>(token).unwrap(),
                mode
            );
        }
    }

    // ---- Gate round 1 regressions (dig_ecosystem#2215) ----

    /// **`PartialOrd` is the hazard the `Ord` probe misses.** `Ord: PartialOrd`, so a type can
    /// derive only `PartialOrd` — satisfying an `Ord`-only probe — while `Unknown < Reported(..)`
    /// still compiles and evaluates. That one-word derive would sort `Unknown` below every real
    /// version, which is the verdict-about-the-live-network the contract forbids. SPEC §4.1 names
    /// all three traits; this pins the weakest of them, which subsumes `Ord`.
    #[test]
    fn peer_software_is_not_partially_ordered_either() {
        assert!(
            PartialOrdProbe::<f64>::is_partial_ord(),
            "control: the probe must detect a type that IS PartialOrd but NOT Ord, or it proves              nothing about the gap between the two"
        );
        assert!(
            PartialOrdProbe::<u32>::is_partial_ord(),
            "control: a fully-ordered type must also be detected"
        );
        assert!(
            !PartialOrdProbe::<PeerSoftware>::is_partial_ord(),
            "PeerSoftware must implement neither PartialOrd nor Ord"
        );
    }

    /// **The sentinel is VERSION ZERO, a class — not the three-character string `\"0.0.0\"`.**
    /// The constant's doc, `parse`'s doc, and SPEC §4.1 all state the rule over the class, so a
    /// string comparison lets `0.0.0+build`, `0.0.0-rc.1`, and `0.0.0-0` through as a *reported*
    /// version zero — the exact reading every one of those three prose statements forbids.
    #[test]
    fn version_zero_is_unknown_however_it_is_decorated() {
        for raw in [
            "dig-node/0.0.0",
            "dig-node/0.0.0+build",
            "dig-node/0.0.0-rc.1",
            "x/0.0.0-0",
            "dig-node/0.0.0-alpha+sha.abc123",
        ] {
            assert_eq!(
                PeerSoftware::parse(raw),
                PeerSoftware::Unknown,
                "{raw:?} is version zero and must be Unknown"
            );
        }
    }

    /// A version that is merely CLOSE to zero is still a real build and must be reported — without
    /// this, a parser that mapped everything below `0.1.0` to Unknown would pass the test above.
    #[test]
    fn a_nonzero_version_near_zero_is_still_reported() {
        for raw in ["dig-node/0.0.1", "dig-node/0.1.0", "dig-node/0.0.1-rc.1"] {
            assert_ne!(
                PeerSoftware::parse(raw),
                PeerSoftware::Unknown,
                "{raw:?} is a real build, not the sentinel"
            );
        }
    }

    /// **`render`'s stated invariant, tested over the class it is stated over.**
    ///
    /// The doc promises: every rendering is either the empty string or a value `parse` reads back
    /// as `Reported`. A `1.4.7` fixture cannot see the case that breaks it — a `0.0.x` build, whose
    /// `MAJOR.MINOR.0` coarsening IS version zero and therefore reads as Unknown. That is the same
    /// Minor-collapses-into-Off defect as the two-part spelling, arriving through the other door.
    #[test]
    fn every_rendering_is_empty_or_readable() {
        let versions = [
            "0.0.1",
            "0.0.7",
            "0.0.99", // the class the 1.4.7 fixture cannot see
            "0.1.0",
            "0.99.1",
            "1.0.0",
            "1.4.7",
            "10.20.30",
            "1.0.0-nightly.20260805+sha.abc123",
            "0.0.1-rc.1",
        ];
        for mode in [
            SoftwareVersionDetail::Full,
            SoftwareVersionDetail::Minor,
            SoftwareVersionDetail::Off,
        ] {
            for v in versions {
                let rendered = mode.render("dig-node", &v.parse().unwrap());
                if rendered.is_empty() {
                    continue;
                }
                assert_ne!(
                    PeerSoftware::parse(&rendered),
                    PeerSoftware::Unknown,
                    "{mode:?} rendered {rendered:?} for {v}, which reads back as Unknown — a                      non-empty rendering must always be readable"
                );
            }
        }
    }

    /// `Minor` on a `0.0.x` build advertises NOTHING, deliberately.
    ///
    /// Hiding the patch of a `0.0.x` version leaves only version zero, which the wire reserves as
    /// the "unknown" sentinel. There is no coarser representable value, so the honest rendering is
    /// the empty string rather than the sentinel dressed up as a report. This differs from the
    /// `0.99` case: there a representable coarse value existed and the wrong spelling was chosen;
    /// here none exists.
    #[test]
    fn minor_of_a_zero_zero_build_advertises_nothing_rather_than_the_sentinel() {
        let rendered = SoftwareVersionDetail::Minor.render("dig-node", &"0.0.7".parse().unwrap());
        assert_eq!(rendered, "");
        assert_ne!(
            rendered, "dig-node/0.0.0",
            "the sentinel must never be ADVERTISED; it is only ever received from a legacy peer"
        );
    }

    /// **Tripwire, not a guard.** `raw` is reconstructible from `product` + `version` for every
    /// string the current grammar accepts, because `semver::Version` re-renders losslessly. This
    /// asserts that equivalence deliberately.
    ///
    /// **When this test FAILS, `raw` has become load-bearing** — the grammar has started accepting
    /// something non-canonical (a `v` prefix, a two-part version, a vendor suffix) and `raw` is now
    /// the only record of what the peer actually sent. Do not "fix" it by deleting the field;
    /// replace this test with real assertions on the divergent inputs.
    #[test]
    fn raw_is_still_reconstructible_from_the_parsed_parts() {
        for advertised in [
            "dig-node/0.0.1",
            "dig-node/0.99.1",
            "dig-node/1.0.0-nightly.20260805",
            "dig-node/1.0.0+sha.abc123",
            "dig-node/1.0.0-rc.1+build.7",
            "acme/dig-node/1.2.3",
        ] {
            let PeerSoftware::Reported {
                product,
                version,
                raw,
            } = PeerSoftware::parse(advertised)
            else {
                panic!("{advertised:?} must be Reported");
            };
            assert_eq!(
                raw,
                format!("{product}/{version}"),
                "raw diverged from the parsed parts for {advertised:?} — `raw` is now                  load-bearing; see this test's doc comment before changing anything"
            );
        }
    }
}
