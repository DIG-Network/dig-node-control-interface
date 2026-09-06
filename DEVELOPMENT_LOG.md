
## 2026-09-06 — mirror.reconcile result shape correction (in progress)

Opening the lane for dig-node-control-interface#54: `0.34.0`'s `MirrorReconcileResult::Completed`/
`Partial` declare outcomes the node cannot honestly report synchronously (reclaim and create are
separate spend bundles; a create's funding is a scan of CONFIRMED coins, so a reclaim's collateral
becomes usable only in a LATER pass). See `dig_ecosystem`'s
`.claude/loop/specs/3203-mirror-reconcile/control-interface.RECOMMENDATIONS.md` for the full brief.
