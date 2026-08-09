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

/// The one bundle [`MockNode`] refuses, so the accepted and rejected branches are both reachable
/// from the SAME handler — a mock that could only accept could not express a refusal at all.
const REJECTED_BUNDLE: &str = "beef";

/// The one upstream [`MockNode`] refuses, so a handler error stays reachable from the SAME handler
/// even though `every_catalog_method_dispatches_without_panicking` requires every method to SUCCEED
/// under its minimal params. The refusal has to hang off a value the minimal fixture never uses;
/// making a whole method unsupported instead would trip that test, and silencing it there would put
/// back the tolerate-any-error hole the test exists to close.
const UNSUPPORTED_UPSTREAM: &str = "__not_supported__";

/// The one coin id [`MockNode`] knows about — SPENT, which is the state `control.wallet.coins`
/// can never report and the state a mint observation turns on.
const SPENT_COIN: &str = "abababababababababababababababababababababababababababababababab";

/// A well-formed coin id [`MockNode`] has never heard of, so "absent" is reachable from the same
/// handler as "found" and the two answers can be told apart.
const ABSENT_COIN: &str = "0101010101010101010101010101010101010101010101010101010101010101";

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
    assert_request(
        &WalletBalanceParams {
            address: "xch1exampleaddr".into(),
            asset: Asset::Dig,
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.wallet.balance","params":{"address":"xch1exampleaddr","asset":"dig"}}),
    );
    assert_request(
        &WalletCoinsParams {
            address: "xch1exampleaddr".into(),
            asset: Asset::Xch,
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.wallet.coins","params":{"address":"xch1exampleaddr","asset":"xch"}}),
    );
    assert_request(
        &WalletCoinByIdParams {
            coin_id: "ab".repeat(32),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.wallet.coinById","params":{"coin_id":"ab".repeat(32)}}),
    );
    assert_request(
        &WalletPeakParams {},
        json!({"jsonrpc":"2.0","id":1,"method":"control.wallet.peak","params":{}}),
    );
    assert_request(
        &PeerCountsParams {},
        json!({"jsonrpc":"2.0","id":1,"method":"control.peerCounts","params":{}}),
    );
    assert_request(
        &WalletSyncStatusParams {},
        json!({"jsonrpc":"2.0","id":1,"method":"control.wallet.syncStatus","params":{}}),
    );
    assert_request(
        &WalletBroadcastParams {
            signed_bundle_hex: "deadbeef".into(),
        },
        json!({"jsonrpc":"2.0","id":1,"method":"control.wallet.broadcast","params":{"signed_bundle_hex":"deadbeef"}}),
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
    assert_result_round_trips::<results::WalletBalanceResult>(json!({
        "balance": 1234u64, "pending": 0u64,
        "source": "db", "synced": true, "peak_height": 5000000u32
    }));
    assert_result_round_trips::<results::WalletCoinsResult>(json!({
        "coins": [{
            "coin_id": "aa".repeat(32), "asset": "xch", "amount": 1_750_000_000_000u64,
            "parent_coin_info": "bb".repeat(32), "puzzle_hash": "cc".repeat(32),
            "created_height": 5_000_000u32, "spent_height": null
        }],
        "source": "db", "synced": true, "peak_height": 5_000_000u32
    }));
    // A chain that was consulted and holds nothing. It must be expressible as a SUCCESS, because
    // that is the only shape that leaves "unreachable" free to be an error.
    assert_result_round_trips::<results::WalletCoinsResult>(json!({
        "coins": [], "source": "fallback", "synced": false, "peak_height": null
    }));
    // `control.wallet.coinById` — the SPENT coin. No `.coins` vector can carry one: that method
    // answers with unspent coins only, which is exactly why observing a mint needs this method.
    // `asset` is null because a coin id alone does not classify a coin (see `WalletCoinRecord`).
    assert_result_round_trips::<results::WalletCoinByIdResult>(json!({
        "coin": {
            "coin_id": "ab".repeat(32), "asset": null, "amount": 1_000_000_000_000u64,
            "parent_coin_info": "bb".repeat(32), "puzzle_hash": "cc".repeat(32),
            "created_height": 5_000_000u32, "spent_height": 5_000_042u32
        },
        "source": "fallback", "synced": false, "peak_height": null
    }));
    // A chain that was consulted and holds no such coin. A SUCCESS, so that "unreachable" stays
    // free to be an error.
    assert_result_round_trips::<results::WalletCoinByIdResult>(json!({
        "coin": null, "source": "fallback", "synced": false, "peak_height": null
    }));
    // The DB tier, which every other by-id vector leaves unexercised. `source:"db"` means the node's
    // OWN replica answered, so it carries `synced:true` and that replica's peak — the same binding
    // the by-address reads have always had. A db answer reporting `synced:false` would be a stale
    // view issuing a verdict; see `WalletCoinByIdResult`.
    assert_result_round_trips::<results::WalletCoinByIdResult>(json!({
        "coin": {
            "coin_id": "ab".repeat(32), "asset": null, "amount": 1_000_000_000_000u64,
            "parent_coin_info": "bb".repeat(32), "puzzle_hash": "cc".repeat(32),
            "created_height": 5_000_000u32, "spent_height": null
        },
        "source": "db", "synced": true, "peak_height": 5_000_100u32
    }));
    assert_result_round_trips::<results::WalletPeakResult>(json!({
        "peak_height": 5_000_000u32, "synced": true
    }));
    assert_result_round_trips::<results::WalletPeakResult>(json!({
        "peak_height": null, "synced": false
    }));
    // `control.wallet.syncStatus`, all three phases. `not_started` carries a null height rather
    // than 0 -- a wallet that has never synced and a wallet synced to the genesis block must not
    // wear the same shape -- and `chia_peer_count` is present in every one of them, because the
    // count is what turns "syncing" into either "syncing" or "syncing, connected to nothing".
    assert_result_round_trips::<results::WalletSyncStatusResult>(json!({
        "phase": "not_started", "peak_height": null, "chia_peer_count": 0u32
    }));
    assert_result_round_trips::<results::WalletSyncStatusResult>(json!({
        "phase": "syncing", "peak_height": 4_000_000u32, "chia_peer_count": 3u32
    }));
    assert_result_round_trips::<results::WalletSyncStatusResult>(json!({
        "phase": "synced", "peak_height": 5_000_000u32, "chia_peer_count": 5u32
    }));
    // A node that cannot observe the peer count at all: `null`, which is NOT `0`. `0` is a measured
    // zero and licenses "syncing -- no peers"; `null` licenses no claim about connectivity.
    assert_result_round_trips::<results::WalletSyncStatusResult>(json!({
        "phase": "syncing", "peak_height": null, "chia_peer_count": null
    }));
    // `control.peerCounts` — the two networks, each named. A node connected to both.
    assert_result_round_trips::<results::PeerCountsResult>(json!({
        "dig_peer_count": 6u32, "chia_peer_count": 3u32
    }));
    // Observed zeros on BOTH networks: the node looked and found nothing connected. Pinned
    // separately from the null case below, because a node with no peer network RUNNING has an
    // unknown count and must not report it as a measured zero.
    assert_result_round_trips::<results::PeerCountsResult>(json!({
        "dig_peer_count": 0u32, "chia_peer_count": 0u32
    }));
    // Unobservable on BOTH networks. `null` is not `0` for either count.
    assert_result_round_trips::<results::PeerCountsResult>(json!({
        "dig_peer_count": null, "chia_peer_count": null
    }));
    // One of each, and this vector is the one that matters: it varies the two counts INDEPENDENTLY,
    // so a serialization that transposed the two field names — the nearest wrong implementation, and
    // exactly the confusion this method exists to end — cannot survive it.
    assert_result_round_trips::<results::PeerCountsResult>(json!({
        "dig_peer_count": 6u32, "chia_peer_count": null
    }));
    assert_result_round_trips::<results::WalletBroadcastResult>(json!({
        "accepted": true, "transaction_id": "dd".repeat(32), "rejection": null
    }));
    assert_result_round_trips::<results::WalletBroadcastResult>(json!({
        "accepted": false, "transaction_id": null, "rejection": "DOUBLE_SPEND"
    }));
    // A fallback-tier answer: `synced` is false and `peak_height` is present as `null` (never
    // omitted), because neither describes a figure the node's own replica did not produce.
    assert_result_round_trips::<results::WalletBalanceResult>(json!({
        "balance": 0u64, "pending": 7u64,
        "source": "fallback", "synced": false, "peak_height": null
    }));
}

/// `source` is ADDITIVE in BOTH directions (dig_ecosystem#2233), which is the property that lets
/// this crate ship ahead of the nodes that emit the field.
///
/// The fixture that matters is the one WITHOUT the key: a node released before tier disclosure
/// emits no `source` at all, and 0.5.0 must still parse that payload. A test that only round-trips
/// a payload carrying the field cannot see a missing `#[serde(default)]` — the field is required on
/// deserialize, every fixture supplies it, and the break surfaces only against a real older node.
#[test]
fn a_pre_disclosure_nodes_payload_still_parses_with_the_tier_unknown() {
    let legacy = json!({
        "balance": 1234u64, "pending": 0u64, "synced": true, "peak_height": 5000000u32
    });
    let parsed: results::WalletBalanceResult =
        serde_json::from_value(legacy).expect("a node predating `source` must still deserialize");

    assert_eq!(parsed.balance, 1234);
    assert_eq!(
        parsed.source, None,
        "an absent tier is UNKNOWN -- never silently reported as one of the two tiers"
    );
}

/// The two tiers spell themselves on the wire as the lowercase tokens dig-node emits, pinned
/// literally so a Rust variant rename cannot silently change what a consumer must match on.
#[test]
fn the_tier_tokens_are_the_lowercase_wire_spellings() {
    for (src, wire) in [
        (results::WalletReadSource::Db, "db"),
        (results::WalletReadSource::Fallback, "fallback"),
    ] {
        assert_eq!(serde_json::to_value(src).unwrap(), json!(wire));
        assert_eq!(
            serde_json::from_value::<results::WalletReadSource>(json!(wire)).unwrap(),
            src
        );
    }
}

/// The three wallet-sync phases spell themselves on the wire as these exact snake_case tokens,
/// pinned literally so renaming a Rust variant cannot silently change what a consumer must match on.
///
/// The set is asserted to be exactly three as well: a fourth phase would be a wire change every
/// consumer's match must be told about, not something that may arrive unannounced.
#[test]
fn the_wallet_sync_phase_tokens_are_the_snake_case_wire_spellings() {
    let pinned = [
        (results::WalletSyncPhase::NotStarted, "not_started"),
        (results::WalletSyncPhase::Syncing, "syncing"),
        (results::WalletSyncPhase::Synced, "synced"),
    ];
    for (phase, wire) in pinned {
        assert_eq!(serde_json::to_value(phase).unwrap(), json!(wire));
        assert_eq!(
            serde_json::from_value::<results::WalletSyncPhase>(json!(wire)).unwrap(),
            phase
        );
    }
    assert_eq!(
        results::WalletSyncPhase::ALL.len(),
        pinned.len(),
        "the phase set is exactly the three pinned above"
    );
}

/// **`not_started` and a synced-at-genesis wallet are different values.** The fixture varies ONLY the
/// phase while holding the height at the one value a bool-plus-height shape would collapse, so an
/// implementation that reported progress as `synced: false` + `peak_height: 0` -- the nearest wrong
/// shape, and the one `WalletPeakResult` already has -- cannot pass.
#[test]
fn never_started_is_distinguishable_from_synced_at_height_zero() {
    let never_started = serde_json::to_value(results::WalletSyncStatusResult {
        phase: results::WalletSyncPhase::NotStarted,
        peak_height: None,
        chia_peer_count: Some(0),
    })
    .unwrap();
    let synced_at_genesis = serde_json::to_value(results::WalletSyncStatusResult {
        phase: results::WalletSyncPhase::Synced,
        peak_height: Some(0),
        chia_peer_count: Some(1),
    })
    .unwrap();

    assert_ne!(never_started, synced_at_genesis);
    assert_eq!(never_started["peak_height"], json!(null));
    assert_eq!(
        synced_at_genesis["peak_height"],
        json!(0),
        "height 0 is a real height and must survive the round trip as one"
    );
}

/// **Each count names its own network, and the two are not interchangeable.**
///
/// The wire keys are pinned as LITERALS rather than derived from the struct, because the whole
/// reason this method exists is that a consumer reaching for a bare `peers`/`connected_peers`/
/// `peer_count` eventually reaches for the wrong network and gets a plausible number. A rename that
/// dropped a network out of a key would be invisible to a derived assertion.
///
/// The fixture gives the two counts DIFFERENT values, so an implementation that transposed the
/// fields — the nearest wrong one — fails here rather than passing on a shape both share.
#[test]
fn each_peer_count_key_names_its_network() {
    let wire = serde_json::to_value(results::PeerCountsResult {
        dig_peer_count: Some(6),
        chia_peer_count: Some(3),
    })
    .unwrap();

    assert_eq!(wire["dig_peer_count"], json!(6));
    assert_eq!(wire["chia_peer_count"], json!(3));
    // The KEY SET, exactly -- so a bare `peers` / `connected_peers` / `peer_count` cannot appear
    // alongside the named pair either. (`serde_json::Value` orders its map, so this is a set
    // assertion by construction; the emitted TEXT below is what a consumer actually parses.)
    let keys: Vec<&str> = wire
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, vec!["chia_peer_count", "dig_peer_count"]);
    assert_eq!(
        serde_json::to_string(&results::PeerCountsResult {
            dig_peer_count: Some(6),
            chia_peer_count: Some(3),
        })
        .unwrap(),
        r#"{"dig_peer_count":6,"chia_peer_count":3}"#,
        "the emitted bytes name each network and give the two counts independently"
    );
}

/// **The two `chia_peer_count` fields are ONE observation under ONE key.**
///
/// The field is deliberately duplicated across `control.peerCounts` and
/// `control.wallet.syncStatus`, and the contract's answer to that duplication is that a conforming
/// node serves both from a single source. That is a node obligation this crate cannot execute — but
/// it can pin the half that makes the obligation expressible: the two payloads must spell the count
/// with the IDENTICAL key, so a node reading one type's field name and emitting the other's cannot
/// arise, and a consumer can compare the two answers at all.
#[test]
fn both_results_spell_the_chia_count_with_the_same_key() {
    const KEY: &str = "chia_peer_count";

    let counts = serde_json::to_value(results::PeerCountsResult {
        dig_peer_count: None,
        chia_peer_count: Some(3),
    })
    .unwrap();
    let sync = serde_json::to_value(results::WalletSyncStatusResult {
        phase: results::WalletSyncPhase::Syncing,
        peak_height: Some(4_000_000),
        chia_peer_count: Some(3),
    })
    .unwrap();

    assert_eq!(counts[KEY], json!(3));
    assert_eq!(sync[KEY], json!(3));
    assert_eq!(
        counts[KEY], sync[KEY],
        "a single node's two answers describe the same observation and must agree"
    );
}

/// **`null` and `0` are different answers for each count, independently.**
///
/// Varies ONE count at a time against a truthful control, so a shape that collapsed absent into zero
/// on either field alone is caught. A node with no peer network running has an UNKNOWN count; a
/// zero there would report "nothing is connected" about a network it never asked.
#[test]
fn an_unobservable_count_is_null_not_zero_on_either_network() {
    let dig_unknown = serde_json::to_value(results::PeerCountsResult {
        dig_peer_count: None,
        chia_peer_count: Some(0),
    })
    .unwrap();
    assert_eq!(dig_unknown["dig_peer_count"], json!(null));
    assert_eq!(
        dig_unknown["chia_peer_count"],
        json!(0),
        "an observed zero must survive as a zero beside an unknown"
    );

    let chia_unknown = serde_json::to_value(results::PeerCountsResult {
        dig_peer_count: Some(0),
        chia_peer_count: None,
    })
    .unwrap();
    assert_eq!(chia_unknown["dig_peer_count"], json!(0));
    assert_eq!(chia_unknown["chia_peer_count"], json!(null));

    assert_ne!(dig_unknown, chia_unknown);
}

/// The "no dig-app code change" guarantee, pinned: the node's richer `WalletBalanceResult` is a
/// strict SUPERSET of dig-app's frozen `BalanceResponse { balance }`, so dig-app deserializes the
/// node's payload losslessly (its struct does not deny unknown fields) and reads the confirmed
/// balance. This mirrors dig-app's `dig-app-core::wallet::engine::BalanceResponse` byte-for-byte.
#[test]
fn node_balance_superset_is_readable_by_dig_apps_balance_struct() {
    /// Byte-identical mirror of dig-app's frozen `BalanceResponse` — NO `deny_unknown_fields`, so the
    /// node's extra fields (`pending`/`source`/`synced`/`peak_height`) are ignored, not rejected.
    #[derive(serde::Deserialize)]
    struct DigAppBalanceResponse {
        balance: u64,
    }

    // The node emits the full superset...
    let node_payload = serde_json::to_value(results::WalletBalanceResult {
        balance: 9_999,
        pending: 42,
        source: Some(results::WalletReadSource::Db),
        synced: true,
        peak_height: Some(6_123_456),
    })
    .unwrap();

    // ...and dig-app's `{balance}` struct reads it without any code change on dig-app's side.
    let app: DigAppBalanceResponse =
        serde_json::from_value(node_payload).expect("dig-app must read the node's richer payload");
    assert_eq!(
        app.balance, 9_999,
        "dig-app must read the confirmed balance verbatim"
    );
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
        Ok(results::ConfigResult {
            addr: "127.0.0.1".into(),
            port: "9256".into(),
            upstream: "https://rpc.dig.net".into(),
            upstream_override: None,
            cache_dir: "/tmp/cache".into(),
            cache_shared: false,
            config_path: "/tmp/config.json".into(),
            sync_available: true,
        })
    }
    async fn config_set_upstream(
        &self,
        params: SetUpstreamParams,
    ) -> Result<results::SetUpstreamResult, ControlError> {
        if params.upstream == UNSUPPORTED_UPSTREAM {
            return Err(unimpl("config_set_upstream"));
        }
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
        Ok(results::CacheView {
            cap_bytes: 64 * 1024 * 1024,
            used_bytes: 1024,
            dir: "/tmp/cache".into(),
            shared: false,
        })
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
        Ok(results::SyncStatusResult {
            available: true,
            method: "dig-sync".into(),
            pinned_total: 0,
            pinned_synced: 0,
            whole_store_trigger_supported: true,
        })
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
    async fn wallet_balance(
        &self,
        _params: WalletBalanceParams,
    ) -> Result<results::WalletBalanceResult, ControlError> {
        Ok(results::WalletBalanceResult {
            balance: 1234,
            pending: 0,
            source: Some(results::WalletReadSource::Db),
            synced: true,
            peak_height: Some(5_000_000),
        })
    }
    /// Echoes the REQUEST into the coin so a mis-routed dispatch cannot look like a hit: the coin
    /// id carries the address and the amount carries the asset.
    async fn wallet_coins(
        &self,
        params: WalletCoinsParams,
    ) -> Result<results::WalletCoinsResult, ControlError> {
        Ok(results::WalletCoinsResult {
            coins: vec![results::WalletCoinRecord {
                coin_id: params.address,
                asset: Some(params.asset),
                amount: match params.asset {
                    Asset::Xch => 1,
                    Asset::Dig => 2,
                },
                parent_coin_info: "11".repeat(32),
                puzzle_hash: "22".repeat(32),
                created_height: Some(5_000_000),
                spent_height: None,
            }],
            source: Some(results::WalletReadSource::Db),
            synced: true,
            peak_height: Some(5_000_000),
        })
    }
    /// Knows exactly one coin — [`SPENT_COIN`] — and reports every other id as absent. Two ids, not
    /// one, so a handler that answered `coin: null` unconditionally (or echoed whatever it was
    /// asked) could not pass the routing test below.
    async fn wallet_coin_by_id(
        &self,
        params: WalletCoinByIdParams,
    ) -> Result<results::WalletCoinByIdResult, ControlError> {
        let coin = (params.coin_id == SPENT_COIN).then(|| results::WalletCoinRecord {
            coin_id: SPENT_COIN.into(),
            asset: None,
            amount: 1_000_000_000_000,
            parent_coin_info: "11".repeat(32),
            puzzle_hash: "22".repeat(32),
            created_height: Some(5_000_000),
            spent_height: Some(5_000_042),
        });
        Ok(results::WalletCoinByIdResult {
            coin,
            source: Some(results::WalletReadSource::Fallback),
            synced: false,
            peak_height: None,
        })
    }
    async fn wallet_peak(&self) -> Result<results::WalletPeakResult, ControlError> {
        Ok(results::WalletPeakResult {
            peak_height: Some(5_000_000),
            synced: true,
        })
    }
    /// Reports two DIFFERENT counts, so a handler that answered one number twice — or a dispatch
    /// arm wired to the wallet's sync status, whose chia count this deliberately matches — is
    /// distinguishable from a correct one by the DIG count alone.
    async fn peer_counts(&self) -> Result<results::PeerCountsResult, ControlError> {
        Ok(results::PeerCountsResult {
            dig_peer_count: Some(6),
            chia_peer_count: Some(3),
        })
    }
    /// Reports a phase and a height NEITHER neighbouring wallet read can produce — `syncing` beside
    /// a height the peak handler never returns — so a dispatch arm wired to `control.wallet.peak`
    /// (whose payload is otherwise a plausible sync status) fails the routing test below.
    async fn wallet_sync_status(&self) -> Result<results::WalletSyncStatusResult, ControlError> {
        Ok(results::WalletSyncStatusResult {
            phase: results::WalletSyncPhase::Syncing,
            peak_height: Some(4_999_000),
            chia_peer_count: Some(3),
        })
    }
    /// Accepts anything except [`REJECTED_BUNDLE`], which it refuses the way a mempool does — as a
    /// successful call reporting a refusal, never as an error.
    async fn wallet_broadcast(
        &self,
        params: WalletBroadcastParams,
    ) -> Result<results::WalletBroadcastResult, ControlError> {
        if params.signed_bundle_hex == REJECTED_BUNDLE {
            return Ok(results::WalletBroadcastResult {
                accepted: false,
                transaction_id: None,
                rejection: Some("DOUBLE_SPEND".into()),
            });
        }
        Ok(results::WalletBroadcastResult {
            accepted: true,
            transaction_id: Some("cc".repeat(32)),
            rejection: None,
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
    let req = build_request(
        RequestId::Number(1),
        &SetUpstreamParams {
            upstream: UNSUPPORTED_UPSTREAM.into(),
        },
    );
    let resp = block_on(node.dispatch(req));
    let err = resp.into_result().unwrap_err();
    assert_eq!(err.code_enum(), Some(ControlErrorCode::NotSupported));
}

#[test]
fn every_catalog_method_dispatches_without_panicking() {
    // The dispatcher must have an arm for EVERY catalog method, and each method's minimal params
    // fixture must be complete enough that the route succeeds rather than erroring.
    let node = MockNode;
    for &m in ControlMethod::ALL {
        let req = JsonRpcRequest::new(RequestId::Number(1), m.name(), minimal_params(m));
        let resp = block_on(node.dispatch(req));
        assert!(
            resp.error.is_none(),
            "{} errored under minimal_params: {:?}",
            m.name(),
            resp.error
        );
    }
}

/// **dig_ecosystem#2215** — the `software` member every `control.peerStatus` peer entry carries.
///
/// `control.peerStatus` is a PROXIED result: SPEC §4.1 forbids freezing a struct over the snapshot,
/// because its shape belongs to the node's peer pool. So this KAT pins the one member this contract
/// DOES own — `software` — in situ, inside a representative `connected` array, rather than pinning
/// the envelope around it.
///
/// The vector carries one REPORTED peer and one UNKNOWN peer together. A vector with only a
/// reported peer would pass against an implementation that omits `software` whenever it is Unknown,
/// which is precisely the bug the always-present rule exists to prevent.
#[test]
fn peer_status_software_member_golden_vector() {
    let snapshot = json!({
        "connected": [
            {
                "peer_id": "aa00",
                "address": "[2001:db8::1]:9444",
                "outbound": true,
                "software": {
                    "kind": "reported",
                    "product": "dig-node",
                    "version": "0.99.1",
                    "raw": "dig-node/0.99.1"
                }
            },
            {
                "peer_id": "bb11",
                "address": "[2001:db8::2]:9444",
                "outbound": false,
                "software": { "kind": "unknown" }
            }
        ]
    });

    let entries = snapshot["connected"].as_array().expect("connected array");

    // Always present: EVERY entry carries `software`, including the peer whose build is unknown.
    for entry in entries {
        assert!(
            entry.get("software").is_some(),
            "every peerStatus entry must carry `software`; omitting it is a serialization bug,              not an Unknown peer"
        );
    }

    // Each member decodes to the typed value and re-encodes byte-identically.
    for entry in entries {
        let wire = entry["software"].clone();
        let parsed: results::PeerSoftware =
            serde_json::from_value(wire.clone()).expect("software member must decode");
        assert_eq!(
            serde_json::to_value(&parsed).unwrap(),
            wire,
            "the software member is not byte-stable"
        );
    }

    // And the decoded values are the ones the vector names, so a decode that silently collapsed
    // both entries to the same value could not pass.
    let reported: results::PeerSoftware =
        serde_json::from_value(entries[0]["software"].clone()).unwrap();
    assert_eq!(reported, results::PeerSoftware::parse("dig-node/0.99.1"));
    let unknown: results::PeerSoftware =
        serde_json::from_value(entries[1]["software"].clone()).unwrap();
    assert_eq!(unknown, results::PeerSoftware::Unknown);
    assert_ne!(reported, unknown);
}

/// The legacy sentinel a peer is advertising RIGHT NOW must reach a reader as Unknown, not as a
/// version — the whole live fleet depends on this one mapping (dig_ecosystem#2215).
#[test]
fn a_legacy_peer_entry_reads_as_unknown_not_as_version_zero() {
    let software = results::PeerSoftware::parse("0.0.0");
    assert_eq!(software, results::PeerSoftware::Unknown);
    let wire = serde_json::to_value(&software).unwrap();
    assert_eq!(wire, json!({"kind": "unknown"}));
    assert_eq!(
        wire.to_string().find("0.0.0"),
        None,
        "no rendering of a legacy peer may contain the sentinel as a version"
    );
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
        ControlMethod::WalletBalance | ControlMethod::WalletCoins => {
            json!({"address": "xch1abc", "asset": "dig"})
        }
        ControlMethod::WalletCoinById => json!({ "coin_id": ABSENT_COIN }),
        ControlMethod::WalletBroadcast => json!({"signed_bundle_hex": "deadbeef"}),
        _ => json!({}),
    }
}

/// **The three new methods reach their own handlers, over the real dispatcher.**
///
/// Each assertion is keyed to something only THAT handler could have produced — the coin echoes the
/// address and asset it was asked for — so an arm wired to the neighbouring method fails here
/// rather than passing on a shape both happen to share.
#[test]
fn the_dispatcher_routes_each_wallet_chain_method_to_its_own_handler() {
    let coins = round_trip(&WalletCoinsParams {
        address: "xch1mintfunder".into(),
        asset: Asset::Dig,
    })
    .expect("coins must route");
    assert_eq!(coins.coins[0].coin_id, "xch1mintfunder");
    assert_eq!(
        coins.coins[0].asset,
        Some(Asset::Dig),
        "an address+asset read KNOWS the asset and must keep reporting it concretely --          `null` is reserved for a read that classified nothing"
    );
    assert_eq!(
        coins.coins[0].amount, 2,
        "the DIG amount proves the ASSET reached the handler, not just the address"
    );

    assert_eq!(
        round_trip(&WalletPeakParams {})
            .expect("peak must route")
            .peak_height,
        Some(5_000_000)
    );

    // Keyed to what only the sync-status handler produces: `control.wallet.peak` answers with a
    // DIFFERENT height and carries no phase at all, so an arm wired to it cannot land here.
    let sync = round_trip(&WalletSyncStatusParams {}).expect("syncStatus must route");
    assert_eq!(sync.phase, results::WalletSyncPhase::Syncing);
    assert_eq!(sync.peak_height, Some(4_999_000));
    assert_eq!(sync.chia_peer_count, Some(3));

    // `control.peerCounts` reaches its OWN handler: it is the only one that produces a DIG count at
    // all, so an arm wired to the sync status (whose chia count is the same 3) cannot land here.
    let counts = round_trip(&PeerCountsParams {}).expect("peerCounts must route");
    assert_eq!(counts.dig_peer_count, Some(6));
    assert_eq!(
        counts.chia_peer_count,
        Some(3),
        "the same observation control.wallet.syncStatus reports"
    );

    let pushed = round_trip(&WalletBroadcastParams {
        signed_bundle_hex: "deadbeef".into(),
    })
    .expect("broadcast must route");
    assert!(pushed.accepted);
    assert_eq!(pushed.rejection, None);
}

/// **`control.wallet.coinById` reaches its own handler, and reports a SPENT coin.**
///
/// The assertion is keyed to something only this handler could produce: a non-null `spent_height`.
/// `control.wallet.coins` answers by address with unspent coins only, so an arm mis-wired to it
/// could not report a spend height at all — which is the fact `mint_status` needs.
#[test]
fn the_dispatcher_routes_coin_by_id_to_its_own_handler() {
    let found = round_trip(&WalletCoinByIdParams {
        coin_id: SPENT_COIN.into(),
    })
    .expect("coinById must route");

    let coin = found.coin.expect("the mock knows this coin");
    assert_eq!(coin.coin_id, SPENT_COIN);
    assert_eq!(
        coin.spent_height,
        Some(5_000_042),
        "the spend height is the whole reason this method exists"
    );
    assert_eq!(
        coin.asset, None,
        "a by-id read classifies nothing; `null` says so rather than asserting a class"
    );
}

/// **An absent coin is an ANSWER, not an error.**
///
/// The nearest wrong implementation maps "no such coin" onto the error channel, where a caller
/// cannot tell it from an unreachable chain — and `mint_status` would then report a mint whose coin
/// genuinely does not exist as merely pending, forever, with the money already gone.
///
/// The fixture varies ONE thing against the found case above: which coin id is asked for. Both
/// answers come from the same handler, so a handler that could only ever answer one way fails here.
#[test]
fn an_absent_coin_is_a_result_not_an_error() {
    let absent = round_trip(&WalletCoinByIdParams {
        coin_id: ABSENT_COIN.into(),
    })
    .expect("an unknown coin must arrive on the Ok channel, never as a control error");

    assert_eq!(absent.coin, None);

    // And it is distinguishable, on the wire, from every error envelope: an error response carries
    // no `result` at all, so a client can never read one as "the chain says no such coin".
    let ok = JsonRpcResponse::success(RequestId::Number(1), serde_json::to_value(&absent).unwrap());
    assert!(ok.error.is_none(), "an absent coin is not an error");
    assert_eq!(ok.into_result().unwrap()["coin"], json!(null));

    for code in [
        ControlErrorCode::WalletNoChainSource,
        ControlErrorCode::WalletReadFailed,
        ControlErrorCode::WalletRateLimited,
    ] {
        let failed =
            JsonRpcResponse::error(RequestId::Number(1), ControlError::of(code, "unreachable"));
        assert!(
            parse_response::<WalletCoinByIdParams>(failed).is_err(),
            "{} must not decode into an absent-coin result",
            code.name()
        );
    }
}

/// A coin id that is not lowercase 64-hex is refused BEFORE any chain is consulted, as
/// `-32602 INVALID_PARAMS` — a malformed request, never a "no such coin" answer.
///
/// The `0x` prefix is the one tolerated decoration, because callers hand-copy ids out of block
/// explorers that print it. It is normalized away, never echoed.
#[test]
fn a_malformed_coin_id_is_invalid_params_not_an_absent_coin() {
    for bad in [
        "",
        "ab",                              // too short
        &"ab".repeat(33),                  // too long
        &"AB".repeat(32),                  // uppercase is not the wire form
        &format!("{}zz", "ab".repeat(31)), // non-hex
        &format!("0x{}", "ab".repeat(31)), // 0x-prefixed but short
        &format!(" {} ", "ab".repeat(32)), // padded
    ] {
        let node = MockNode;
        let req = JsonRpcRequest::new(
            RequestId::Number(1),
            ControlMethod::WalletCoinById.name(),
            json!({ "coin_id": bad }),
        );
        let err = block_on(node.dispatch(req)).into_result().unwrap_err();
        assert_eq!(
            err.code_enum(),
            Some(ControlErrorCode::InvalidParams),
            "{bad:?} must be refused as malformed params"
        );
    }

    // The tolerated decoration: `0x` + 64 hex reaches the handler as the unprefixed id.
    let node = MockNode;
    let req = JsonRpcRequest::new(
        RequestId::Number(1),
        ControlMethod::WalletCoinById.name(),
        json!({ "coin_id": format!("0x{SPENT_COIN}") }),
    );
    let found: results::WalletCoinByIdResult =
        serde_json::from_value(block_on(node.dispatch(req)).into_result().unwrap()).unwrap();
    assert_eq!(
        found
            .coin
            .expect("the 0x form names a coin the mock knows")
            .coin_id,
        SPENT_COIN,
        "the prefix is normalized away and never emitted"
    );
}

/// **A verdict must be stated, never inferred from silence.** `coin:null` says the chain holds no
/// such coin, so an ABSENT `coin` key must not decode into one. Serde's default treatment of
/// `Option` makes the two indistinguishable, which would let almost any JSON object — including
/// three sibling wallet results — decode into a confident "your coin does not exist".
#[test]
fn an_omitted_coin_field_is_a_decode_error_not_a_null_verdict() {
    // Explicit null still decodes: the verdict is expressible, just not by omission.
    let stated = serde_json::from_value::<results::WalletCoinByIdResult>(json!({
        "coin": null, "source": "fallback", "synced": false, "peak_height": null
    }))
    .expect("an explicitly null coin is a valid verdict");
    assert_eq!(stated.coin, None);

    // Omission does not. Each of these decoded to a complete "no such coin" before the field was
    // made required; the last three are other methods' result shapes.
    for wire in [
        json!({ "source": "fallback", "synced": false, "peak_height": null }),
        json!({ "synced": false }),
        json!({ "peak_height": 5_000_000u32, "synced": true }),
        json!({ "coins": [], "source": "db", "synced": true, "peak_height": 5_000_000u32 }),
    ] {
        assert!(
            serde_json::from_value::<results::WalletCoinByIdResult>(wire.clone()).is_err(),
            "a payload without a `coin` key must not decode into a no-such-coin verdict: {wire}"
        );
    }
}

#[test]
fn wallet_coin_by_id_params_refuse_malformed_ids_at_deserialization() {
    let bad_id = "AB".repeat(32);
    let upper = serde_json::from_value::<WalletCoinByIdParams>(json!({"coin_id": bad_id}));
    assert!(
        upper.is_err(),
        "uppercase ids must fail during deserialization"
    );
}

/// **A mempool refusal is a successful call.** The nearest wrong implementation maps a refusal onto
/// the error channel, where it becomes indistinguishable from a network that could not be reached —
/// and the two demand opposite remedies (retry this bundle, versus build a different one).
///
/// The fixture varies ONE thing against the accepting case above: which bundle is pushed.
#[test]
fn a_mempool_refusal_arrives_as_a_value_not_as_an_error() {
    let outcome = round_trip(&WalletBroadcastParams {
        signed_bundle_hex: REJECTED_BUNDLE.into(),
    })
    .expect("a refusal must arrive on the Ok channel, never as a control error");

    assert!(!outcome.accepted);
    assert_eq!(outcome.rejection.as_deref(), Some("DOUBLE_SPEND"));
    assert_eq!(
        outcome.transaction_id, None,
        "a refused bundle has no transaction to report"
    );
}

/// **dig-app's frozen shapes read our results losslessly.** These local structs are transcribed
/// from `dig-app-core::wallet::engine` — the consumer's own types, which this crate cannot depend
/// on — so the superset relationship is proven here rather than assumed at adoption time.
///
/// Written as SEPARATE structs, not as a field-name string comparison: a name check would pass on a
/// field whose TYPE had drifted, and `amount: u64` versus a string is exactly the drift a
/// hand-copied contract produces.
#[test]
fn dig_apps_frozen_engine_shapes_deserialize_our_wallet_results() {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct AppCoinRecord {
        coin_id: String,
        asset: Asset,
        amount: u64,
    }
    #[derive(Debug, Deserialize)]
    struct AppCoinsResponse {
        coins: Vec<AppCoinRecord>,
    }
    #[derive(Debug, Deserialize)]
    struct AppBroadcastResponse {
        accepted: bool,
        #[serde(default)]
        transaction_id: Option<String>,
    }

    let coins = serde_json::to_value(results::WalletCoinsResult {
        coins: vec![results::WalletCoinRecord {
            coin_id: "aa".repeat(32),
            asset: Some(Asset::Dig),
            amount: 2_000_000_000_000,
            parent_coin_info: "bb".repeat(32),
            puzzle_hash: "cc".repeat(32),
            created_height: Some(5_000_000),
            spent_height: None,
        }],
        source: Some(results::WalletReadSource::Db),
        synced: true,
        peak_height: Some(5_000_000),
    })
    .unwrap();
    let read: AppCoinsResponse =
        serde_json::from_value(coins).expect("dig-app must read our coins");
    assert_eq!(
        read.coins,
        vec![AppCoinRecord {
            coin_id: "aa".repeat(32),
            asset: Asset::Dig,
            amount: 2_000_000_000_000,
        }]
    );

    let pushed = serde_json::to_value(results::WalletBroadcastResult {
        accepted: true,
        transaction_id: Some("dd".repeat(32)),
        rejection: None,
    })
    .unwrap();
    let read: AppBroadcastResponse =
        serde_json::from_value(pushed).expect("dig-app must read our broadcast outcome");
    assert!(read.accepted);
    assert_eq!(read.transaction_id, Some("dd".repeat(32)));
}

#[test]
fn dig_apps_frozen_coin_shape_rejects_null_asset_in_wallet_coins() {
    use serde::Deserialize;

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct AppCoinRecord {
        coin_id: String,
        asset: Asset,
        amount: u64,
    }
    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct AppCoinsResponse {
        coins: Vec<AppCoinRecord>,
    }

    let coins = serde_json::to_value(results::WalletCoinsResult {
        coins: vec![results::WalletCoinRecord {
            coin_id: "aa".repeat(32),
            asset: None,
            amount: 2_000_000_000_000,
            parent_coin_info: "bb".repeat(32),
            puzzle_hash: "cc".repeat(32),
            created_height: Some(5_000_000),
            spent_height: None,
        }],
        source: Some(results::WalletReadSource::Db),
        synced: true,
        peak_height: Some(5_000_000),
    })
    .unwrap();
    assert!(
        serde_json::from_value::<AppCoinsResponse>(coins).is_err(),
        "dig-app's frozen coin shape requires a non-null asset in control.wallet.coins"
    );
}

/// The wallet failure codes are the ones dig-node already emits, pinned by NUMBER and SYMBOL.
///
/// They live here so a client branches on the contract's constant instead of retyping the literal —
/// the drift dig-app's own review caught itself doing (dig-app#109), and which no fixture could
/// have caught, because the fixture would have retyped the same literal.
#[test]
fn the_wallet_failure_codes_match_the_nodes_catalogue() {
    for (code, number, symbol) in [
        (
            ControlErrorCode::WalletNoChainSource,
            -32040,
            "WALLET_NO_CHAIN_SOURCE",
        ),
        (
            ControlErrorCode::WalletNotSynced,
            -32041,
            "WALLET_NOT_SYNCED",
        ),
        (
            ControlErrorCode::WalletReadFailed,
            -32042,
            "WALLET_READ_FAILED",
        ),
        (
            ControlErrorCode::WalletRateLimited,
            -32043,
            "WALLET_RATE_LIMITED",
        ),
    ] {
        assert_eq!(code.code(), number, "{symbol} number drifted");
        assert_eq!(code.name(), symbol);
        assert_eq!(ControlErrorCode::from_code(number), Some(code));
        assert_eq!(
            code.origin(),
            "node",
            "a wallet read is served by the node's backend, not the shell"
        );
    }
}
