//! Typed request params for the control methods, each bound to its method + result via
//! [`ControlCall`](crate::traits::ControlCall).
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
