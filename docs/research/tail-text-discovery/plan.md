# Bounded tail-text discovery experiment

Authority: RROPP-tail-text-discovery; isolated reversible local source work and
verification only. No commits, pushes, installation, release, or live record
changes. Historical release plans remain historical.

Base: `71ad5fd8f0533a7ab34b35ffeb3f86d88a92a2c5`, verified against remote
default main before mutation. Original worktree clean and detached.

Contract: shared same-snapshot matcher; preserve projection matches; fallback to
complete knowledge bodies; literal <=512-scalar excerpts; existing JSON reasons
and handoff versions. Query normalization, limits, order, validation and
confinement stay intact.

## Progress

- Baseline release built and preserved under `target/tail-text-evidence/baseline`.
- Synthetic prefix hit and tail miss reproduced through search and context.
- Shared matcher, Unicode source-span mapping, human reasons and CLI tests added.
- Focused regressions, both cross-reader directions and targeted review passed.
- Formatting, compile, strict clippy, standards, 358 normal tests and the
  doctest command passed (zero doctests are currently defined).
- The settled coverage run passed 282 unit/coverage tests, 138 product tests and
  two standards tests. All four changed production files have complete measured
  coverage. Repository totals remain red: 511/14597 regions, 156/7577 lines and
  48/955 functions missed, all outside those four files.
- An earlier coverage attempt stopped at the new router declaration exceeding
  the 250-line cap. The matcher now belongs under retrieval; the corrected
  semantic check passes. The final coverage run above is the settled result.
- Comparative release measurements found an ASCII mapping cost; the equivalent
  direct-offset path removed the provisional-trigger regression. Final binary
  compatibility and all 24 measurement cases passed; canonical bytes stayed
  unchanged. Peak memory was measured through fresh-parent child resource usage.
- Complete: [retain recommendation and evidence](results.md). The local source
  patch is `a0e77cac26611c268b166b90cb4f7696baa996399321ec49224a10307a9bfc73`.
  Stop here; a commit and package/install checks need the next explicit decision.

The `scripts/check fast` implementation requires clean commit custody even
though STANDARD.md describes dirty-source permission. Record its actual result
and run component checks without changing that gate; commits are unauthorized.
Full aggregate/package/install and exact repository-wide coverage claims remain
subject to their existing gates.
