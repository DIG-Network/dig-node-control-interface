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

const COIN_ID_ERROR: &str = "coin_id must be lowercase 64-hex, optionally 0x-prefixed";

fn normalize_coin_id(coin_id: &str) -> Option<&str> {
    let normalized = coin_id.strip_prefix("0x").unwrap_or(coin_id);
    let well_formed = normalized.len() == COIN_ID_HEX_LEN
        && normalized
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    well_formed.then_some(normalized)
}

impl<'de> Deserialize<'de> for WalletCoinByIdParams {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawWalletCoinByIdParams {
            coin_id: String,
        }

        let raw = RawWalletCoinByIdParams::deserialize(deserializer)?;
        let coin_id = normalize_coin_id(&raw.coin_id)
            .ok_or_else(|| serde::de::Error::custom(COIN_ID_ERROR))?
            .to_owned();
        Ok(Self { coin_id })
    }
}

impl WalletCoinByIdParams {
    /// Normalize and check the coin id, or reject the request as `-32602 INVALID_PARAMS`.
    ///
    /// A malformed id is a malformed REQUEST, and the node refuses it here — before consulting any
    /// chain. That ordering is normative rather than an optimisation: were a bad id allowed through,
    /// the read would come back with no such coin, and the caller would be told the honest-looking
    /// answer `coin: null` about a coin it never actually asked after. An unanswerable question and
    /// a chain that answered "no" must never wear the same shape.
    ///
    /// Accepts exactly two spellings — 64 lowercase hex characters, or the same 64 preceded by
    /// `0x`. Uppercase, whitespace and every other length are refused, because the contract's hex
    /// wire form is lowercase and unprefixed everywhere else in this crate.
    pub fn validated(self) -> Result<Self, crate::error::ControlError> {
        let normalized = normalize_coin_id(&self.coin_id).ok_or_else(|| {
            crate::error::ControlError::of(
                crate::error::ControlErrorCode::InvalidParams,
                COIN_ID_ERROR,
            )
        })?;
        Ok(WalletCoinByIdParams {
            coin_id: normalized.to_owned(),
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
