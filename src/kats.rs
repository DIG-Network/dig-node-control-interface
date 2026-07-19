//! Conformance known-answer tests (KATs) — the anti-drift layer.
//!
//! These pin the exact wire shapes both sides must agree on: golden JSON-RPC request vectors for
//! every method, golden response vectors that the typed results decode-then-re-encode byte-for-byte,
//! and an end-to-end route through a mock [`ControlHandler`] proving the dispatcher maps every
//! method to its typed handler and back. Any client (extension/CLI/hub, T8–T10) and the node side
//! (T7) pin against these; a change that alters a wire shape fails here first.

use futures::executor::block_on;
use serde_json::{json, Value};

use crate::envelope::{JsonRpcRequest, JsonRpcResponse, RequestId};
use crate::error::{ControlError, ControlErrorCode};
use crate::method::ControlMethod;
use crate::params::*;
use crate::results;
use crate::traits::{build_request, parse_response, ControlHandler};

const STORE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ROOT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A golden request vector: build the typed call, assert it serializes to the exact envelope bytes.
fn assert_request<C: crate::traits::ControlCall>(call: &C, expected: Value) {
    let req = build_request(RequestId::Number(1), call);
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        expected,
        "request wire shape drifted for {}",
        C::METHOD.name()
    );
}

/// A golden response vector: the typed result decodes from `wire` and re-encodes byte-identically.
fn assert_result_round_trips<T>(wire: Value)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_value(wire.clone()).expect("golden result must decode");
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        wire,
        "result wire shape is not byte-stable"
    );
}

#[test]
fn golden_request_vectors() {
    assert_request(
        &StatusParams {},
        json!({"jsonrpc":"2.0","id":1,"method":"control.status","params":{}}),
    );
    assert_request(
        &SetUpstreamParams {
            upstream: "https://rpc.dig.net".into(),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.config.setUpstream","params":{"upstream":"https://rpc.dig.net"}}),
    );
    assert_request(
        &SetLevelParams {
            filter: "info,dig_node_core=debug".into(),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.log.setLevel","params":{"filter":"info,dig_node_core=debug"}}),
    );
    assert_request(
        &SetCapParams {
            cap_bytes: 67108864,
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.cache.setCap","params":{"cap_bytes":67108864}}),
    );
    assert_request(
        &PinParams {
            store: format!("{STORE}:{ROOT}"),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.hostedStores.pin","params":{"store":format!("{STORE}:{ROOT}")}}),
    );
    assert_request(
        &SyncTriggerParams {
            store: format!("{STORE}:{ROOT}"),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.sync.trigger","params":{"store":format!("{STORE}:{ROOT}")}}),
    );
    assert_request(
        &PauseParams {
            until: Some(1_800_000_000),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.updater.pause","params":{"until":1800000000}}),
    );
    assert_request(
        &ApproveParams {
            pairing_id: "pid-1".into(),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.pairing.approve","params":{"pairing_id":"pid-1"}}),
    );
    assert_request(
        &PeersConnectParams {
            peer: "1.2.3.4:9257".into(),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.peers.connect","params":{"peer":"1.2.3.4:9257"}}),
    );
    assert_request(
        &SubscribeParams {
            store_id: STORE.into(),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.subscribe","params":{"store_id":STORE}}),
    );
    assert_request(
        &RequestParams {
            client_name: "DIG extension".into(),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"pairing.request","params":{"client_name":"DIG extension"}}),
    );
}

#[test]
fn golden_response_result_vectors_are_byte_stable() {
    assert_result_round_trips::<results::StatusResult>(json!({
        "running": true, "service": "dig-node", "version": "0.30.0", "commit": "deadbee",
        "protocol": "21", "uptime_secs": 42, "addr": "127.0.0.1:9256",
        "upstream": "https://rpc.dig.net",
        "cache": {"cap_bytes": 67108864, "used_bytes": 1024, "dir": "/var/cache/dig", "shared": true},
        "hosted_store_count": 3, "cached_capsule_count": 5, "pinned_store_count": 2,
        "sync": {"available": true}
    }));
    assert_result_round_trips::<results::ConfigResult>(json!({
        "addr": "127.0.0.1:9256", "port": "9256", "upstream": "https://rpc.dig.net",
        "upstream_override": null, "cache_dir": "/var/cache/dig", "cache_shared": true,
        "config_path": "/etc/dig/config.json", "sync_available": false
    }));
    assert_result_round_trips::<results::CacheView>(json!({
        "cap_bytes": 67108864, "used_bytes": 0, "dir": "/c", "shared": false
    }));
    assert_result_round_trips::<results::HostedStoresListResult>(json!({
        "stores": [{
            "store_id": STORE, "pinned": true, "capsule_count": 1, "total_bytes": 10,
            "capsules": [{"capsule": format!("{STORE}:{ROOT}"), "root": ROOT,
                          "size_bytes": 10, "last_used_unix_ms": 1700000000000u64}]
        }]
    }));
    assert_result_round_trips::<results::SyncStatusResult>(json!({
        "available": true, "method": "section-21-whole-store-sync",
        "pinned_total": 2, "pinned_synced": 1, "whole_store_trigger_supported": false
    }));
    assert_result_round_trips::<results::SyncTriggerResult>(json!({
        "store_id": STORE, "root": ROOT, "status": "synced",
        "size_bytes": 2048, "served_root": ROOT
    }));
    assert_result_round_trips::<results::SubscribeResult>(json!({
        "subscribed": true, "added": true, "store_id": STORE
    }));
    assert_result_round_trips::<results::ListSubscriptionsResult>(json!({
        "subscriptions": [STORE], "count": 1
    }));
    assert_result_round_trips::<results::PairingApproveResult>(json!({
        "approved": true, "client_name": "DIG extension", "token_id": "abcd1234"
    }));
    assert_result_round_trips::<results::PairingPollResult>(json!({
        "status": "approved", "token": "deadbeef"
    }));
}

#[test]
fn error_envelope_golden_vector() {
    let resp = JsonRpcResponse::error(
        RequestId::Number(1),
        ControlError::of(ControlErrorCode::Unauthorized, "control.* requires a token"),
    );
    assert_eq!(
        serde_json::to_value(&resp).unwrap(),
        json!({
            "jsonrpc": "2.0", "id": 1,
            "error": {
                "code": -32030,
                "message": "control.* requires a token",
                "data": {"code": "UNAUTHORIZED", "origin": "shell"}
            }
        })
    );
}

/// A mock node that serves canned typed results — exercises the [`ControlHandler`] dispatcher for
/// every method group without a running node.
struct MockNode;

#[async_trait::async_trait]
impl ControlHandler for MockNode {
    async fn status(&self) -> Result<results::StatusResult, ControlError> {
        Ok(results::StatusResult {
            running: true,
            service: "dig-node".into(),
            version: "0.30.0".into(),
            commit: "deadbee".into(),
            protocol: "21".into(),
            uptime_secs: 1,
            addr: "127.0.0.1:9256".into(),
            upstream: "https://rpc.dig.net".into(),
            cache: results::CacheView {
                cap_bytes: 67108864,
                used_bytes: 0,
                dir: "/c".into(),
                shared: false,
            },
            hosted_store_count: 0,
            cached_capsule_count: 0,
            pinned_store_count: 0,
            sync: results::SyncAvailability { available: false },
        })
    }
    async fn config_get(&self) -> Result<results::ConfigResult, ControlError> {
        Err(unimpl("config_get"))
    }
    async fn config_set_upstream(
        &self,
        params: SetUpstreamParams,
    ) -> Result<results::SetUpstreamResult, ControlError> {
        Ok(results::SetUpstreamResult {
            upstream: params.upstream,
            requires_restart: true,
        })
    }
    async fn log_set_level(
        &self,
        params: SetLevelParams,
    ) -> Result<results::SetLevelResult, ControlError> {
        Ok(results::SetLevelResult {
            filter: params.filter,
        })
    }
    async fn cache_get(&self) -> Result<results::CacheView, ControlError> {
        Err(unimpl("cache_get"))
    }
    async fn cache_set_cap(
        &self,
        params: SetCapParams,
    ) -> Result<results::SetCapResult, ControlError> {
        Ok(results::SetCapResult {
            cap_bytes: params.cap_bytes.max(64 * 1024 * 1024),
        })
    }
    async fn cache_clear(&self) -> Result<results::CacheClearResult, ControlError> {
        Ok(results::CacheClearResult { cleared: true })
    }
    async fn hosted_stores_list(&self) -> Result<results::HostedStoresListResult, ControlError> {
        Ok(results::HostedStoresListResult { stores: vec![] })
    }
    async fn hosted_stores_pin(
        &self,
        params: PinParams,
    ) -> Result<results::PinResult, ControlError> {
        Ok(results::PinResult {
            store_id: params.store,
            root: None,
            pinned: true,
            fetch: json!({"status": "skipped"}),
        })
    }
    async fn hosted_stores_unpin(
        &self,
        params: UnpinParams,
    ) -> Result<results::UnpinResult, ControlError> {
        Ok(results::UnpinResult {
            store_id: params.store,
            unpinned: true,
            evicted_capsules: 0,
        })
    }
    async fn hosted_stores_status(
        &self,
        params: HostedStoreStatusParams,
    ) -> Result<results::HostedStoreStatusResult, ControlError> {
        Ok(results::HostedStoreStatusResult {
            store_id: params.store,
            pinned: false,
            capsule_count: 0,
            total_bytes: 0,
            capsules: vec![],
        })
    }
    async fn sync_status(&self) -> Result<results::SyncStatusResult, ControlError> {
        Err(unimpl("sync_status"))
    }
    async fn sync_trigger(
        &self,
        params: SyncTriggerParams,
    ) -> Result<results::SyncTriggerResult, ControlError> {
        let (store_id, root) = params.store.split_once(':').unwrap_or((&params.store, ""));
        Ok(results::SyncTriggerResult {
            store_id: store_id.into(),
            root: root.into(),
            status: "synced".into(),
            size_bytes: 1,
            served_root: root.into(),
        })
    }
    async fn updater_status(&self) -> Result<Value, ControlError> {
        Ok(json!({"channel": "stable"}))
    }
    async fn updater_set_channel(&self, params: SetChannelParams) -> Result<Value, ControlError> {
        Ok(json!({"channel": params.channel}))
    }
    async fn updater_pause(&self, params: PauseParams) -> Result<Value, ControlError> {
        Ok(json!({"paused": true, "until": params.until}))
    }
    async fn updater_resume(&self) -> Result<Value, ControlError> {
        Ok(json!({"paused": false}))
    }
    async fn updater_check_now(&self) -> Result<Value, ControlError> {
        Ok(json!({"checked": true}))
    }
    async fn pairing_list(&self) -> Result<Value, ControlError> {
        Ok(json!({"pending": [], "tokens": []}))
    }
    async fn pairing_approve(
        &self,
        params: ApproveParams,
    ) -> Result<results::PairingApproveResult, ControlError> {
        Ok(results::PairingApproveResult {
            approved: true,
            client_name: params.pairing_id,
            token_id: "abcd1234".into(),
        })
    }
    async fn pairing_revoke(
        &self,
        params: RevokeParams,
    ) -> Result<results::PairingRevokeResult, ControlError> {
        Ok(results::PairingRevokeResult {
            revoked: true,
            token_id: params.token_id,
        })
    }
    async fn peer_status(&self) -> Result<Value, ControlError> {
        Ok(json!({"running": false}))
    }
    async fn peers_connect(
        &self,
        params: PeersConnectParams,
    ) -> Result<results::PeersConnectResult, ControlError> {
        Ok(results::PeersConnectResult {
            connected: true,
            peer_id: params.peer,
        })
    }
    async fn peers_disconnect(
        &self,
        params: PeersDisconnectParams,
    ) -> Result<results::PeersDisconnectResult, ControlError> {
        Ok(results::PeersDisconnectResult {
            disconnected: true,
            peer_id: params.peer,
        })
    }
    async fn subscribe(
        &self,
        params: SubscribeParams,
    ) -> Result<results::SubscribeResult, ControlError> {
        Ok(results::SubscribeResult {
            subscribed: true,
            added: true,
            store_id: params.store_id,
        })
    }
    async fn unsubscribe(
        &self,
        params: UnsubscribeParams,
    ) -> Result<results::UnsubscribeResult, ControlError> {
        Ok(results::UnsubscribeResult {
            subscribed: false,
            removed: true,
            store_id: params.store_id,
        })
    }
    async fn list_subscriptions(&self) -> Result<results::ListSubscriptionsResult, ControlError> {
        Ok(results::ListSubscriptionsResult {
            subscriptions: vec![],
            count: 0,
        })
    }
    async fn pairing_request(
        &self,
        _params: RequestParams,
    ) -> Result<results::PairingRequestResult, ControlError> {
        Ok(results::PairingRequestResult {
            pairing_id: "pid-1".into(),
            pairing_code: "012345".into(),
            expires_ms: 1_700_000_000_000,
        })
    }
    async fn pairing_poll(
        &self,
        _params: PollParams,
    ) -> Result<results::PairingPollResult, ControlError> {
        Ok(results::PairingPollResult {
            status: "pending".into(),
            token: None,
        })
    }
}

fn unimpl(what: &str) -> ControlError {
    ControlError::of(ControlErrorCode::NotSupported, format!("{what} not mocked"))
}

/// Route a typed call through the mock node's dispatcher and parse the typed result back.
fn round_trip<C: crate::traits::ControlCall>(call: &C) -> Result<C::Output, ControlError> {
    let node = MockNode;
    let req = build_request(RequestId::Number(1), call);
    let resp = block_on(node.dispatch(req));
    parse_response::<C>(resp)
}

#[test]
fn dispatcher_routes_every_taking_params_method_to_its_typed_handler() {
    assert!(round_trip(&StatusParams {}).unwrap().running);
    assert_eq!(
        round_trip(&SetCapParams { cap_bytes: 1 })
            .unwrap()
            .cap_bytes,
        64 * 1024 * 1024,
        "the node floors the cap"
    );
    assert!(round_trip(&CacheClearParams {}).unwrap().cleared);
    let pin = round_trip(&PinParams {
        store: STORE.into(),
    })
    .unwrap();
    assert_eq!(pin.store_id, STORE);
    let sync = round_trip(&SyncTriggerParams {
        store: format!("{STORE}:{ROOT}"),
    })
    .unwrap();
    assert_eq!(sync.root, ROOT);
    let sub = round_trip(&SubscribeParams {
        store_id: STORE.into(),
    })
    .unwrap();
    assert!(sub.added);
    let conn = round_trip(&PeersConnectParams { peer: "p".into() }).unwrap();
    assert_eq!(conn.peer_id, "p");
    assert_eq!(
        round_trip(&UpdaterStatusParams {}).unwrap(),
        json!({"channel": "stable"})
    );
    assert_eq!(
        round_trip(&PollParams {
            pairing_id: "x".into()
        })
        .unwrap()
        .status,
        "pending"
    );
}

#[test]
fn default_control_client_builds_and_parses_via_the_trait() {
    use crate::traits::{ControlClient, DefaultControlClient};
    let client = DefaultControlClient;
    let req = client.build_request(RequestId::Number(9), &SetCapParams { cap_bytes: 5 });
    assert_eq!(req.id, RequestId::Number(9));
    assert_eq!(req.method, "control.cache.setCap");
    let resp = JsonRpcResponse::success(RequestId::Number(9), json!({"cap_bytes": 5}));
    let out = client
        .parse_response::<SetCapParams>(resp)
        .expect("typed parse");
    assert_eq!(out.cap_bytes, 5);
}

#[test]
fn every_method_maps_to_a_category() {
    use crate::method::Category;
    // Touch the category of every method so the grouping table can't silently lose an arm.
    assert_eq!(ControlMethod::Status.category(), Category::Status);
    assert_eq!(ControlMethod::PeerStatus.category(), Category::Peers);
    for &m in ControlMethod::ALL {
        let _ = m.category();
    }
}

#[test]
fn dispatcher_rejects_an_unknown_method_with_method_not_found() {
    let node = MockNode;
    let req = JsonRpcRequest::new(RequestId::Number(1), "control.nope", json!({}));
    let resp = block_on(node.dispatch(req));
    let err = resp.into_result().unwrap_err();
    assert_eq!(err.code_enum(), Some(ControlErrorCode::MethodNotFound));
}

#[test]
fn dispatcher_maps_malformed_params_to_invalid_params() {
    let node = MockNode;
    // `control.cache.setCap` needs a numeric `cap_bytes`; a string is malformed.
    let req = JsonRpcRequest::new(
        RequestId::Number(1),
        ControlMethod::CacheSetCap.name(),
        json!({"cap_bytes": "not-a-number"}),
    );
    let resp = block_on(node.dispatch(req));
    let err = resp.into_result().unwrap_err();
    assert_eq!(err.code_enum(), Some(ControlErrorCode::InvalidParams));
}

#[test]
fn dispatcher_surfaces_a_handler_error_verbatim() {
    let node = MockNode;
    let req = build_request(RequestId::Number(1), &ConfigGetParams {});
    let resp = block_on(node.dispatch(req));
    let err = resp.into_result().unwrap_err();
    assert_eq!(err.code_enum(), Some(ControlErrorCode::NotSupported));
}

#[test]
fn every_catalog_method_dispatches_without_panicking() {
    // The dispatcher must have an arm for EVERY catalog method — a missing arm would not compile,
    // but this also proves no method routes to a MethodNotFound (i.e. the name table + the match
    // agree for all 29 methods).
    let node = MockNode;
    for &m in ControlMethod::ALL {
        let req = JsonRpcRequest::new(RequestId::Number(1), m.name(), minimal_params(m));
        let resp = block_on(node.dispatch(req));
        if let Some(err) = &resp.error {
            assert_ne!(
                err.code_enum(),
                Some(ControlErrorCode::MethodNotFound),
                "{} routed to MethodNotFound — the dispatcher is missing an arm",
                m.name()
            );
        }
    }
}

/// The smallest valid params object for a method, so the coverage sweep above never trips
/// `INVALID_PARAMS` for a param-taking method.
fn minimal_params(m: ControlMethod) -> Value {
    match m {
        ControlMethod::ConfigSetUpstream => json!({"upstream": ""}),
        ControlMethod::LogSetLevel => json!({"filter": "info"}),
        ControlMethod::CacheSetCap => json!({"cap_bytes": 0}),
        ControlMethod::HostedStoresPin
        | ControlMethod::HostedStoresUnpin
        | ControlMethod::HostedStoresStatus
        | ControlMethod::SyncTrigger => json!({"store": STORE}),
        ControlMethod::UpdaterSetChannel => json!({"channel": "stable"}),
        ControlMethod::UpdaterPause => json!({}),
        ControlMethod::PairingApprove => json!({"pairing_id": "x"}),
        ControlMethod::PairingRevoke => json!({"token_id": "x"}),
        ControlMethod::PeersConnect | ControlMethod::PeersDisconnect => json!({"peer": "p"}),
        ControlMethod::Subscribe | ControlMethod::Unsubscribe => json!({"store_id": STORE}),
        ControlMethod::PairingRequest => json!({"client_name": "c"}),
        ControlMethod::PairingPoll => json!({"pairing_id": "x"}),
        _ => json!({}),
    }
}
