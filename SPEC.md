# dig-node-control-interface — normative specification

This document is the authoritative contract for the client ⇄ dig-node **control interface**: how a
client (the CLI `dign`, the browser extension, dig-app, hub) controls and queries a running dig-node.
An independent reimplementation of either side MUST conform to this specification byte-for-byte. Where
this document and the code disagree, the conformance KATs in `tests/` (once populated, T2) are the
tie-breaker.

Layering: this file is the repo's own contract; the cross-repo interaction map is the superproject
`SYSTEM.md`; the session/transport envelope this catalog rides over is `dig-ipc-protocol`'s `SPEC.md`;
the node↔node peer wire is `dig-rpc-protocol`'s `SPEC.md`. All MUST agree.

> **Status: skeleton.** This is the epic #1110 T1 provisioning commit — the repository, CI, and module
> layout only. The normative method catalog below is filled in by T2 (dig_ecosystem#1147). Sections
> marked TBD are reserved headings, not yet a contract.

## 1. Scope and boundary

`dig-node-control-interface` owns the **client↔node CONTROL method catalog**: the method names,
parameter/result types, and error taxonomy for controlling and querying a running dig-node —
configuration, cache configuration, peer status, subscription lifecycle, peer connect/disconnect, and
other node-control operations.

It does **not** own:
- the node↔node peer wire (dig-rpc-protocol's PublicRead + Peer tiers), or
- the local session/signing handshake and transport a client authenticates over (dig-ipc-protocol).

This crate is transport-agnostic: the same method catalog is carried over the dig-ipc-protocol local
session, or over loopback-mTLS + a signed control token for clients reached via HTTP/WebSocket
(CLAUDE.md §5.3). Consumers select the transport; this crate defines only the payload contract that
rides over it.

## 2. Roles and trust boundary

- **Client:** a user-facing surface (CLI, extension, app, hub) that wants to control or query a
  running dig-node. Authenticates over whichever transport is in play before issuing control calls.
- **Node (server):** the identity-agnostic engine dispatching control calls against its live state
  (config, cache, peer table, subscriptions).

## 3. Method catalog (TBD — T2)

Reserved for the request/response method names, parameter/result types, and the associated error
taxonomy. Designed and landed by epic #1110 T2 (dig_ecosystem#1147), which also resolves whether the
existing `Control` tier in dig-rpc-protocol migrates here.

## 4. Error taxonomy (TBD — T2)

Reserved for the stable control-error codes clients key their UX off, analogous to dig-ipc-protocol's
`SignErrorCode`.

## 5. Conformance

Golden known-answer tests (KATs) proving the wire shapes are byte-exact across independent
implementations land alongside the method catalog in T2.
