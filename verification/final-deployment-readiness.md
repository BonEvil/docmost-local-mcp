# Final deployment-readiness decision

Date: 2026-08-30

Decision: **READY FOR PR-ONLY REPOSITORY DELIVERY**

Deployment: **DISABLED**

## Verified facts

- Final-card HEAD `459c27f7a610d557995b2f4f02c271c2338760ec`
  contains source tree `aab54d519e0bf3f32c730f5c7cf1ee0e9f272153`,
  source-equivalent at remediation commits `e178c7d…` and `cbfea89`.
- `Cargo.lock` is unchanged at SHA-256
  `0db9682d4bf880bf7769e2565c8ec75b75f8d1a3820d482b0be5db3ec6374690`.
- Fresh Linux x86-64 binary
  `4237827500f5fd51db2ce86b767bf4aeb4cdb803a5ed28ccb2723237c6e4a90e`
  and Linux arm64 binary
  `f27cf1d7626f1889d77710059e60ea7995e048ad8980955fc2eac0af805f4b46`
  are bound to the exact corrected source and pinned build image.
- Format, warnings-denied Clippy, all-feature tests, and no-default-feature tests
  passed; fresh independent shell release-integrity and pinned-input checks pass.
- Docmost Community v0.95.0 compatibility exercised every read tool, default
  write denial, the exact narrow write inventory, identity rollover, expiry,
  401 recovery, scoped forget, loopback controls, and cleanup.
- Exact Atlas runtime `efad3719…` recorded ten approved read dispatches, one
  approved write dispatch, zero denied-write dispatches, no confirmation argument
  leakage, and zero remaining registrations.
- Direct and Atlas hostile matrices failed closed without private-value leakage.
- Isolated cleanup read back zero containers and zero volumes. Ordinary Docmost
  was not targeted, read, or modified.
- The independent F-01 through F-10 re-audit found no deployment-blocking issue.

## F-06 / IA-01 closure

Persisted sessions now carry an authenticated identity and are reusable only
when canonical origin and email exactly match active configuration. Legacy
unbound and mismatched sessions fail closed. The prior session is invalidated
before replacement config/session persistence. Deterministic failure injection
proves a failed session replacement leaves no identity-A token or remembered
credential that restart can reuse.

## Accepted residual risks

- Remember-password fails closed on the reviewed headless build unless the
  acknowledged encrypted-file fallback is enabled; adding a real credential
  backend requires dependency and advisory review.
- The exact unreachable rmcp advisory exception and advisory data must be
  refreshed for every candidate and release.
- JSON-RPC tool errors cause generic Atlas unavailability, reducing diagnostic
  quality without weakening fail-closed behavior.
- Synthetic content remains only inside the disposable Docmost containment
  boundary because the reviewed surface has no delete operation.
- Release safety depends on recorded provider controls remaining enforced; no
  real release has been authorized or exercised.
- A tailnet-only `:8443` proxy rule remains because disabling it requires
  interactive sudo. It contains no data, points to a closed port, and is outside
  the repository/runtime acceptance boundary.

## Remaining blockers

None for PR-only repository delivery of this exact candidate.

## Authority boundary

Repository delivery may proceed only to an **unmerged pull request targeting
`BonEvil/main`**. This decision does not authorize merge, tag creation, release,
installation, production access, production Atlas registration, or deployment.
Deployment remains disabled pending separate approval. Any source, lockfile,
dependency, workflow, or build-input change invalidates this decision and
requires the complete evidence chain and independent gate to be rerun.
