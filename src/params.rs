//! Typed request params for the control methods, each bound to its method + result via
//! [`crate::traits::ControlCall`].
//!
//! One params type per method (even where two methods share the same field shape, e.g. the four
//! `{ store }` methods) so the compile-time method↔params↔result binding is exact: a caller passes
//! `PinParams { store }` and the type system yields a [`PinResult`](crate::results::PinResult).
//! Field names are the exact wire names dig-node reads.

use serde::{Deserialize, Serialize};

use crate::method::ControlMethod;
use crate::results;
use crate::traits::ControlCall;

/// Bind a params type to its wire method + typed result.
macro_rules! control_call {
    ($ty:ty => $method:expr, $out:ty) => {
        impl ControlCall for $ty {
            const METHOD: ControlMethod = $method;
            type Output = $out;
        }
    };
}

/// Define a no-param call: an empty params struct (serializes to `{}`) bound to its method + result.
macro_rules! no_params {
    ($(#[$doc:meta])* $name:ident => $method:expr, $out:ty) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name {}
        control_call!($name => $method, $out);
    };
}

no_params!(
    /// `control.status` params (none).
    StatusParams => ControlMethod::Status, results::StatusResult
);
no_params!(
    /// `control.config.get` params (none).
    ConfigGetParams => ControlMethod::ConfigGet, results::ConfigResult
);
no_params!(
    /// `control.cache.get` params (none).
    CacheGetParams => ControlMethod::CacheGet, results::CacheView
);
no_params!(
    /// `control.cache.clear` params (none).
    CacheClearParams => ControlMethod::CacheClear, results::CacheClearResult
);
no_params!(
    /// `control.hostedStores.list` params (none).
    HostedStoresListParams => ControlMethod::HostedStoresList, results::HostedStoresListResult
);
no_params!(
    /// `control.sync.status` params (none).
    SyncStatusParams => ControlMethod::SyncStatus, results::SyncStatusResult
);
no_params!(
    /// `control.updater.status` params (none). Result is the proxied beacon status.
    UpdaterStatusParams => ControlMethod::UpdaterStatus, serde_json::Value
);
no_params!(
    /// `control.updater.resume` params (none).
    UpdaterResumeParams => ControlMethod::UpdaterResume, serde_json::Value
);
no_params!(
    /// `control.updater.checkNow` params (none).
    UpdaterCheckNowParams => ControlMethod::UpdaterCheckNow, serde_json::Value
);
no_params!(
    /// `control.pairing.list` params (none). Result is the pending + issued-token list.
    PairingListParams => ControlMethod::PairingList, serde_json::Value
);
no_params!(
    /// `control.peerStatus` params (none). Result is the peer-pool snapshot.
    PeerStatusParams => ControlMethod::PeerStatus, serde_json::Value
);
no_params!(
    /// `control.listSubscriptions` params (none).
    ListSubscriptionsParams => ControlMethod::ListSubscriptions, results::ListSubscriptionsResult
);

/// `control.config.setUpstream` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetUpstreamParams {
    /// The upstream DIG RPC URL to persist (blank clears the override).
    pub upstream: String,
}
control_call!(SetUpstreamParams => ControlMethod::ConfigSetUpstream, results::SetUpstreamResult);

/// `control.log.setLevel` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetLevelParams {
    /// An `EnvFilter` directive, e.g. `"debug"` or `"info,dig_node_core=debug"`.
    pub filter: String,
}
control_call!(SetLevelParams => ControlMethod::LogSetLevel, results::SetLevelResult);

/// `control.cache.setCap` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCapParams {
    /// The cache size cap in bytes (floored at 64 MiB by the node).
    pub cap_bytes: u64,
}
control_call!(SetCapParams => ControlMethod::CacheSetCap, results::SetCapResult);

/// `control.hostedStores.pin` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinParams {
    /// A store reference: `storeId` or `storeId:rootHash`.
    pub store: String,
}
control_call!(PinParams => ControlMethod::HostedStoresPin, results::PinResult);

/// `control.hostedStores.unpin` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnpinParams {
    /// A store reference: `storeId` or `storeId:rootHash`.
    pub store: String,
}
control_call!(UnpinParams => ControlMethod::HostedStoresUnpin, results::UnpinResult);

/// `control.hostedStores.status` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostedStoreStatusParams {
    /// A store reference: `storeId` or `storeId:rootHash`.
    pub store: String,
}
control_call!(HostedStoreStatusParams => ControlMethod::HostedStoresStatus, results::HostedStoreStatusResult);

/// `control.sync.trigger` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncTriggerParams {
    /// A capsule reference: `storeId:rootHash` (a concrete root is required).
    pub store: String,
}
control_call!(SyncTriggerParams => ControlMethod::SyncTrigger, results::SyncTriggerResult);

/// `control.updater.setChannel` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetChannelParams {
    /// The update channel (`"nightly"` | `"stable"`; the beacon CLI is the sole validator).
    pub channel: String,
}
control_call!(SetChannelParams => ControlMethod::UpdaterSetChannel, serde_json::Value);

/// `control.updater.pause` params.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseParams {
    /// The unix-seconds time to pause until; omit to pause indefinitely.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub until: Option<u64>,
}
control_call!(PauseParams => ControlMethod::UpdaterPause, serde_json::Value);

/// `control.pairing.approve` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproveParams {
    /// The pending pairing's id (from `pairing.request`).
    pub pairing_id: String,
}
control_call!(ApproveParams => ControlMethod::PairingApprove, results::PairingApproveResult);

/// `control.pairing.revoke` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevokeParams {
    /// The short id of the paired token to revoke.
    pub token_id: String,
}
control_call!(RevokeParams => ControlMethod::PairingRevoke, results::PairingRevokeResult);

/// `control.peers.connect` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeersConnectParams {
    /// A peer address to dial, or an already-connected peer_id to resolve.
    pub peer: String,
}
control_call!(PeersConnectParams => ControlMethod::PeersConnect, results::PeersConnectResult);

/// `control.peers.disconnect` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeersDisconnectParams {
    /// The peer_id to drop.
    pub peer: String,
}
control_call!(PeersDisconnectParams => ControlMethod::PeersDisconnect, results::PeersDisconnectResult);

/// `control.subscribe` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscribeParams {
    /// The store id to subscribe to.
    pub store_id: String,
}
control_call!(SubscribeParams => ControlMethod::Subscribe, results::SubscribeResult);

/// `control.unsubscribe` params.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsubscribeParams {
    /// The store id to stop watching.
    pub store_id: String,
}
control_call!(UnsubscribeParams => ControlMethod::Unsubscribe, results::UnsubscribeResult);

/// The asset a wallet balance/coin read is denominated in.
///
/// Serializes to a lowercase, language-neutral wire token (`"xch"` / `"dig"`) — byte-identical to the
/// frozen consumer type in dig-app (`dig-app-core::wallet::state::Asset`), so the contract and the
/// consumer share one wire form. Extended additively as the wallet grows to hold more CAT types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Asset {
    /// Native Chia (XCH), denominated in mojos.
    Xch,
    /// The DIG CAT, denominated in its base units.
    Dig,
}

/// `control.wallet.balance` params: which address + asset to read the balance of.
///
/// A READ over the loopback control plane — never a spend. Field names + the [`Asset`] wire form are
/// byte-identical to dig-app's frozen `BalanceRequest`, so the node reads exactly what dig-app emits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletBalanceParams {
    /// The `xch1…` address to read the balance of.
    pub address: String,
    /// The asset to read the balance for.
    pub asset: Asset,
}
control_call!(WalletBalanceParams => ControlMethod::WalletBalance, results::WalletBalanceResult);

/// `control.wallet.coins` params: which address + asset to read spendable coins for.
///
/// Field-for-field identical to [`WalletBalanceParams`] — a balance is this read reduced to a sum —
/// and byte-identical to dig-app's frozen `CoinsRequest`, so adopting the method is a body swap
/// rather than a re-shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletCoinsParams {
    /// The `xch1…` address to read coins for.
    pub address: String,
    /// The asset to read coins for.
    pub asset: Asset,
}
control_call!(WalletCoinsParams => ControlMethod::WalletCoins, results::WalletCoinsResult);

/// The length of a coin id in lowercase hex characters: a 32-byte hash.
const COIN_ID_HEX_LEN: usize = 64;

/// `control.wallet.coinById` params: WHICH coin, named by its own id.
///
/// # Why there is no `asset` here
///
/// A coin id names one coin on one chain. It is not scoped to an address and not scoped to an
/// asset, and a node reading a coin record learns neither — so an asset parameter here could only
/// be a claim the read never checks. The answer's
/// [`asset`](crate::results::WalletCoinRecord::asset) is `null` for the same reason.
///
/// # Why the method exists at all
///
/// [`WalletCoinsParams`] answers by ADDRESS and lists UNSPENT coins only. A mint's evidence is the
/// opposite shape: the created DID coin (which sits at nobody's wallet address) and the funding
/// coin the mint SPENT (which is, by then, gone from every unspent list). Without a by-id read a
/// pushed mint can never be observed — a permanent "pending" with the money already spent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WalletCoinByIdParams {
    /// The coin id: lowercase 64-hex, unprefixed. A `0x` prefix is TOLERATED on input (block
    /// explorers print one) and normalized away by [`Self::validated`]; it is never emitted.
    pub coin_id: String,
}
control_call!(WalletCoinByIdParams => ControlMethod::WalletCoinById, results::WalletCoinByIdResult);

/// `control.wallet.coinSpend` params: WHICH coin's spend to read, named by that coin's own id.
///
/// # Why the spend and the coin record are separate methods
///
/// [`WalletCoinByIdParams`] answers *what is this coin, and was it spent?* — an id, an amount, a
/// puzzle HASH and two heights. None of that reveals what the spend DID. Reconstructing a lineage
/// (which is how a dig-profile's DID singleton is followed forward) needs the puzzle REVEAL and the
/// solution, and those exist only in the spend. A caller holding a coin record alone can see that a
/// coin is gone and cannot see what it became.
///
/// # The id names the SPENT coin, not the spend
///
/// A spend has no id of its own on chain; it is identified by the coin it consumed. So the parameter
/// is the same 64-hex coin id [`WalletCoinByIdParams`] takes, validated by the same rule, and the
/// two methods are asked with the identical value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WalletCoinSpendParams {
    /// The SPENT coin's id: lowercase 64-hex, unprefixed. A `0x` prefix is TOLERATED on input and
    /// normalized away by [`Self::validated`]; it is never emitted.
    pub coin_id: String,
}
control_call!(WalletCoinSpendParams => ControlMethod::WalletCoinSpend, results::WalletCoinSpendResult);

/// `control.wallet.coinsByParent` params: WHICH coin's direct children to read.
///
/// # One hop, and the field name says so
///
/// The field is `parent_coin_id` rather than `coin_id` because the coin named here is the one being
/// asked ABOUT as a parent — it is never the coin the caller wants back. A walk up or down a lineage
/// is the CALLER's composition of repeated single hops; the node performs exactly one. Naming it
/// `coin_id` would make a recursive reading of the method plausible from the request alone, and a
/// caller expecting a whole lineage from one call would read a one-hop answer as a truncated chain.
///
/// # Bounded, because a parent's child count is not
///
/// This is the only OPEN wallet read whose answer has unbounded cardinality — every other one
/// returns a single record (`coinById`, `peak`, `syncStatus`) or is already paged (`arrivals`). So
/// the read is PAGED, and the page is bounded by [`COINS_BY_PARENT_MAX_LIMIT`].
///
/// **The bound is the ONLY thing bounding this call — there is no rate limiter anywhere behind it.**
/// dig-node's control plane has no request rate limiting of any kind (dig_ecosystem#2577); the
/// bandwidth limiter it does have governs content serving and is not on this path. A future reader
/// weighing whether to relax this cap should assume no limiter exists, because none does.
///
/// That matters more than a local resource bound would, because the node does not necessarily answer
/// from its own replica: on the fallback tier it forwards a caller-supplied identifier to a
/// THIRD-PARTY coinset HTTPS oracle. An unbounded page is therefore unbounded work against somebody
/// else's service, requested by a token-less caller on a loopback endpoint.
///
/// Paging rather than a bare cap, because a bare cap is a dead end: a parent with more children than
/// the cap could never be fully enumerated, and this method exists to WALK a lineage. A walk that
/// cannot see past the cap is a walk that silently stops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WalletCoinsByParentParams {
    /// The PARENT coin's id: lowercase 64-hex, unprefixed. A `0x` prefix is TOLERATED on input and
    /// normalized away by [`Self::validated`]; it is never emitted.
    pub parent_coin_id: String,
    /// Resume STRICTLY AFTER this child, in the read's
    /// [documented order](results::WalletCoinsByParentResult). `None` starts at the first child.
    ///
    /// This is the value the previous page handed back as
    /// [`cursor`](results::WalletCoinsByParentResult::cursor) — never a value the caller invented,
    /// and never a marker for where the chain "got to". `control.wallet.arrivals` records why that
    /// distinction loses rows; this read avoids the trap by having no such marker to reach for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_coin_id: Option<String>,
    /// The page size. `None` asks for [`COINS_BY_PARENT_DEFAULT_LIMIT`].
    ///
    /// A value above [`COINS_BY_PARENT_MAX_LIMIT`], or a zero, is REFUSED as `INVALID_PARAMS` rather
    /// than clamped — see [`Self::validated`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
control_call!(WalletCoinsByParentParams => ControlMethod::WalletCoinsByParent, results::WalletCoinsByParentResult);

/// `control.wallet.arrivals` — confirmed INCOMING funds recorded since a cursor position.
///
/// The answer to "was I just paid?", which neither a balance nor a coin list can give: a balance
/// moves for the user's OWN change too, and an unspent-coin list cannot say which of its coins are
/// new. Each row the node returns is a coin it determined ARRIVED — confirmed on chain, above the
/// wallet's arrival baseline, not previously reported, and not created by spending one of the
/// wallet's own coins. The determination is the NODE's; a client MUST NOT re-derive it, because the
/// signals it takes (spent parents, the catch-up baseline) live only in the node's replica.
///
/// # A cursor, not a stream
///
/// The control envelope is strictly request→response, so this is polled. A client resumes from
/// [`WalletArrivalsResult::cursor`](results::WalletArrivalsResult::cursor) — the last position it
/// was actually handed — and NEVER from `latest`; see that field for why the distinction loses a
/// notification when it is collapsed.
///
/// # `after_seq` is unsigned so a rewind is unexpressible
///
/// Positions are monotonic and start at 1, so there is no meaning for a negative one. `0` is the
/// beginning of the ledger and is what a client sends when it deliberately wants everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WalletArrivalsParams {
    /// Return arrivals STRICTLY after this position. `0` starts at the beginning of the ledger,
    /// and is also what an omitted field means — the same default the node applies.
    #[serde(default)]
    pub after_seq: u64,
    /// The page size. `None` asks for the node's default rather than a number this client invented;
    /// a node clamps whatever it is given to its own maximum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
control_call!(WalletArrivalsParams => ControlMethod::WalletArrivals, results::WalletArrivalsResult);

fn normalize_coin_id(coin_id: &str) -> Option<&str> {
    let normalized = coin_id.strip_prefix("0x").unwrap_or(coin_id);
    let well_formed = normalized.len() == COIN_ID_HEX_LEN
        && normalized
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    well_formed.then_some(normalized)
}

/// Give a single-coin-id params type its validating `Deserialize` and its `validated` constructor.
///
/// The three by-coin reads (`coinById`, `coinSpend`, `coinsByParent`) enforce the IDENTICAL id rule
/// under three different field names, and the rule is normative rather than incidental — see
/// [`WalletCoinByIdParams::validated`] for why a malformed id must be refused BEFORE a chain is
/// consulted. Written once here so a fourth by-coin read cannot arrive with a subtly looser copy of
/// it, and so a change to the rule cannot land on two of three types.
macro_rules! coin_id_params {
    ($ty:ident, $field:ident, $raw:ident, $error:expr) => {
        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                #[derive(Deserialize)]
                struct $raw {
                    $field: String,
                }

                let raw = $raw::deserialize(deserializer)?;
                let $field = normalize_coin_id(&raw.$field)
                    .ok_or_else(|| serde::de::Error::custom($error))?
                    .to_owned();
                Ok(Self { $field })
            }
        }

        impl $ty {
            /// Normalize and check the coin id, or reject the request as `-32602 INVALID_PARAMS`.
            ///
            /// A malformed id is a malformed REQUEST, and the node refuses it here — before
            /// consulting any chain. That ordering is normative rather than an optimisation: were a
            /// bad id allowed through, the read would come back empty, and the caller would be told
            /// the honest-looking answer *the chain holds nothing* about a coin it never actually
            /// asked after. An unanswerable question and a chain that answered "no" must never wear
            /// the same shape.
            ///
            /// Accepts exactly two spellings — 64 lowercase hex characters, or the same 64 preceded
            /// by `0x`. Uppercase, whitespace and every other length are refused, because the
            /// contract's hex wire form is lowercase and unprefixed everywhere else in this crate.
            pub fn validated(self) -> Result<Self, crate::error::ControlError> {
                let normalized = normalize_coin_id(&self.$field).ok_or_else(|| {
                    crate::error::ControlError::of(
                        crate::error::ControlErrorCode::InvalidParams,
                        $error,
                    )
                })?;
                Ok($ty {
                    $field: normalized.to_owned(),
                })
            }
        }
    };
}

const COIN_ID_ERROR: &str = "coin_id must be lowercase 64-hex, optionally 0x-prefixed";
const PARENT_COIN_ID_ERROR: &str =
    "parent_coin_id must be lowercase 64-hex, optionally 0x-prefixed";
const AFTER_COIN_ID_ERROR: &str = "after_coin_id must be lowercase 64-hex, optionally 0x-prefixed";

coin_id_params!(
    WalletCoinByIdParams,
    coin_id,
    RawWalletCoinByIdParams,
    COIN_ID_ERROR
);
coin_id_params!(
    WalletCoinSpendParams,
    coin_id,
    RawWalletCoinSpendParams,
    COIN_ID_ERROR
);
/// The page size `control.wallet.coinsByParent` uses when the caller names none.
///
/// A spend in the lineages this read exists to follow — a singleton, a DID, an ordinary transfer —
/// creates a small handful of children, so one default page covers a realistic hop in a single round
/// trip and a caller never pages at all.
pub const COINS_BY_PARENT_DEFAULT_LIMIT: u32 = 100;

/// The largest page `control.wallet.coinsByParent` will accept, derived from the transport's own
/// frame limit rather than chosen for feel.
///
/// dig-ipc-protocol caps a control frame at `MAX_FRAME_BYTES` = 1 MiB (its `SPEC.md` §
/// bounds), and that is the hard ceiling every answer on this plane has to fit inside. A
/// [`WalletCoinRecord`](results::WalletCoinRecord) is at most ~350 bytes of JSON — three 64-hex
/// hashes at 66 bytes quoted, a 20-digit `u64`, two 10-digit heights, and their keys — so the
/// arithmetic that fixes this number is:
///
/// ```text
/// 1 MiB / 350 B  ~=  2,996 records is where a page STOPS FITTING
/// 1,000 records  ~=  350 KB, roughly a third of the frame
/// ```
///
/// The cap is set at a third of what fits, not at what fits, so the envelope, the freshness fields
/// and any future additive member cannot push a legal page over the transport's limit. A larger
/// value would put the contract's own maximum inside the region where a conforming node's honest
/// answer is undeliverable — the failure would surface as a truncated frame, not as a refusal.
pub const COINS_BY_PARENT_MAX_LIMIT: u32 = 1_000;

const COINS_BY_PARENT_LIMIT_ERROR: &str = "limit must be between 1 and 1000";

impl<'de> Deserialize<'de> for WalletCoinsByParentParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawWalletCoinsByParentParams {
            parent_coin_id: String,
            #[serde(default)]
            after_coin_id: Option<String>,
            #[serde(default)]
            limit: Option<u32>,
        }

        let raw = RawWalletCoinsByParentParams::deserialize(deserializer)?;
        let parent_coin_id = normalize_coin_id(&raw.parent_coin_id)
            .ok_or_else(|| serde::de::Error::custom(PARENT_COIN_ID_ERROR))?
            .to_owned();
        let after_coin_id = raw
            .after_coin_id
            .map(|id| {
                normalize_coin_id(&id)
                    .map(str::to_owned)
                    .ok_or_else(|| serde::de::Error::custom(AFTER_COIN_ID_ERROR))
            })
            .transpose()?;
        if !raw.limit.map_or(true, is_legal_page) {
            return Err(serde::de::Error::custom(COINS_BY_PARENT_LIMIT_ERROR));
        }
        Ok(Self {
            parent_coin_id,
            after_coin_id,
            limit: raw.limit,
        })
    }
}

/// Is this a page size the contract accepts — at least one row, at most the frame-derived maximum?
fn is_legal_page(limit: u32) -> bool {
    (1..=COINS_BY_PARENT_MAX_LIMIT).contains(&limit)
}

impl WalletCoinsByParentParams {
    /// A first page of children for one parent: the node's default size, starting at the beginning.
    ///
    /// The common case, and the one a caller should not have to spell out — naming a page size means
    /// asserting a number this caller invented over the one the contract chose.
    pub fn first_page(parent_coin_id: impl Into<String>) -> Self {
        Self {
            parent_coin_id: parent_coin_id.into(),
            after_coin_id: None,
            limit: None,
        }
    }

    /// The page size this request asks for, resolving `None` to [`COINS_BY_PARENT_DEFAULT_LIMIT`].
    ///
    /// Stated once here so a node and a client cannot resolve the same omitted field to two
    /// different numbers — a disagreement that would show up as a page boundary in the wrong place,
    /// which is exactly where a paged walk loses rows.
    pub fn effective_limit(&self) -> u32 {
        self.limit.unwrap_or(COINS_BY_PARENT_DEFAULT_LIMIT)
    }

    /// Normalize and check both ids and the page bound, or reject as `-32602 INVALID_PARAMS`.
    ///
    /// The ids follow the rule every by-coin read in this crate follows (lowercase 64-hex, `0x`
    /// tolerated on input and never emitted).
    ///
    /// An out-of-range `limit` is REFUSED, never clamped. That is a deliberate departure from
    /// `control.wallet.arrivals`, which lets a node clamp: this read's page boundary is what a
    /// caller RESUMES from, so a silently shrunk page hands back a cursor for a position the caller
    /// did not ask about, and a caller that believed its own number would mis-size every subsequent
    /// request. Refusing keeps the caller's model of the page and the node's identical, which is the
    /// same reason a 65-hex coin id is refused rather than truncated.
    ///
    /// `limit: 0` is refused for a separate reason: a page that can hold nothing makes no progress,
    /// so a caller looping until a page comes back short would loop forever.
    pub fn validated(self) -> Result<Self, crate::error::ControlError> {
        fn invalid(message: &'static str) -> crate::error::ControlError {
            crate::error::ControlError::of(crate::error::ControlErrorCode::InvalidParams, message)
        }

        let parent_coin_id = normalize_coin_id(&self.parent_coin_id)
            .ok_or_else(|| invalid(PARENT_COIN_ID_ERROR))?
            .to_owned();
        let after_coin_id = self
            .after_coin_id
            .as_deref()
            .map(|id| {
                normalize_coin_id(id)
                    .map(str::to_owned)
                    .ok_or_else(|| invalid(AFTER_COIN_ID_ERROR))
            })
            .transpose()?;
        if !self.limit.map_or(true, is_legal_page) {
            return Err(invalid(COINS_BY_PARENT_LIMIT_ERROR));
        }
        Ok(WalletCoinsByParentParams {
            parent_coin_id,
            after_coin_id,
            limit: self.limit,
        })
    }
}

/// `control.wallet.peak` params — none.
///
/// The peak is a property of the node's chain view, not of any address, which is exactly why it is
/// its own method: [`results::WalletBalanceResult::peak_height`] is `null` on every fallback-tier
/// answer, so a caller bounding a claimed confirmation cannot rely on getting one from a balance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletPeakParams {}
control_call!(WalletPeakParams => ControlMethod::WalletPeak, results::WalletPeakResult);

/// `control.peerCounts` params — none.
///
/// The counts describe this node's own connectivity on each network, so there is nothing to scope
/// the question by. One call answers for BOTH networks deliberately: a consumer that had to collect
/// the DIG count from a peer method and the Chia count from a wallet method is a consumer that can
/// reach for the wrong one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerCountsParams {}
control_call!(PeerCountsParams => ControlMethod::PeerCounts, results::PeerCountsResult);

/// `control.wallet.syncStatus` params — none.
///
/// The wallet's sync progress is a property of the node's own chain replica, not of any address, so
/// there is nothing to scope the question by. Deliberately NOT confused with `control.sync.status`,
/// which reports §21 DIG STORE sync and has nothing to do with the chain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletSyncStatusParams {}
control_call!(WalletSyncStatusParams => ControlMethod::WalletSyncStatus, results::WalletSyncStatusResult);

/// `control.wallet.broadcast` params: an ALREADY-SIGNED spend bundle to push.
///
/// # The custody boundary (§908)
///
/// This carries signed bytes and nothing else. There is deliberately no key, no seed, no phrase and
/// no unsigned-spend-plus-key field here, and there never may be: the node's role on the money path
/// is to read chain state and to push what somebody else signed. A parameter that let the node
/// produce a signature would move custody into an identity-agnostic daemon, which is the one thing
/// the boundary exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletBroadcastParams {
    /// The signed spend bundle: lowercase hex of its chia `Streamable` serialization.
    pub signed_bundle_hex: String,
}
control_call!(WalletBroadcastParams => ControlMethod::WalletBroadcast, results::WalletBroadcastResult);

/// `pairing.request` params (OPEN — no token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestParams {
    /// A human-readable name for the requesting client (shown to the operator).
    pub client_name: String,
}
control_call!(RequestParams => ControlMethod::PairingRequest, results::PairingRequestResult);

/// `pairing.poll` params (OPEN — no token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PollParams {
    /// The pairing id to poll.
    pub pairing_id: String,
}
control_call!(PollParams => ControlMethod::PairingPoll, results::PairingPollResult);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::build_request;
    use serde_json::json;

    #[test]
    fn no_param_call_serializes_params_to_empty_object() {
        let req = build_request(1.into(), &StatusParams {});
        assert_eq!(req.method, "control.status");
        assert_eq!(req.params, json!({}));
    }

    #[test]
    fn data_param_call_carries_its_fields() {
        let req = build_request(2.into(), &SetCapParams { cap_bytes: 128 });
        assert_eq!(req.method, "control.cache.setCap");
        assert_eq!(req.params, json!({ "cap_bytes": 128 }));
    }

    #[test]
    fn pause_omits_until_when_indefinite() {
        assert_eq!(
            serde_json::to_value(PauseParams { until: None }).unwrap(),
            json!({})
        );
        assert_eq!(
            serde_json::to_value(PauseParams { until: Some(99) }).unwrap(),
            json!({ "until": 99 })
        );
    }

    #[test]
    fn method_binding_matches_the_catalog_name() {
        assert_eq!(
            SetUpstreamParams::METHOD.name(),
            "control.config.setUpstream"
        );
        assert_eq!(RequestParams::METHOD.name(), "pairing.request");
        assert_eq!(PollParams::METHOD.name(), "pairing.poll");
    }
}
