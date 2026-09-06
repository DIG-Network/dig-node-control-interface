
## 2026-09-06 — mirror.reconcile result shape correction

`0.34.0`'s `MirrorReconcileResult::Completed`/`Partial` declared outcomes the node cannot honestly
report synchronously: reclaim and create are separate spend bundles, and a create's funding is a scan
of CONFIRMED coins, so a reclaim's collateral becomes usable only in a LATER pass, never inside the
call that submitted it. `0.35.0` replaces both with one `Submitted` outcome carrying what the node
actually knows at return time — reclaims accepted/rejected and recreates OWED to the ordinary create
pass — and grows `MirrorReconcileRefusal` from 6 to 10 reasons so a wallet that cannot spend, or has
not yet measured its funds, is never mis-reported as `insufficient_funds` (dig-node SPEC §25.8's
conflation rule). Proven on a live host BEFORE this fix: two real creates landed in the audit record
as `Unresolved` ("the producer ended without recording an outcome") because a create's coin id cannot
be derived synchronously — the same fact this shape now states honestly instead of hiding. See
`dig_ecosystem`'s `.claude/loop/specs/3203-mirror-reconcile/control-interface.RECOMMENDATIONS.md` for
the full brief this correction was built from.
