# Research Run

Research Run is a local-first Rust CLI for keeping sources, claims, evidence
links, experiment receipts, and human review decisions inspectable in Git. It
works offline and does not call an LLM or network service.

It answers a bounded question: **what does this workspace currently claim, what
evidence is connected to it, what failed or remains ambiguous, and what needs a
human decision next?** Validation checks the ledger; it does not prove the
science. A `supported` assessment means reviewed support within the recorded
workspace scope, not scientific truth in general.

## Five-minute quickstart

Build or install with the pinned Rust toolchain:

```console
cargo install --path . --locked
research-run init my-study --name "My study"
cd my-study
```

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

Inspect the claim ceiling, then record an explicit human review:

```console
research-run status
research-run review add \
  --id review-binding \
  --claim claim-binding \
  --decision limited \
  --rationale "The paper supports the claim, but the negative repeat limits it." \
  --reviewer "Researcher Name"
research-run validate
research-run status --json
```

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

Review decisions are local assertions made by the CLI user; v0.1 has no account
system or identity verification. Treat Git review and repository access controls
as the human-authorship boundary.
The CLI records the claim's current sorted evidence IDs with each review. Adding
later evidence makes the old decision historical and returns the claim to
`unreviewed` until another explicit human review covers the changed graph.

## Retrofit an existing project

Planning is read-only and emits authoritative JSON. Keep the plan outside the
target so writing the plan itself cannot change the candidate it describes:

```console
research-run retrofit plan existing-project \
  --name "Existing project" \
  --id inventory-initial \
  --observed-at 2026-07-18T20:00:00Z > /tmp/research-run-plan.json
research-run retrofit apply existing-project \
  --input /tmp/research-run-plan.json --json
```

Apply rescans every indexed byte and fails if the project changed after
planning. It creates only `.research-run/`, never modifies existing project
files, and repeating the same accepted plan is a no-op. After files change,
`reconcile plan` compares the current directory with the latest inventory and
reports added, changed, moved, missing, duplicate, or ambiguous material.
Ambiguous identity conflicts cannot be applied.

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
assessment still requires an explicit human `review add` decision bound to the
current evidence graph.

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

## Development and proof surfaces

See [`STANDARD.md`](STANDARD.md) for invariants, budgets, dependencies, and exact
gates. Use `scripts/check fast` during implementation and `scripts/check full`
when a source or package claim can move. `scripts/check coverage` is the separate
100% line/function/region source-coverage authority and requires
`cargo-llvm-cov`; a test pass is not a coverage pass. Source, tests, package
creation, local install, runtime
journey, CI, pull request, release, and real researcher use are separate proof
surfaces. v0.1 does not include a GUI, service, sync, telemetry, database, LLM
provider, or generic workflow engine.
