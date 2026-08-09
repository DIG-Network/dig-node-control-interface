//! The two contract traits: the client-facing call builder/parser and the node-facing handler.
//!
//! * [`ControlCall`] binds a typed params struct to its [`ControlMethod`] and its typed result — so
//!   a caller writes `client.request(&SetCapParams { cap_bytes })` and gets back a `SetCapResult`,
//!   never a stringly-typed `Value`.
//! * [`ControlClient`] is what a CLIENT depends on: build a JSON-RPC request from a typed call, and
//!   parse a response back into the typed result (or a [`ControlError`]). Pure — no transport; the
//!   consumer carries the bytes over dig-ipc / loopback-mTLS itself.
//! * [`ControlHandler`] is what a NODE implements to SERVE the surface: one typed method per control
//!   method, plus a provided [`dispatch`](ControlHandler::dispatch) that routes a raw request to the
//!   right method — the single anti-drift seam the conformance KATs exercise.

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::envelope::{JsonRpcRequest, JsonRpcResponse, RequestId};
use crate::error::{ControlError, ControlErrorCode};
use crate::method::ControlMethod;
use crate::params;
use crate::results;

/// A typed control call: a params struct that knows its [`ControlMethod`] and its result type.
///
/// Implemented by every struct in [`crate::params`]; this is what makes [`ControlClient::request`]
/// return the right typed result for each method at compile time.
pub trait ControlCall: Serialize {
    /// The wire method this call invokes.
    const METHOD: ControlMethod;
    /// The typed result this call returns on success.
    type Output: DeserializeOwned;
}

/// Serialize a typed call's params into a JSON object (`{}` for a no-param call, never `null`).
fn params_value<C: ControlCall>(call: &C) -> Value {
    match serde_json::to_value(call) {
        Ok(Value::Null) => Value::Object(Default::default()),
        Ok(v) => v,
        Err(_) => Value::Object(Default::default()),
    }
}

/// Build the JSON-RPC request envelope for a typed control call. Pure.
pub fn build_request<C: ControlCall>(id: RequestId, call: &C) -> JsonRpcRequest {
    JsonRpcRequest::new(id, C::METHOD.name(), params_value(call))
}

/// Parse a JSON-RPC response into a typed result, or the [`ControlError`] it carried. Pure.
pub fn parse_response<C: ControlCall>(
    response: JsonRpcResponse,
) -> Result<C::Output, ControlError> {
    let value = response.into_result()?;
    serde_json::from_value(value).map_err(|e| {
        ControlError::of(
            ControlErrorCode::ControlError,
            format!("failed to parse {} result: {e}", C::METHOD.name()),
        )
    })
}

/// The client-facing half of the contract: turn typed calls into requests and responses back into
/// typed results.
///
/// The default implementations cover every client; a consumer implements this trait only to
/// customise request construction (e.g. attaching the control token in a bespoke way). The blanket
/// [`DefaultControlClient`] gives callers the standard behaviour for free.
pub trait ControlClient {
    /// Build the request envelope for a typed call with the given request `id`.
    fn build_request<C: ControlCall>(&self, id: RequestId, call: &C) -> JsonRpcRequest {
        build_request(id, call)
    }

    /// Parse a response envelope into the typed result for call type `C`.
    fn parse_response<C: ControlCall>(
        &self,
        response: JsonRpcResponse,
    ) -> Result<C::Output, ControlError> {
        parse_response::<C>(response)
    }
}

/// The standard, zero-configuration [`ControlClient`] using the default request/response behaviour.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultControlClient;

impl ControlClient for DefaultControlClient {}

/// The node-facing half of the contract: a running node implements this to SERVE the control
/// surface. Each method is typed to the catalog's params/results; the provided
/// [`dispatch`](ControlHandler::dispatch) routes a raw [`JsonRpcRequest`] to the right method so a
/// server needs only one entry point and can never mis-route.
///
/// Open/proxied shapes (the updater beacon status, the pairing list, the peer-pool snapshot) return
/// [`Value`] rather than a frozen struct, matching the catalog's [`ControlCall::Output`] for those
/// methods.
#[async_trait]
pub trait ControlHandler: Sync {
    /// `control.status`
    async fn status(&self) -> Result<results::StatusResult, ControlError>;
    /// `control.config.get`
    async fn config_get(&self) -> Result<results::ConfigResult, ControlError>;
    /// `control.config.setUpstream`
    async fn config_set_upstream(
        &self,
        params: params::SetUpstreamParams,
    ) -> Result<results::SetUpstreamResult, ControlError>;
    /// `control.log.setLevel`
    async fn log_set_level(
        &self,
        params: params::SetLevelParams,
    ) -> Result<results::SetLevelResult, ControlError>;
    /// `control.cache.get`
    async fn cache_get(&self) -> Result<results::CacheView, ControlError>;
    /// `control.cache.setCap`
    async fn cache_set_cap(
        &self,
        params: params::SetCapParams,
    ) -> Result<results::SetCapResult, ControlError>;
    /// `control.cache.clear`
    async fn cache_clear(&self) -> Result<results::CacheClearResult, ControlError>;
    /// `control.hostedStores.list`
    async fn hosted_stores_list(&self) -> Result<results::HostedStoresListResult, ControlError>;
    /// `control.hostedStores.pin`
    async fn hosted_stores_pin(
        &self,
        params: params::PinParams,
    ) -> Result<results::PinResult, ControlError>;
    /// `control.hostedStores.unpin`
    async fn hosted_stores_unpin(
        &self,
        params: params::UnpinParams,
    ) -> Result<results::UnpinResult, ControlError>;
    /// `control.hostedStores.status`
    async fn hosted_stores_status(
        &self,
        params: params::HostedStoreStatusParams,
    ) -> Result<results::HostedStoreStatusResult, ControlError>;
    /// `control.sync.status`
    async fn sync_status(&self) -> Result<results::SyncStatusResult, ControlError>;
    /// `control.sync.trigger`
    async fn sync_trigger(
        &self,
        params: params::SyncTriggerParams,
    ) -> Result<results::SyncTriggerResult, ControlError>;
    /// `control.updater.status`
    async fn updater_status(&self) -> Result<Value, ControlError>;
    /// `control.updater.setChannel`
    async fn updater_set_channel(
        &self,
        params: params::SetChannelParams,
    ) -> Result<Value, ControlError>;
    /// `control.updater.pause`
    async fn updater_pause(&self, params: params::PauseParams) -> Result<Value, ControlError>;
    /// `control.updater.resume`
    async fn updater_resume(&self) -> Result<Value, ControlError>;
    /// `control.updater.checkNow`
    async fn updater_check_now(&self) -> Result<Value, ControlError>;
    /// `control.pairing.list`
    async fn pairing_list(&self) -> Result<Value, ControlError>;
    /// `control.pairing.approve`
    async fn pairing_approve(
        &self,
        params: params::ApproveParams,
    ) -> Result<results::PairingApproveResult, ControlError>;
    /// `control.pairing.revoke`
    async fn pairing_revoke(
        &self,
        params: params::RevokeParams,
    ) -> Result<results::PairingRevokeResult, ControlError>;
    /// `control.peerStatus`
    async fn peer_status(&self) -> Result<Value, ControlError>;
    /// `control.peers.connect`
    async fn peers_connect(
        &self,
        params: params::PeersConnectParams,
    ) -> Result<results::PeersConnectResult, ControlError>;
    /// `control.peers.disconnect`
    async fn peers_disconnect(
        &self,
        params: params::PeersDisconnectParams,
    ) -> Result<results::PeersDisconnectResult, ControlError>;
    /// `control.subscribe`
    async fn subscribe(
        &self,
        params: params::SubscribeParams,
    ) -> Result<results::SubscribeResult, ControlError>;
    /// `control.unsubscribe`
    async fn unsubscribe(
        &self,
        params: params::UnsubscribeParams,
    ) -> Result<results::UnsubscribeResult, ControlError>;
    /// `control.listSubscriptions`
    async fn list_subscriptions(&self) -> Result<results::ListSubscriptionsResult, ControlError>;
    /// `control.wallet.balance` (READ-only)
    async fn wallet_balance(
        &self,
        params: params::WalletBalanceParams,
    ) -> Result<results::WalletBalanceResult, ControlError>;
    /// `control.wallet.coins` (READ-only, OPEN)
    ///
    /// An empty `coins` list MUST mean "a chain was consulted and this address holds nothing".
    /// A read that could not consult a chain MUST return the matching catalogued error instead.
    async fn wallet_coins(
        &self,
        params: params::WalletCoinsParams,
    ) -> Result<results::WalletCoinsResult, ControlError>;
    /// `control.wallet.coinById` (READ-only, OPEN)
    ///
    /// `Ok(coin: None)` MUST mean "a chain was consulted and holds no such coin". A read that could
    /// not consult a chain MUST return the matching catalogued error instead — a caller that cannot
    /// tell those apart reports a spent mint as pending forever.
    ///
    /// The params are validated at DESERIALIZATION (lowercase 64-hex, `0x` stripped), so any path
    /// that decodes `WalletCoinByIdParams` refuses malformed ids as `INVALID_PARAMS` before this
    /// method is called.
    async fn wallet_coin_by_id(
        &self,
        params: params::WalletCoinByIdParams,
    ) -> Result<results::WalletCoinByIdResult, ControlError>;
    /// `control.wallet.peak` (READ-only, OPEN)
    async fn wallet_peak(&self) -> Result<results::WalletPeakResult, ControlError>;
    /// `control.wallet.broadcast` (TOKEN-GATED)
    ///
    /// Pushes an ALREADY-SIGNED bundle: the implementation never signs, and never receives anything
    /// it could sign with (§908). A mempool refusal is `Ok` with `accepted: false`; failing to
    /// reach a mempool is `Err`.
    async fn wallet_broadcast(
        &self,
        params: params::WalletBroadcastParams,
    ) -> Result<results::WalletBroadcastResult, ControlError>;
    /// `pairing.request` (OPEN)
    async fn pairing_request(
        &self,
        params: params::RequestParams,
    ) -> Result<results::PairingRequestResult, ControlError>;
    /// `pairing.poll` (OPEN)
    async fn pairing_poll(
        &self,
        params: params::PollParams,
    ) -> Result<results::PairingPollResult, ControlError>;

    /// Route a raw JSON-RPC request to the right typed method and build the response envelope.
    ///
    /// Deserializes the params for methods that take them, calls the handler, and serializes the
    /// typed result. An unknown method → `METHOD_NOT_FOUND`; malformed params → `INVALID_PARAMS`.
    /// This is the single seam a server dispatches through — the KATs exercise it end-to-end.
    async fn dispatch(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        let Some(method) = ControlMethod::from_name(&request.method) else {
            return JsonRpcResponse::error(
                id,
                ControlError::of(
                    ControlErrorCode::MethodNotFound,
                    format!("unknown control method: {}", request.method),
                ),
            );
        };
        match self.dispatch_method(method, request.params).await {
            Ok(result) => JsonRpcResponse::success(id, result),
            Err(err) => JsonRpcResponse::error(id, err),
        }
    }

    /// Route to the typed method by [`ControlMethod`], returning the result as a [`Value`]. Split
    /// from [`dispatch`](ControlHandler::dispatch) so the envelope wrapping stays in one place.
    #[doc(hidden)]
    async fn dispatch_method(
        &self,
        method: ControlMethod,
        params: Value,
    ) -> Result<Value, ControlError> {
        /// Deserialize a method's params, mapping a shape error to `INVALID_PARAMS`.
        fn decode<T: DeserializeOwned>(params: Value) -> Result<T, ControlError> {
            serde_json::from_value(params)
                .map_err(|e| ControlError::of(ControlErrorCode::InvalidParams, e.to_string()))
        }
        /// Serialize a typed result to a `Value` (infallible for our derive-Serialize results).
        fn encode<T: Serialize>(value: T) -> Result<Value, ControlError> {
            serde_json::to_value(value)
                .map_err(|e| ControlError::of(ControlErrorCode::ControlError, e.to_string()))
        }
        match method {
            ControlMethod::Status => encode(self.status().await?),
            ControlMethod::ConfigGet => encode(self.config_get().await?),
            ControlMethod::ConfigSetUpstream => {
                encode(self.config_set_upstream(decode(params)?).await?)
            }
            ControlMethod::LogSetLevel => encode(self.log_set_level(decode(params)?).await?),
            ControlMethod::CacheGet => encode(self.cache_get().await?),
            ControlMethod::CacheSetCap => encode(self.cache_set_cap(decode(params)?).await?),
            ControlMethod::CacheClear => encode(self.cache_clear().await?),
            ControlMethod::HostedStoresList => encode(self.hosted_stores_list().await?),
            ControlMethod::HostedStoresPin => {
                encode(self.hosted_stores_pin(decode(params)?).await?)
            }
            ControlMethod::HostedStoresUnpin => {
                encode(self.hosted_stores_unpin(decode(params)?).await?)
            }
            ControlMethod::HostedStoresStatus => {
                encode(self.hosted_stores_status(decode(params)?).await?)
            }
            ControlMethod::SyncStatus => encode(self.sync_status().await?),
            ControlMethod::SyncTrigger => encode(self.sync_trigger(decode(params)?).await?),
            ControlMethod::UpdaterStatus => self.updater_status().await,
            ControlMethod::UpdaterSetChannel => self.updater_set_channel(decode(params)?).await,
            ControlMethod::UpdaterPause => self.updater_pause(decode(params)?).await,
            ControlMethod::UpdaterResume => self.updater_resume().await,
            ControlMethod::UpdaterCheckNow => self.updater_check_now().await,
            ControlMethod::PairingList => self.pairing_list().await,
            ControlMethod::PairingApprove => encode(self.pairing_approve(decode(params)?).await?),
            ControlMethod::PairingRevoke => encode(self.pairing_revoke(decode(params)?).await?),
            ControlMethod::PeerStatus => self.peer_status().await,
            ControlMethod::PeersConnect => encode(self.peers_connect(decode(params)?).await?),
            ControlMethod::PeersDisconnect => encode(self.peers_disconnect(decode(params)?).await?),
            ControlMethod::Subscribe => encode(self.subscribe(decode(params)?).await?),
            ControlMethod::Unsubscribe => encode(self.unsubscribe(decode(params)?).await?),
            ControlMethod::ListSubscriptions => encode(self.list_subscriptions().await?),
            ControlMethod::WalletBalance => encode(self.wallet_balance(decode(params)?).await?),
            ControlMethod::WalletCoins => encode(self.wallet_coins(decode(params)?).await?),
            // Re-validated here idempotently; deserialization already enforced the same rule.
            ControlMethod::WalletCoinById => {
                let params: params::WalletCoinByIdParams = decode(params)?;
                encode(self.wallet_coin_by_id(params.validated()?).await?)
            }
            ControlMethod::WalletPeak => encode(self.wallet_peak().await?),
            ControlMethod::WalletBroadcast => encode(self.wallet_broadcast(decode(params)?).await?),
            ControlMethod::PairingRequest => encode(self.pairing_request(decode(params)?).await?),
            ControlMethod::PairingPoll => encode(self.pairing_poll(decode(params)?).await?),
        }
    }
}
