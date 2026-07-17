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

## Development and proof surfaces

See [`STANDARD.md`](STANDARD.md) for invariants, budgets, dependencies, and exact
gates. Source, tests, package creation, local install, runtime journey, CI, pull
request, release, and real researcher use are separate proof surfaces. v0.1 does
not include a GUI, service, sync, telemetry, database, LLM provider, or generic
workflow engine.
