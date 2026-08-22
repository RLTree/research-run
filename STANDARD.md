# Research Run Standard

## Authority and claim ceiling

Canonical, versioned JSON records under `.research-run/` are the only product
authority. Human and JSON status are deterministic projections. Evidence links
describe relationships but do not promote claims. Only an explicit human review
decision may set a claim assessment to `supported`, `limited`, `contradicted`, or
`unsupported`. `supported` means reviewed support within this workspace and
scope—not general scientific truth.

Each review binds to the immutable random workspace identity and the sorted
immutable evidence IDs present for its claim at the decision boundary. It
carries an SSHSIG from the Ed25519 key anchored during owner-controlled
workspace initialization. Cryptography proves configured-key control, not
personhood. Reviewer text is metadata and never authorizes promotion. Later
evidence makes that decision historical and returns the claim to `unreviewed`
until a new signed review binds the changed graph.
`supported` requires at least one recorded evidence link; recovery and direct
record ingestion enforce the same content and signature binding as the CLI.

## Current stack

Research Run v0.1 is a correctness-critical Rust CLI on the pinned stable 1.97.1
toolchain. Cargo is the build, test, package, and distribution substrate. The
runtime dependency budget is deliberately small:

- `clap`: typed public CLI parsing and generated help;
- `serde`: typed domain serialization and deserialization;
- `serde_json`: deterministic, human-readable canonical JSON;
- `sha2`: portable SHA-256 material identity for byte-preserving retrofit and
  reconciliation plans; fingerprints detect exact content identity but do not
  infer scientific meaning.
- `base64ct` and `ed25519-dalek`: narrowly parse OpenSSH Ed25519 public keys and
  SSHSIG envelopes and verify detached signatures. Research Run never reads or
  stores a review private key.
- `getrandom`: generate an immutable 256-bit workspace authorization-domain ID
  during initialization.
- `rustix` 1.1.4 with `fs`: safe, descriptor-anchored atomic exchange for
  supported Linux and macOS project-instruction append publication.

No async runtime, database, network client, logging/telemetry stack, production
schema framework, or UI dependency is justified by the v0.1 job. Standard-
library filesystem primitives provide bounded reads and ordinary no-clobber
publication; the narrowly pinned `rustix` surface owns atomic instruction-file
exchange without weakening `unsafe_code = "forbid"`.

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
- Each record is at most 1 MiB except one inventory plan or canonical inventory
  record, which is independently bounded to 32 MiB. Canonical collections are
  bounded to 10,000 records per kind; one snapshot or recovery read is bounded
  to 64 MiB; user text is bounded; status is derived with indexed relationships
  in deterministic ID order. These are safety and memory/storage budgets, not
  scale claims.
- A versioned typed inventory policy controls indexed entry, per-file, and
  aggregate byte budgets. New workspaces default to 20,000 entries, 2 GiB per
  file, and 8 GiB total; explicit policies use `u64` byte budgets and may name
  totals above 64 GiB. Legacy policyless inventories retain exactly 2,048
  entries, 64 MiB per material, and 512 MiB total until an explicit reconcile
  plan embeds a policy.
- `.git/` and the root `.research-run/` authority are skipped. Every ordinary
  regular file outside an explicit boundary is streamed into SHA-256. A
  reference-only boundary records only its normalized root, classification,
  declaration metadata, and root type/size; descendants are not walked,
  hashed, validated, or scientifically verified. A child-workspace boundary
  records parsed child manifest identity and available canonical inventory
  identity without recursively inventorying child material. There is no
  `.gitignore` import or generic ignore engine. Undeclared nested workspaces,
  symlinks at declared roots or path components, path escape, malformed or
  overlapping declarations, unsupported nodes, budget exhaustion, and identity
  races fail closed.
- Mutating CLI commands take an operating-system file lock under the canonical
  state root. The lock releases on process exit and serializes supported CLI
  writers; uncooperative concurrent filesystem mutation is not a supported
  product mode and every read still rechecks opened-file identity.
- Installing the canonical contribution protocol does not make agent integration
  ready. Project-instruction integration is an explicit plan/apply boundary that
  binds the current manifest, protocol, active root instruction file, managed
  content, and prospective bytes with SHA-256. Apply preserves existing
  instructions, is confined to `AGENTS.md` or `AGENTS.override.md`, rejects
  symlinks, drift, and conflicting managed content, and is idempotent on
  identical retry. Append stages independently durable version, original `O`,
  reviewed `P`, and exchange=`P` witnesses with the reviewed Unix mode. On
  Linux and macOS it exchanges the transaction leaf and canonical instruction
  leaf atomically through opened, identity-checked directory descriptors; the
  canonical pathname remains bound throughout the supported exchange. There is
  no non-atomic fallback. Pre- and post-exchange directories are synced and
  recovery accepts only exact versioned `canonical=O/exchange=P` or
  `canonical=P/exchange=O` states. Legacy, unsupported, unknown, malformed,
  impossible, and uncertain states fail closed and retain transaction evidence.
  The preservation claim covers exact bytes and Unix mode only, not owner,
  timestamps, xattrs, ACLs, writes through already-open descriptors, or every
  uncooperative filesystem writer. Readiness is only for a fresh agent
  run/session; the current session remains unverified. This proves no universal
  compliance.
- Diagnostics name record paths and invariant failures but never echo record
  bodies, environment variables, or secrets.
- Review preparation is read-only. The repository owner anchors the one Ed25519
  authority during initialization; post-initialization enrollment and rotation
  are unsupported. Promotion requires import of a detached
  `research-run-review-v1` SSH signature from that authority over the immutable
  workspace ID and exact request. Missing, malformed, wrong-key, replayed,
  cross-workspace, and modified-request signatures fail closed during
  publication, recovery, validation, and status.
- Typed knowledge records are append-only. `revises`, `supersedes`, and
  `invalidates` relationships require knowledge endpoints and must remain
  acyclic. Generic relationships may describe a contradiction or dependency,
  but cannot affect claim assessment; only the existing evidence graph plus an
  explicit human review decision can do that.
- v0.1 migration is lossless adoption, not schema reinterpretation. It binds
  all canonical JSON paths and bytes before effects, rechecks them under the
  write lock, creates only missing record directories, and appends one migration
  record. Existing canonical bytes are never rewritten or deleted.
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
candidate source, dependency, security, and mutation gate and runs only when a
claim can move. It also runs the package, install, journey, and observation gate
when the current host matches the platform declared by the artifact disposition;
otherwise it reports those distinct claims as withheld. `scripts/check
coverage` requires `cargo-llvm-cov` and enforces 100% line, function, and region
coverage over every target with every production file retained in the
denominator. The checker counts each source coordinate once because LLVM emits
duplicate mappings for generic monomorphs and the separate `cfg(test)` library
build; every unique production region, line, and function must execute at least
once. Unit tests run through nextest and doctests run through Cargo as separate
correctness surfaces.
`scripts/check-standards`
validates all 12 source-guidance modules plus contextual namespace red, green,
and tamper fixtures plus semantic-tree limits. `scripts/check dependencies`
requires zero duplicate dependency versions, current RustSec audit and Cargo Deny
passes, and a valid ephemeral CycloneDX inventory. `scripts/check mutations`
tests a bounded set covering typed parsing and validation, claim promotion, path
confinement, locking, publication, recovery, inventory/reconciliation,
knowledge/history, migration, retrieval, handoff validation, and terminal
neutralization; every viable mutant must be killed and compiler-rejected mutants
remain explicitly classified.
`scripts/check artifacts` requires a clean commit, builds and installs the Cargo
package, exercises the installed researcher journey, and emits machine-local
latency, size, install-footprint, and target-growth observations. There is no
cross-machine performance or regression-budget claim. CI and local gitleaks prove
their own secret-scan surfaces. Package, install, runtime, journey, GitHub,
release, and real-user evidence remain distinct.

## Release boundary

The v0.1 release class is a technical preview governed by
`docs/release/0.1.0.md`. Release readiness may claim exact-candidate ledger
integrity, package/install mechanics, and the installed journey on named
platforms. It cannot claim personhood, scientific truth, general Product
Fitness, continuance, or scientific impact. A Product Fitness observation uses
the consent-first external-receipt protocol in
`docs/release/product-fitness-protocol.md`; synthetic and agent-operated
journeys remain non-substitutes. A detached release-candidate signature proves
configured-key control over the exact request, not identity beyond the
repository owner's key choice. No repository check publishes, tags, merges, or
creates a release.

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
signoff. The following fit observation is historical and non-authoritative for
the current candidate; it cannot establish current fit classification or
readiness. A canonical source probe used a clean detached worktree at UltraGoal
commit `0355039bf621113e7089c235a298c7a8b085397f` and built the `ultragoal` binary
with `cargo build --locked --offline --package ultragoal --bin ultragoal`.
Against historical Research Run candidate
`80657ca065a93a49521f414db891dff775cf45e9`, `fit inspect` and `fit plan`
classified the retrofit as `conflicting`: 67 missing generated files and four
conflicts at Research Run-owned `AGENTS.md`, `AGENT_STANDARDS.md`,
`ARCHITECTURE.md`, and `scripts/check`. The stable plan digest in that clean
source context was
`sha256:71dd2b2a2bb8246586596de234d9e5cf5b53827ea15446de78b0a0ba4efdd7a2`.
UltraGoal's canonical production adapter refuses every conflicting plan before
effects, so the plan was not accepted or applied. The observed installed cache
remains 0.0.11 and no installed `ultragoal` command was discovered. These facts
support a historical source-built inspection and plan only. The current fit
classification has not been reprobed, no fit receipt exists, and current
package, installed, discovered, runtime-active, applied-fit, fit-readiness, and
full fitted-governance claims remain withheld.

The additional validated source-guidance artifact is
`harness-ultragoal-governance-complete.zip`, SHA-256
`089382f27b64c6eb219b3a032296917dc8984129a52e56e63cb77ba66ba72377`.
Its independent manifest and validator classify 160 laws and 172 items with zero
issues. This portable identity does not establish UltraGoal installation,
discovery, or runtime activation. The compact adoption profile is machine-readable
in `agent-standards/obligations.json`; no host-local artifact path is authority.
