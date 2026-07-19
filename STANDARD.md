# Research Run Standard

## Authority and claim ceiling

Canonical, versioned JSON records under `.research-run/` are the only product
authority. Human and JSON status are deterministic projections. Evidence links
describe relationships but do not promote claims. Only an explicit human review
decision may set a claim assessment to `supported`, `limited`, `contradicted`, or
`unsupported`. `supported` means reviewed support within this workspace and
scope—not general scientific truth.

Each review binds to the sorted immutable evidence IDs present for its claim at
the decision boundary. Later evidence makes that decision historical and returns
the claim to `unreviewed` until a new human review binds the changed graph.
`supported` requires at least one recorded evidence link; recovery and direct
record ingestion enforce the same binding as the CLI.

## Current stack

Research Run v0.1 is a correctness-critical Rust CLI on the pinned stable 1.97.1
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
  records per kind; one snapshot or recovery read is bounded to 64 MiB; user
  text is bounded; status is derived with indexed relationships in deterministic
  ID order. These are safety and memory/storage budgets, not scale claims.
- Mutating CLI commands take an operating-system file lock under the canonical
  state root. The lock releases on process exit and serializes supported CLI
  writers; uncooperative concurrent filesystem mutation is not a supported
  product mode and every read still rechecks opened-file identity.
- Diagnostics name record paths and invariant failures but never echo record
  bodies, environment variables, or secrets.
- Production and hand-authored test Rust files contain at most 250 noncomment
  lines, and functions at most 80 physical lines. Splits must own product behavior;
  forwarding shells and generic buckets do not satisfy the limit.

## Required gates

Run from the repository root:

```console
scripts/check fast
scripts/check full
scripts/check coverage
scripts/check dependencies
scripts/check mutations
scripts/check artifacts
```

`scripts/check fast` is the inner loop. `scripts/check full` is the broad clean-
candidate source, dependency, security, mutation, package, install, journey, and
observation gate and runs only when a claim can move. `scripts/check
coverage` requires `cargo-llvm-cov` and enforces 100% line, function, and region
coverage over the single production-shaped `product` integration target, with
all production files retained in the denominator. Unit tests run through
nextest and doctests run through Cargo as separate correctness surfaces.
`scripts/check-standards`
validates all 12 source-guidance modules plus contextual namespace red, green,
and tamper fixtures plus semantic-tree limits. `scripts/check dependencies`
requires zero duplicate dependency versions, current RustSec audit and Cargo Deny
passes, and a valid ephemeral CycloneDX inventory. `scripts/check mutations`
tests a bounded set covering typed parsing, claim promotion, path confinement,
locking, publication, recovery, and terminal neutralization; every viable mutant
must be killed and compiler-rejected mutants remain explicitly classified.
`scripts/check artifacts` requires a clean commit, builds and installs the Cargo
package, exercises the installed researcher journey, and emits machine-local
latency, size, install-footprint, and target-growth observations. There is no
cross-machine performance or regression-budget claim. CI and local gitleaks prove
their own secret-scan surfaces. Package, install, runtime, journey, GitHub,
release, and real-user evidence remain distinct.

### Tool contract

| Command | Owner and risk | Preconditions and effects | Failure and claim impact |
| --- | --- | --- | --- |
| `scripts/check fast` | Implementer; bounded local execution | Dirty source allowed; writes only reproducible Cargo output | Stops at first failed format, compile, focused test, or standards check; source claim stays withheld |
| `scripts/check coverage` | Implementer; bounded local execution | Pinned toolchain and `cargo-llvm-cov`; replaces ignored coverage target output | Any missed line, function, region, or production file withholds exact source coverage |
| `scripts/check dependencies` | Implementer; read-only network plus local cache/build metadata | Lockfile present; RustSec may refresh its public advisory cache; SBOM is ephemeral and deleted | Duplication, advisory, license/source policy, or SBOM failure withholds dependency and package closure |
| `scripts/check mutations` | Implementer; bounded local execution | Pinned source and product tests; 60-second per-command limits; output remains ignored and reproducible | Missed, timed-out, or unresolved mutants withhold mutation and package-delta closure |
| `scripts/check artifacts` | Implementer; local package/install execution | Clean commit required; package, install root, journey workspace, and timing files are temporary | Any build, install, journey, or observation failure withholds its distinct package/install/runtime/journey/baseline claim |
| Local gitleaks and `actionlint` | Implementer; read-only local scan | Repository bytes only; findings are redacted | A leak or workflow error withholds local security or GitHub-workflow claims |

The commands are idempotent with respect to canonical product state. Generated
Cargo, mutation, package, inventory, and journey output is reproducible and
ignored or deleted; no command is authorized to merge, release, change repository
settings, or publish externally.

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

The additional validated source-guidance artifact is
`harness-ultragoal-governance-complete.zip`, SHA-256
`089382f27b64c6eb219b3a032296917dc8984129a52e56e63cb77ba66ba72377`.
Its independent manifest and validator classify 160 laws and 172 items with zero
issues. This portable identity does not establish UltraGoal installation,
discovery, or runtime activation. The compact adoption profile is machine-readable
in `agent-standards/obligations.json`; no host-local artifact path is authority.
