# Research Run

Research Run is a local-first Rust CLI for keeping sources, claims, evidence
links, experiment receipts, and human review decisions inspectable in Git. It
works offline and does not call an LLM or network service.

It answers a bounded question: **what does this workspace currently claim, what
evidence is connected to it, what failed or remains ambiguous, and what needs a
human decision next?** Validation checks the ledger; it does not prove the
science. A `supported` assessment means reviewed support within the recorded
workspace scope, not scientific truth in general.

## Source-checkout quickstart

Research Run 0.1.0 is not published. This path requires a local source checkout,
the pinned Rust toolchain, Git, and an Ed25519 public key. No human timing,
accessibility, or cognitive-load claim is implied.

Build or install with the pinned Rust toolchain:

```console
cargo install --path . --locked
REVIEW_PUBLIC_KEY="$(pwd)/researcher-review.pub"
research-run init my-study \
  --name "My study" \
  --review-authority-id primary-researcher \
  --review-authority-public-key "$REVIEW_PUBLIC_KEY"
cd my-study
```

Initialization anchors the one permitted Ed25519 review authority. The public
key may come from a hardware-backed key or 1Password SSH Agent; its private key
and each signing approval stay outside Research Run and the agent process.
`--without-review-authority` is an explicit irreversible opt-out that disables
claim promotion for that workspace.

Every initialized or retrofitted workspace contains the versioned canonical
protocol at
`.research-run/contribution-protocols/agent-contribution.json`. That proves the
protocol is installed, not that an agent will receive it before work. Bind the
protocol into the active root project instructions with an explicit reviewed
plan kept outside the target workspace:

```console
research-run agent-integration plan my-study > research-run-agent-plan.json
research-run agent-integration apply my-study \
  --input research-run-agent-plan.json --json
research-run agent-integration status my-study --json
```

The plan binds the manifest, contribution protocol, active `AGENTS.md` or
`AGENTS.override.md`, managed block, prospective bytes, and plan itself with
SHA-256. Apply rechecks the binding under the workspace lock, rejects symlinks,
drift, path escape, and conflicting managed content, and atomically creates or
appends without replacing existing instructions. An identical retry is a
no-op. Init, retrofit, and migration report onboarding as incomplete until this
separate step succeeds.

Codex discovers project instructions once when a run starts, so after apply,
start a fresh run/session before claiming that the instruction contract was
loaded ([Codex `AGENTS.md` guidance](https://learn.chatgpt.com/docs/agent-configuration/agents-md)).
Project hooks may be an optional trusted guardrail, but they require trust and
do not cover every tool path; they are not this product invariant
([Codex hooks guidance](https://learn.chatgpt.com/docs/hooks)). The supported
readiness claim is limited to deterministic instruction generation,
installation, and verification for a new agent run. It does not prove universal
agent compliance or prevent every out-of-band filesystem write.

A fresh agent that received the installed contract
must retrieve bounded context before work. Before answering or handing off, it
must classify new material and append supported typed records for new human
observations, corrections, decisions, negative or ambiguous results, blockers,
and next actions, then run `research-run validate --json` and retain that actual
CLI result as the validation receipt. Human input keeps `human`
authorship; agent analysis uses `ai`. No new material means no write, an
identical retry is a no-op, and only a signed human review may promote a claim.
`context` and `handoff create` surface this protocol without writing.
New handoffs use schema version 2 to carry the protocol; existing version 1
handoffs remain inspectable without it.

Add a source and a scoped claim:

```console
research-run source add \
  --id source-paper \
  --citation "Example et al. (2026)" \
  --locator "doi:10.0000/example" \
  --provenance human

research-run claim add \
  --id claim-binding \
  --text "The construct binds the target in this assay." \
  --scope "This assay and construct only" \
  --owner "Researcher Name" \
  --authorship human
```

Connect specific evidence. The link still cannot promote the claim:

```console
research-run evidence add \
  --id evidence-paper \
  --claim claim-binding \
  --source source-paper \
  --stance supports \
  --specific-evidence "Figure 2 reports binding in the matching assay." \
  --authorship human
```

Record a negative or ambiguous result with observations kept separate from
interpretation. Workspace artifacts are pointers; Research Run does not copy raw
data:

```console
research-run experiment add \
  --id experiment-repeat \
  --question "Does the effect reproduce?" \
  --method-ref "protocols/assay.md" \
  --observation "Signal matched the negative control." \
  --interpretation "This run does not reproduce the reported effect." \
  --limitation "One batch was tested." \
  --outcome negative \
  --next-move "Repeat with an independent batch." \
  --artifact "workspace:artifacts/summary.txt:Deidentified summary"
```

The quickstart created the workspace with its one review key anchored in the
immutable manifest. This initialization is the repository owner's
trust-bootstrap boundary; there is no post-initialization enrollment command.
Inspect the claim ceiling, prepare the exact request, sign it outside the agent
process, and import the detached SSH signature:

```console
research-run status
research-run review prepare \
  --id review-binding \
  --claim claim-binding \
  --decision limited \
  --rationale "The paper supports the claim, but the negative repeat limits it." \
  --reviewer "Researcher Name" > review-binding.json
ssh-keygen -Y sign \
  -f "$REVIEW_PUBLIC_KEY" \
  -n research-run-review-v1 \
  review-binding.json
research-run review add \
  --request review-binding.json \
  --signature review-binding.json.sig
research-run validate
research-run status --json
```

The signer may be a hardware-backed key or a 1Password SSH Agent key. Keep its
private key and signing approval outside the agent-accessible environment.
Research Run stores only the public key and detached signature. Cryptography
proves control of that configured key, not human personhood; repository access
control and owner-run initialization establish who may choose the key. v0.1
provides neither post-initialization enrollment nor key rotation.

Canonical records live under `.research-run/` as deterministic, versioned JSON.
The machine-readable v1 contracts are in [`schemas/v1`](schemas/v1). The complete
synthetic example is in [`examples/synthetic-assay`](examples/synthetic-assay).

## Failure and recovery behavior

- Repeating an identical add is a no-op; reusing an ID with different content
  fails closed.
- Unknown versions, fields, identities, or references fail validation.
- Absolute paths, `..` traversal, and symlinks in workspace authority paths are
  rejected.
- If a process stops between writing and publishing a record, validation reports
  the pending file. `research-run recover [PATH]` validates schema, references,
  paths, and current authority before completing or reconciling the publication.
  An explicit path also recovers an `init` interrupted before its manifest was
  published. Recovery never silently chooses conflicting content.
- Diagnostics identify a path and invariant, not record bodies or secret values.

Reviewer text is metadata, not authority. A review affects assessment only when
its detached SSH signature verifies against the workspace's one enrolled
Ed25519 authority over the exact immutable workspace identity, project, claim,
decision, rationale, evidence IDs, and subject digest. Adding later evidence
makes the old decision historical and returns the claim to `unreviewed` until
another signed review covers the changed graph.

Automation can distinguish failure classes by exit status: `1` is local I/O,
`2` is malformed or invalid input, `3` is not found, `4` is conflicting or
ambiguous authority, and `5` is budget exhaustion. Successful commands return
`0`. Error text is diagnostic only; callers should branch on the status.

## Retrofit an existing project

Planning is read-only and emits authoritative JSON. Keep the plan outside the
target so writing the plan itself cannot change the candidate it describes. Use
a durable sibling plan directory rather than a system temporary path:

```console
PLAN_DIR="$PWD/research-run-plans"
mkdir -p "$PLAN_DIR"
research-run retrofit plan existing-project \
  --name "Existing project" \
  --id inventory-initial \
  --observed-at 2026-07-18T20:00:00Z \
  --review-authority-id primary-researcher \
  --review-authority-public-key researcher-review.pub \
  > "$PLAN_DIR/research-run-plan.json"
research-run retrofit apply existing-project \
  --input "$PLAN_DIR/research-run-plan.json" --json
```

Apply rescans every indexed byte and fails if the project changed after
planning. It creates only `.research-run/`, never modifies existing project
files, installs the same canonical contribution protocol as `init`, and repeating
the same accepted plan is a no-op. The accepted plan binds
the authority decision used if apply creates the workspace. Passing
`--without-review-authority` to `retrofit plan` instead is the same explicit,
irreversible non-promoting opt-out as `init`. After files change,
`reconcile plan` compares the current directory with the latest inventory and
reports added, changed, moved, missing, duplicate, or ambiguous material.
Ambiguous identity conflicts cannot be applied.
If initialization commits but inventory publication does not, apply reports an
ambiguous effect and retains a plan-digest marker; reapply that exact accepted
plan to finish the combined bootstrap transaction.
Retrofit does not edit an existing project instruction file. Run the separate
`agent-integration plan` and reviewed `apply` flow afterward, then start a fresh
agent run/session.

### Large datasets and nested projects

New inventories embed a versioned policy. Without `--policy`, a new workspace
defaults to 20,000 indexed entries, 2 GiB per file, and 8 GiB total. A policy
may explicitly set `u64` byte totals above 64 GiB; content still streams through
SHA-256 rather than being buffered in memory. A legacy inventory without a
policy keeps its prior 2,048-entry, 64 MiB-per-file, 512 MiB-total behavior
until a `reconcile plan --policy` migration is accepted.

Use exact boundaries instead of broad exclusions. This compact policy is both
the CLI input and the policy copied into the accepted plan and inventory:

```json
{
  "schema_version": 1,
  "kind": "inventory-policy",
  "limits": {
    "max_entries": 20000,
    "max_file_bytes": 2147483648,
    "max_total_bytes": 68719476736
  },
  "boundaries": [
    {
      "boundary_kind": "reference-only",
      "path": "data/raw",
      "class": "artifact",
      "rationale": "Instrument-managed raw data",
      "reference": {
        "reference_kind": "manifest",
        "locator_type": "workspace",
        "locator": "manifests/raw-data.json",
        "description": "Operator-supplied dataset manifest"
      }
    },
    {
      "boundary_kind": "child-workspace",
      "path": "projects/znf385a",
      "rationale": "Independent project evidence authority"
    }
  ]
}
```

Pass it only while planning:

```console
research-run reconcile plan thesis-workspace \
  --name "Thesis synthesis" \
  --id inventory-policy-migration \
  --observed-at 2026-07-20T04:00:00Z \
  --policy "$PLAN_DIR/inventory-policy.json" \
  > "$PLAN_DIR/inventory-policy-plan.json"
```

Reference-only roots remain visible in canonical inventory state, but their
contents are not inspected, hashed, validated, or scientifically verified.
Child roots retain child identity and the available canonical snapshot identity,
not a recursive copy of child material. Missing, overlapping, escaping, or
symlinked declarations fail; `.gitignore` is never imported.

## Capture typed project knowledge

Nontrivial records use JSON files or standard input so an agent can inspect the
complete mutation before applying it:

```console
research-run knowledge add --input observation.json
research-run relationship add --input revision.json
```

Knowledge types include goals, research questions, hypotheses, protocols,
methods, observations, measurements, analyses, interpretations, decisions,
risks, blockers, uncertainties, contradictions, next actions, plans,
presentations, and session summaries. Relationships are append-only and typed;
revision, supersession, and invalidation history remains acyclic and never
erases the older record. These relationships cannot promote a claim. Claim
assessment still requires a signed human `review add` decision bound to the
current evidence graph and enrolled review authority.

## Retrieve bounded context

Retrieval commands emit deterministic JSON by default and accept `--human` for
a compact researcher-facing view:

```console
research-run list --kind observation --limit 20
research-run show --kind knowledge --id observation-one
research-run search "negative control" --limit 20
research-run recent --limit 20
research-run timeline --limit 50
research-run related --kind knowledge --id analysis-one
research-run unresolved
research-run blockers
research-run next
research-run context --query "binding assay" --limit 20
```

Search explains which fields matched. Results identify canonical authority and
keep stale or invalidated records visible instead of silently preferring newer
prose. Context bundles include the requested matches, unresolved material,
blockers, next actions, connected relationships, and the scientific claim
ceiling.

Create a portable session handoff and validate it from a fresh directory:

```console
research-run handoff create \
  --id handoff-current \
  --generated-at 2026-07-18T20:05:00Z \
  --limit 50 > /tmp/research-run-handoff.json
research-run handoff inspect \
  --input /tmp/research-run-handoff.json --human
```

The handoff includes recent or query-matched state, important authority paths,
unresolved questions, blockers, next actions, relationships, and the claim
ceiling. It is a portable projection, not claim or workspace authority.

## Migrate an accepted v0.1 workspace

Migration adopts the extended directory shape without rewriting v0.1 records:

```console
research-run migrate plan existing-project \
  --id migration-v01 \
  --migrated-at 2026-07-18T21:00:00Z > /tmp/research-run-migration.json
research-run migrate apply existing-project \
  --input /tmp/research-run-migration.json --json
```

The plan binds every canonical v0.1 JSON byte into a sorted aggregate SHA-256.
Apply fails if authority changed, creates only missing extended record
directories, installs the contribution protocol for a pre-activation workspace,
and appends a migration record without rewriting existing canonical records.
Repeating the same accepted plan is a no-op. Until migrated, validation and
mutation fail closed with a workspace-activation diagnostic; migration planning
remains available so the workspace can be upgraded losslessly.
Migration likewise preserves project instructions and reports agent integration
as incomplete until the separate plan/apply flow is accepted.

## Development and proof surfaces

See [`STANDARD.md`](STANDARD.md) for invariants, budgets, dependencies, and exact
gates. Use `scripts/check fast` during implementation and `scripts/check full`
when a source claim can move; its artifact sub-gate runs only on the platform
declared by the artifact disposition. `scripts/check coverage` is the separate
100% line/function/region source-coverage authority and requires
`cargo-llvm-cov`; a test pass is not a coverage pass. Source, tests, package
creation, local install, runtime
journey, CI, pull request, release, and real researcher use are separate proof
surfaces. v0.1 does not include a GUI, service, sync, telemetry, database, LLM
provider, or generic workflow engine.

The unreleased `0.1.0` candidate is governed by
[`docs/release/0.1.0.md`](docs/release/0.1.0.md). Product Fitness observations
use the consent-first
[`docs/release/product-fitness-protocol.md`](docs/release/product-fitness-protocol.md)
and remain external, deidentified evidence by default.
