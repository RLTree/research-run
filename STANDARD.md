# Research Run Standard

## Authority and claim ceiling

Canonical, versioned JSON records under `.research-run/` are the only product
authority. Human and JSON status are deterministic projections. Evidence links
describe relationships but do not promote claims. Only an explicit human review
decision may set a claim assessment to `supported`, `limited`, `contradicted`, or
`unsupported`. `supported` means reviewed support within this workspace and
scope—not general scientific truth.

## Current stack

Research Run v0.1 is a correctness-critical Rust CLI on the pinned stable 1.95.0
toolchain. Cargo is the build, test, package, and distribution substrate. The
runtime dependency budget is deliberately small:

- `clap`: typed public CLI parsing and generated help;
- `serde`: typed domain serialization and deserialization;
- `serde_json`: deterministic, human-readable canonical JSON.

No async runtime, database, network client, logging/telemetry stack, temporary
file crate, schema framework, or UI dependency is justified by the v0.1 job.
Standard-library filesystem primitives provide bounded reads and atomic same-
directory publication.

## Product and security invariants

- Parse before effects; reject unknown fields and versions.
- Stable IDs are lowercase ASCII slugs and filenames must match IDs.
- Reject absolute workspace pointers, parent traversal, and symlinks in workspace
  authority paths. Raw data is referenced, not copied.
- Publication uses a new same-directory temporary file, flush and `sync_all`, an
  atomic no-clobber hard link, cleanup, then directory sync. Identical retry is a
  no-op; conflicting identity fails closed. Startup/validation removes no
  ambiguous state automatically; explicit `recover` validates pending content
  before completing or reconciling an interrupted publication.
- Each record is at most 1 MiB; canonical collections are bounded to 10,000
  records per kind; user text is bounded; status is derived in deterministic ID
  order. These are safety and memory/storage budgets, not scale claims.
- Diagnostics name record paths and invariant failures but never echo record
  bodies, environment variables, or secrets.

## Required gates

Run from the repository root:

```console
scripts/check fast
scripts/check full
scripts/check coverage
cargo audit
cargo deny check
gitleaks detect --source . --no-banner --redact
```

`scripts/check fast` is the inner loop. `scripts/check full` is the broad source
and package gate and runs only when a source claim can move. `scripts/check
coverage` requires `cargo-llvm-cov` and enforces 100% line coverage for an exact
material source-coverage claim; the present candidate does not meet that bar, so
that claim and full fitted-governance activation remain withheld. `cargo audit`,
`cargo deny`, and `gitleaks` are stronger local gates when available. CI runs the
deterministic Cargo gates and a repository secret scan. Packaging, install smoke,
the public CLI journey, GitHub branch/PR state, release, and real-user use remain
separate proof surfaces.

## Audit cadence

The review owner classifies the decision boundary before review. Signoff binds
to a clean commit SHA unless an immutable artifact is named explicitly; a dirty
worktree cannot satisfy it.

| Trigger | Owner | Exact candidate identity | Proof surface | Claims withheld until pass |
| --- | --- | --- | --- | --- |
| Critical protected boundary changes or fails | Implementer plus required independent boundary reviewer | Clean `HEAD` and failing input or fixture digest | Named security, authority, recovery, or data-loss boundary | Every claim depending on that boundary |
| Claim-bearing material integration, product, release, or completion boundary | Implementer; full four-persona team only here | Clean `HEAD`, lockfile digest, and current claim ceiling | Complete affected source and required package/install/runtime/journey evidence | Completion and every unproven downstream claim |
| Bounded reversible change | Implementer; at most one targeted reviewer | Clean `HEAD` or named staged-diff digest | Focused affected tests and deterministic fast gates | Only the changed behavior claim |
| Non-material docs or mechanical change | Owner | Clean `HEAD` or exact diff | Freshness/check surface | Only affected documentation or mechanical claim |
| UltraGoal repository fit | Repository owner | Research Run clean `HEAD`, UltraGoal source commit, and plan digest | Source fit inspect/plan; verify only after safe accepted apply | Install, discovery, runtime-active, and full fitted-governance claims |

Material boundary cadence is one exhaustive independent issue-set review, one
coherent repair, and one final exact-candidate signoff attempt. A material final
failure does not start Round 3: redesign, split or narrow the scope, withhold the
claim, or defer only demonstrably nonblocking debt. Bounded reversible work uses
focused checks and at most one targeted review. Non-material work uses owner
review and deterministic checks only. Broad review is not repeated after each
source increment.

The recovery repair is exactly commit
`9e1d411b670eec4d2073cc775d8bf5081f841154`, which passed bounded final Round 2
signoff. Its UltraGoal 0.0.12 fit plan was `conflicting`: 70 proposed mutations,
one conflict at Research Run-owned `AGENTS.md`, digest
`sha256:a9fa6fbc512b6f27abfcc133f9ee4d4a4a87c5ffb37271c1179fcef0a40a7d36`.
It was not accepted or applied. Source authority is manifest 0.0.12 at commit
`69787f20adcf0b99c7f3a26f71b35a215fda28d6`; the observed installed cache is
0.0.11. Building the exact source commit independently failed under its own
warnings-as-errors policy, so local reproduction of the source CLI and full fit
verification are unavailable. These facts support source-guided adaptation only;
installed, discovered, runtime-active, and full fitted-governance claims remain
withheld.
