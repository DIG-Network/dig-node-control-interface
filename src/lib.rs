//! # dig-node-control-interface — the canonical client ⇄ dig-node CONTROL interface contract
//!
//! This crate is the single source of truth for the management/query surface a client — the CLI
//! `dign`, the browser extension, dig-app, or hub — uses to **control and query a running dig-node**:
//! node configuration, cache configuration, peer status, subscription lifecycle, peer connect/
//! disconnect, and other node-control operations. Both the node (the server side, dispatching these
//! calls) and every client (the callers) depend on THIS crate rather than each maintaining a
//! byte-identical copy of the method catalog, so the two can never silently drift.
//!
//! ## Boundary (the 6th directed-boundary contract crate)
//!
//! The DIG ecosystem draws its cross-process contracts as small, single-purpose crates rather than
//! folding every wire shape into the systems that use them:
//!
//! - **dig-rpc-protocol** — node ⇄ node peer wire (PublicRead + Peer tiers).
//! - **dig-ipc-protocol** — app ⇄ node local session/signing envelope (the pipe/transport + the
//!   challenge/callback handshake a client authenticates over).
//! - **dig-node-control-interface (this crate)** — the CONTROL METHOD CATALOG a client sends *inside*
//!   that authenticated channel (or over loopback-mTLS + a signed control token, CLAUDE.md §5.3, for
//!   clients that don't ride the local IPC session): the method names, parameter/result types, and
//!   error taxonomy for config/status/peers/subscriptions/cache/wallet control operations.
//!
//! This crate is deliberately **transport-agnostic** — it describes WHAT a client can ask a node to do
//! and what the node replies, not HOW the bytes travel. Consumers pick the transport (dig-ipc-protocol
//! session, loopback HTTP/WebSocket, or a future channel) and carry these types over it.
//!
//! ## Status: skeleton (epic #1110, T1 of the cascade)
//!
//! This is the PROVISIONING commit: the repository, CI/release plumbing, and module layout exist so
//! the contract can be designed and filled in without also bootstrapping infrastructure. The actual
//! method catalog — request/response types, the control-error taxonomy, and the conformance KATs —
//! lands in T2 (tracked as dig_ecosystem#1147). Until then [`control`] is an intentionally empty
//! placeholder module reserved for that catalog.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Reserved for the client↔node control method catalog (request/response types, error taxonomy).
///
/// Populated by epic #1110 T2 (dig_ecosystem#1147) — the reconciliation decision on migrating the
/// `Control` tier out of dig-rpc-protocol, plus the exact method/type/error set this crate owns.
pub mod control {
    /// The crate's semantic version, exposed so consumers can assert compatibility at runtime
    /// without re-parsing `Cargo.toml` (mirrors the pattern used by the sibling contract crates).
    pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
}

#[cfg(test)]
mod tests {
    use super::control;

    /// The skeleton's only behaviour today: the crate reports its own version. This is a canary
    /// that the crate builds, links, and exposes `control` as expected — the real conformance KATs
    /// arrive with the method catalog in T2.
    #[test]
    fn crate_version_is_exposed_and_matches_cargo_toml() {
        assert_eq!(control::CRATE_VERSION, "0.1.0");
    }
}
