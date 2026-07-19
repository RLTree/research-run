# Complete Agent-Operated Research Run

## Goal

Complete Research Run as an installed Rust CLI that lets an agent initialize or
retrofit a scientific project, preserve and index existing material, maintain a
typed and review-governed knowledge history, retrieve bounded context, reconcile
filesystem change safely, migrate v0.1 without data loss, and hand useful state to
a fresh agent. Finish on a clean feature-branch commit, push it, and create or
update a pull request. Do not merge or release.

## Success contract

1. **User and job:** an agent operating for a researcher can make an unfamiliar
   research directory durable and navigable without a manual structural tour.
2. **Complete journey:** installed-binary init, populated retrofit, indexing,
   typed capture and relationships, bounded retrieval, reconciliation, history,
   migration, and fresh-agent handoff work as one deterministic workflow.
3. **Semantic authority:** versioned canonical JSON under `.research-run/` is the
   only product authority; only an explicit typed human `ReviewDecision` can
   promote a claim assessment.
4. **Safety:** parse before effects; reject malformed, unknown, escaping,
   symlinked, over-budget, identity-conflicting, and ambiguous prior-effect input;
   publish atomically and recoverably without overwriting project bytes.
5. **Proof:** exact-candidate Rust gates, 100% authoritative coverage, bounded
   mutation, dependency/security checks, package/install journeys, Gold Stack
   profile check, UltraGoal-governed reviews, and the disposable Nathan benchmark
   support only their distinct claim surfaces.
6. **Delivery:** coherent commits, synchronized feature branch, green required
   GitHub jobs, and an open pull request; release, merge, scientific truth, and
   unobserved real-use claims remain withheld.

## Baseline and authority

- Repository root: the Git root containing this plan (`$REPO_ROOT`).
- Starting branch and commit: `main` at
  `4379bb5356e5898020b239c3ae7e7852d213f73c`, synchronized with `origin/main`.
- Transfer disposition: the accepted Rust workspace, product source, schemas,
  tests, checks, standards, and prior active plan are present. This run may
  extend them; it must not reconstruct an absent implementation from memory.
- Product law: `STANDARD.md`, `policy.toml`, and `ARCHITECTURE.md`.
- Workflow and proof law: `AGENT_STANDARDS.md`,
  `agent-standards/obligations.json`, `scripts/check-standards`, this plan, and
  `docs/research/2026-07-17-v0.1-evidence-and-decisions.md`.
- Current branch: `codex/research-run-complete-agent-workspace`.

## Constraints

- Preserve accepted behavior, transferred material, unrelated user work, and
  every existing project byte during retrofit unless a separately authorized
  operation explicitly changes it.
- Keep one semantic mutation path: typed parse, complete validation, intended
  mutation computation, atomic/recoverable publication, non-authoritative
  projection or receipt.
- Keep observations distinct from interpretation and retain negative,
  ambiguous, contradictory, invalidated, revised, and superseded history.
- Keep JSON authoritative, output bounded and deterministic, commands
  noninteractive and composable, and errors typed with meaningful exit codes.
- Retain the local-filesystem Rust architecture and minimal dependencies unless
  live evidence demonstrates a conflicting requirement. Do not add a service,
  database, hosted model, vector store, telemetry stack, or browser UI.
- Treat source, profile, package, install, cache, discovery, runtime, journey,
  GitHub, real-use, release, and scientific-validity proof separately.
- Use the representative Nathan project read-only; mutate only a disposable
  copy and avoid sensitive or unnecessary project content in retained evidence.

## Execution graph

### N0 — Freeze live authority and routing

Prove repository identity and transferred state. Refresh Gold Stack and
UltraGoal source/install/runtime identities. Run the deterministic Gold Stack
route and compact context, inspect the retrofit preview, and run the UltraGoal
capability/context and exact-candidate fit probes. Record conflicts and claim
ceilings before accepting mutations.

### N1 — Contract and gap freeze

Inventory existing commands, records, schemas, storage, migrations, tests, and
the Nathan benchmark shape. Produce the smallest complete knowledge model and
public command contract. Reconcile the Gold Stack profile without creating a
second governance plane. Freeze the product/authority/storage boundary and run
the first critical four-persona review.

### N2 — Lifecycle and indexing

Implement safe empty-directory init, unrelated-file init, populated retrofit
planning/application, material inventory and indexing, retry/interruption
behavior, and byte-preservation checks through the existing authority path.

### N3 — Knowledge and history

Implement the missing record classes and typed relationships, including goals,
questions, hypotheses, methods, observations, analyses, decisions, risks,
uncertainties, contradictions, next actions, session summaries, provenance,
supersession, revision, invalidation, and unresolved conflict history. Keep
claim promotion exclusive to explicit human review.

### N4 — Retrieval, reconciliation, migration, and handoff

Implement deterministic bounded list, show, search, recent, timeline, related,
unresolved, blocker, next-action, and context projections; reconcile changed,
moved, missing, stale, duplicate, or conflicting material fail-closed; migrate
the accepted v0.1 format without loss; generate and consume compact session
handoffs.

### N5 — Exact-candidate proof

Run focused tests during integration, then freeze a clean candidate. Run the
standards, namespace, Rust, coverage, mutation, dependency, security,
package/install, installed journey, Gold Stack checker, disposable Nathan
benchmark, performance/size, and clean-checkout surfaces. Re-run only invalidated
surfaces after later bytes.

### N6 — Reviews and delivery

Use the current UltraGoal Material Review Scope Gate before each review. Run
routine risk-matched review where required and the final fresh four-persona
completion round against one exact candidate. Repair once within the defined
cadence or narrow/withhold the affected claim. Push the synchronized feature
branch, verify required GitHub jobs, and create or update the pull request.

## Verification commands

The final exact candidate must include the repository-routed commands below plus
any narrower checks added by this run:

```console
scripts/check fast
scripts/check full
scripts/check coverage
scripts/check dependencies
scripts/check mutations
scripts/check artifacts
scripts/check-standards
cargo fmt --check
cargo check --locked --all-targets
cargo nextest run --locked --all-targets
cargo test --locked --doc
cargo clippy --locked --all-targets --all-features -- -D warnings
gold-stack-kernel check-profile --repo "$PWD"
```

The UltraGoal and installed-binary commands, benchmark-copy recipe, candidate
anchors, review receipts, GitHub checks, and observed results will be recorded as
they become current. A command listed here is an obligation, not evidence of a
pass.

## Progress and decisions

- 2026-07-18: Created durable goal
  `019f7821-d991-7701-b68d-9c3b1328645e` before substantive implementation.
- 2026-07-18: Confirmed the transferred implementation is present and the
  starting worktree is clean and synchronized. Created the feature branch.
- 2026-07-18: Loaded the named Gold Stack router skill. Its generated profile can
  prove profile consistency only; it cannot promote product or runtime claims.

## Current claim ceiling

No completion claim is active for this plan. Prior accepted behavior is present,
but all new and final claims remain withheld pending a clean exact candidate and
fresh proof on their own surfaces. Release, merge, scientific truth, scientific
impact, and unobserved real researcher usefulness are outside this run.
