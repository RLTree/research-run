# Research Run Architecture

Research Run is one offline Rust command-line interface. It turns untrusted CLI
arguments and local JSON into typed research records, then publishes those
records inside one workspace without silently raising scientific claims.

## Command to authority flow

1. `src/cli/arguments.rs` parses the public command and enums.
2. `src/cli/commands.rs` constructs typed domain records and invokes the
   workspace authority.
3. `src/domain/` validates record identity, bounded text, references, and
   versioned shapes before effects.
4. `src/workspace/` confines paths, loads bounded snapshots, serializes writers,
   publishes atomically, validates references, recovers interrupted effects, and
   computes deterministic status.
5. `src/cli/render.rs` emits JSON or terminal-neutralized human output. Rendered
   output and receipts cannot promote a claim.

Canonical authority lives only in `.research-run/`: one manifest plus source,
claim, evidence, experiment, review-authority, human-review, inventory, typed
knowledge, and relationship JSON records. A signed review decision is the only
semantic authority that changes a claim assessment. The agent prepares a
canonical request; a separately controlled Ed25519 key signs it under the
`research-run-review-v1` namespace; import, recovery, validation, and status
verify the detached SSHSIG against the public key anchored by the repository
owner during initialization. The manifest's immutable random workspace ID
prevents same-name workspace replay. This proves configured-key control, while
human custody remains an operational boundary. The review also stores the
sorted evidence IDs reviewed by the human, and status applies it only while that
binding exactly matches the current claim evidence graph.

Retrofit and reconciliation use the same authority path. A read-only plan embeds
one versioned inventory policy, then scans each ordinary project file outside
`.git/` and the root `.research-run/`, classifies only from path and extension,
and binds byte count plus streamed SHA-256. The policy can declare exact
reference-only artifact roots and exact child Research Run workspaces. A
reference-only root retains only root metadata and declared provenance or
manifest metadata; its descendants are deliberately not inspected. A child
boundary retains the child manifest and available canonical inventory identity,
not child material. Apply rescans with the accepted policy and publishes one
complete inventory atomically. Later reconciliation records added, changed,
moved, missing, duplicate, or ambiguous material without editing project bytes
or converting absence into deletion.
New-workspace apply retains a plan-digest bootstrap marker until that inventory
commits. A failure after initialization is therefore an explicit ambiguous
effect recoverable only by reapplying the exact accepted plan.

Knowledge records cover goals, research questions, hypotheses, protocols,
methods, observations, measurements, analyses, interpretations, decisions,
risks, blockers, uncertainties, contradictions, next actions, plans,
presentations, and session summaries. Relationships append explicit dependency,
provenance, revision, supersession, invalidation, resolution, blocking, and
contradiction history. They never overwrite their endpoints and never participate
in claim assessment; `EvidenceLink` plus `ReviewDecision` remain the sole claim
graph and promotion authority.

Retrieval is a bounded deterministic projection over one canonical snapshot per
command. `list`,
`show`, `search`, `recent`, `timeline`, `related`, `unresolved`, `blockers`,
`next`, and `context` never write. JSON is the automation interface; `--human`
renders a terminal-neutralized inspection view. Search results name matched
fields and every item names its canonical authority path plus stale and
invalidation state.

`handoff create` wraps a bounded context projection in a versioned portable
envelope with an operator-supplied identity and UTC generation time. `handoff
inspect` validates and renders that envelope without requiring the originating
workspace. The envelope is a projection and cannot promote or replace canonical
records.

Migration keeps the accepted v0.1 JSON format intact. A read-only plan hashes
the sorted canonical authority paths and bytes into one SHA-256 boundary. Apply
rechecks that fingerprint under the workspace lock, creates only missing
extended record directories, and appends one migration record. It does not
rewrite, delete, or reinterpret an existing record.

Agent activation is a separate project-instruction boundary. Canonical protocol
installation and agent-integration readiness are distinct states. A read-only
`agent-integration plan` binds the exact manifest and protocol bytes, the active
root `AGENTS.override.md` or `AGENTS.md` surface, its existing digest, the
managed block, prospective digest, and plan digest. Apply rechecks the plan
under the canonical workspace lock, confines the target to those two root
filenames, rejects symlinks, drift, and conflicting markers, preserves existing
instruction bytes, and publishes the resulting instruction file atomically.
Fresh-Create pending instructions and append transaction leaves are born no
broader than `0600`, and the append transaction directory is born no broader
than `0700` in its access permission bits. On Linux only, the kernel-inherited
`S_ISGID` bit is accepted when a setgid parent snapshot and matching child group
make that inheritance internally consistent; it grants no group/other access.
Other special bits and all group/other access fail closed. Append writes version,
original `O`, reviewed `P`, and exchange=`P` through their held creation
descriptors while private. Before the first durability sync it sets `O`, `P`,
and exchange=`P` through those descriptors to the exact reviewed Unix mode; the
owner-only-access transaction directory contains any witness whose reviewed
mode is broader than `0600`. On Linux and macOS,
`rustix::fs::renameat_with` exchanges the
transaction and canonical leaves atomically relative to opened, identity-checked
directory descriptors; the canonical pathname remains continuously bound and
there is no ordinary-rename, link, or move fallback for canonical publication.
The canonical `P`, exchanged exact observed bytes, transaction, and root are
synced after exchange. Cleanup starts only after a bounded versioned completion
record is staged and synced inside the transaction, published create-only as an
fd-relative hard link, and synced with the root directory. Completion leaves are
born no broader than `0600` and receive that mode through their held descriptor
before content is written. Recovery opens recognized leaves fd-relatively with
no-follow and nonblocking flags and rejects every non-regular leaf before read
or sync. That plan-, byte-,
mode-, target-, and transaction-bound record is the sole authority for partial
or empty transaction cleanup; recovery re-syncs it before the next unlink. With
no completion record, recovery advances or finishes only exact full
`canonical=O/exchange=P` and `canonical=P/exchange=O` states whose version, plan
binding, bytes, identities, and modes agree. It classifies legacy layouts
separately and retains unknown, malformed, impossible, unsupported, or uncertain
state. Identical retry is a no-op. The preservation claim is exact
bytes plus Unix mode, not ownership, timestamps, xattrs, ACLs, universal safety
against direct writers, or protection from writes through already-open file
descriptors. The installed block requires bounded retrieval,
typed CLI contributions, no generic canonical JSON writes, a real validation
receipt, and v2 handoff. Readiness is scoped to a newly started agent run because
instruction discovery occurs at run start; current-session loading remains
unverified. Validation v2 reports readiness separately from canonical ledger
validity. An unavailable instruction inspection leaves a structurally valid
ledger valid while exposing readiness as unavailable; explicit
`agent-integration status` still fails closed. This boundary does not claim
universal agent compliance or prevent uncooperative filesystem mutation.
Handoff v2 acquires the workspace lock, derives context and validation from one
snapshot, and embeds the authority-bound validation result plus digests of the
result and context. Standalone v1 handoffs remain inspectable without that field.

## Proof and governance

- `tests/product.rs` routes product journeys, failure/recovery behavior, and
  repository-law tests.
- `scripts/check*` owns fast, full, coverage, standards, dependency, mutation,
  package/install, and observation commands.
- `AGENTS.md` routes the current contract; `STANDARD.md`, `policy.toml`, and
  `agent-standards/obligations.json` hold human and machine-readable law.
- `schemas/v1/` preserves canonical and legacy projection contracts;
  `schemas/v2/validation.schema.json` describes current validation output.
  Validation v1 does not describe or accept the v2 readiness-bearing shape.
- `examples/` provides the checked-in synthetic workspace.

The repository has no network runtime, service, database, async executor,
telemetry backend, user interface, Python product code, or release machinery.
Those absences are architectural constraints until a measured product need and
new authority contract justify them.
