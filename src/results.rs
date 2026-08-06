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
/// Unknown carries no `version` member at all — never `"0.0.0"`, never `""`, never `null` in a
/// field a consumer might read as a version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PeerSoftware {
    /// The peer's build is not known: it advertised nothing, advertised the legacy `"0.0.0"`
    /// sentinel, or advertised something this contract cannot parse. See [`PeerSoftware::parse`]
    /// for why those three are one case.
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

/// The legacy sentinel every peer built before dig_ecosystem#2215 advertises.
///
/// Three of dig-gossip's four handshake send sites hardcoded this literal (the outbound dial and
/// both introducer dials), so it is not a hypothetical value — it is what the live fleet is sending
/// right now. It means "this build predates the field", which is [`PeerSoftware::Unknown`], and
/// mapping it to a *version* would make the whole existing network read as ancient.
const LEGACY_UNVERSIONED_SENTINEL: &str = "0.0.0";

/// The separator between the product and the version in a `product/semver` advertisement.
const PRODUCT_VERSION_SEPARATOR: char = '/';

impl PeerSoftware {
    /// Interpret a peer's advertised `software_version` string.
    ///
    /// Returns [`Unknown`](PeerSoftware::Unknown) for an empty or blank string, for the legacy
    /// `"0.0.0"` sentinel, and for anything that is not `product/semver` with both
    /// parts non-empty and the version parsing as semver. A version that is *itself* the legacy
    /// sentinel (`dig-node/0.0.0`) is also Unknown: the sentinel means "unversioned" whether or not
    /// a product name was attached to it.
    ///
    /// A product name may contain `/`; the split is at the LAST separator.
    pub fn parse(advertised: &str) -> Self {
        let raw = advertised.trim();

        // No separator at all: an empty advertisement, a bare version, a product with no version,
        // or the bare legacy `"0.0.0"` sentinel — which contains no `/` and so lands here rather
        // than needing a clause of its own. None of them name a build.
        let Some((product, version)) = raw.rsplit_once(PRODUCT_VERSION_SEPARATOR) else {
            return Self::Unknown;
        };
        if product.is_empty() || version == LEGACY_UNVERSIONED_SENTINEL {
            return Self::Unknown;
        }
        let Ok(version) = version.parse::<semver::Version>() else {
            return Self::Unknown;
        };

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
}
