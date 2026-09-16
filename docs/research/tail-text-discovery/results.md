# Retain the bounded tail-text discovery change

This report records the initial local experiment. The subsequently approved
[commit and package stage](package-stage.md) has a separate candidate and proof
route; the original measurements and holds below remain scoped to this stage.

The local implementation finds tail-only knowledge through both `search` and
`context --query`, preserves existing projection matches, returns literal bounded
Unicode excerpts, and keeps the existing handoff wire contract. The final
candidate passed all provisional performance review triggers on this Mac.

## Candidate and review material

- Base: `71ad5fd8f0533a7ab34b35ffeb3f86d88a92a2c5`; worktree remains detached.
- [Implementation patch](implementation.patch), SHA-256:
  `a0e77cac26611c268b166b90cb4f7696baa996399321ec49224a10307a9bfc73`.
  It covers production, tests, root README and architecture. Experiment progress
  and evidence files are separate from that patch identity.
- [Manifest](manifest.json) binds source files, evidence files, host and toolchain.
- [Full measurements](performance.md) and [method/reproduction](README.md).
- Final release binary SHA-256:
  `52faf6e23d76fe5399583e860e526a7d9b452184dc845279176b16e9a2074f7c`.
  Both executables remain under `target/tail-text-evidence/` for local review.

## Passed surfaces

- Synthetic baseline red/candidate green through both real commands.
- Literal excerpts within 512 Unicode scalars, including ellipses; U+0130
  expansion, contextual sigma with 600 combining marks, emoji, combining marks,
  maximum body/query sizes, boundary and first-occurrence cases.
- Existing ID/title/prefix reasons and summaries, kind-disjoint shared IDs,
  history flags, deterministic counts/order/limits, unqueried projections,
  rejection limits and terminal escaping.
- Actual handoff create/inspect/validate, baseline executable accepting the new
  v2 tail handoff, and candidate accepting baseline v2 plus synthetic legacy v1.
  Canonical byte preservation passed for synthetic and read-only live queries.
- One targeted independent review and its bounded repair follow-up found no
  remaining material algorithm, excerpt or compatibility issue. The subsequent
  module nesting correction was verified by compile and semantic checks.
- Formatting, all-target compile, strict clippy, repository authority,
  standards, all **358 normal tests**, and the doctest command (zero doctests).
- The settled coverage run passed 282 unit/coverage tests, 138 product tests and
  two standards tests. All **1,027 measured regions, 468 lines and 78 functions**
  across the four changed production files executed.

## Final local performance

All 24 command/query/workload cases stayed below the provisional triggers.
Each case has three warmup pairs and 15 alternating measured pairs; p95 is the
nearest-rank maximum of those 15 observations. Initial observations and raw
durations are retained. Filesystem caches were not flushed or assumed cold.

The refreshed helper evidence records qualification metadata. Its many-short
tail control is explicitly `NOT_APPLICABLE` because baseline results saturate
the bounded result set, so absence cannot be proven. The near-budget tail
control remains qualified. The table below preserves the earlier experiment's
historical timing scope rather than re-labeling it as the final helper run.

| Tail search workload | Matches, baseline → candidate | Median, baseline → candidate | p95, baseline → candidate |
| --- | ---: | ---: | ---: |
| Read-only current workspace, 55 knowledge records | 0 → 1 | 11.98 → 12.17 ms | 12.63 → 13.56 ms |
| Near-budget synthetic, 1,000 knowledge records | 0 → 1,000 | 73.07 → 83.96 ms | 79.24 → 92.38 ms |

The near-budget canonical snapshot is 65,745,856 bytes, about 98% of the 64-MiB
budget. Its tail context median moved from 72.40 to 85.22 ms. New results include
50 returned excerpts at the default limit, so these are useful-work comparisons,
not equal-output microbenchmarks. Near-budget tail-search peak RSS was 68.5 →
69.3 MiB, measured separately through a fresh parent's child resource usage.
The many-short workload has 1,000 records and 409,854 canonical bytes; the current
workspace has 502,510 canonical bytes. No private query text or scientific
content is included in retained measurement files.

An initial ASCII tail implementation crossed the trigger at about 200 ms; the
equivalent ASCII offset/slicing path reduced the cost. The retained change is
the optimized, subsequently rebuilt and remeasured candidate.

## Holds and next decision

The repository-wide 100% coverage gate remains **HOLD**: 511 of 14,597 regions,
156 of 7,577 lines and 48 of 955 functions are missed. Every miss is outside the
four changed production files. This identifies unchanged-source deficits; it is
not a fresh baseline-versus-candidate coverage-delta measurement. No unrelated
coverage repair or threshold change was made.

`scripts/check fast` rejects dirty candidates at entry. Its component checks
passed separately; this does not turn the aggregate gate green. Full aggregate,
clean-commit package/install, release and real-use fitness claims remain held.
The broad historical mutation campaign was not rerun. These measurements cover
one Mac, selected real-workspace queries and synthetic predominantly ASCII
workloads; they do not establish universal Unicode workload performance or
scientific validity.

**Recommendation: retain for source review.** The smallest next decision is
whether to authorize committing this bounded patch and proceeding to the
separate package/install checks under the repository's existing claim ceiling.
No commit, push, PR, installation, release, live canonical mutation, monitor
closure, settings change or automation change was performed.
