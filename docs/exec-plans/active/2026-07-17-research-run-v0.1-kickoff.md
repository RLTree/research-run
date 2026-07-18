# Research Run v0.1 Development Kickoff

## Goal

Ship a strong, usable Research Run v0.1: a local-first, provenance-bound CLI and
Git-friendly workspace that helps a researcher connect sources to claims, record
experiments including negative results, validate the evidence graph, and produce
a truthful current-state brief for humans and agents.

Immediately create one durable goal with that objective. Keep it active until the
required product and proof surfaces pass.

## Success

A new user can complete this clean-clone journey in five minutes:

1. Initialize a research project.
2. Add a source.
3. Add a claim and connect evidence that supports, limits, or contradicts it.
4. Record an experiment with observations kept separate from interpretation,
   including a negative or ambiguous result and bounded artifact pointers.
5. Validate the workspace.
6. Read deterministic human and JSON status that identifies claim ceilings,
   blockers, unsupported claims, unreviewed AI drafts, and next actions.

The same journey must survive interruption and retry without corrupting,
duplicating, or silently promoting authoritative state. Ship a synthetic or fully
deidentified example, a concise quickstart, versioned data contracts, continuous
integration, and a distributable CLI. Commit coherent verified units, push the
feature branch, and open a pull request. Do not merge or release.

## Context

Read `docs/research/2026-07-17-v0.1-evidence-and-decisions.md` before choosing the
stack or data model. It records the reviewed sessions, live repositories,
official guidance, product alternatives, domain boundaries, implementation graph,
and applicable Superpowers and Product Design procedures.

The primary user is a graduate researcher or research engineer who already uses
files, notebooks, Git, literature managers, instruments, and AI assistants. The
job is to make the current evidence state inspectable and actionable without
replacing those tools or scientific judgment. The product should provide value
offline without an LLM; agents are consumers of the same typed, auditable state as
humans.

## Constraints

- Keep canonical records local, human-readable, deterministic, versioned, and
  reviewable in Git. Derived status, indexes, summaries, and receipts must be
  reconstructable and cannot raise claims.
- Give records stable identities and typed references. Parse untrusted input at
  the boundary before behavior. Use atomic, workspace-confined writes and fail
  closed on path, symlink, schema, reference, or effect ambiguity.
- Keep observations separate from interpretation. Preserve negative and ambiguous
  results. Evidence and AI-generated prose cannot mark a claim supported without
  an explicit human review decision.
- Reference raw data instead of copying it by default. Never persist or expose
  secrets, private collaborator messages, institutional forms, unpublished raw
  datasets, human-subject data, or biosafety-sensitive operational detail.
- Choose the smallest coherent stack that provides typed parsing, deterministic
  storage, safe writes, fast tests, and practical CLI distribution. Record the
  decision from current repository evidence.
- Do not build a GUI, network service, account system, telemetry, cloud sync,
  hosted database, LLM provider, vector store, plugin platform, workflow engine,
  collaboration server, or generic scientific ontology in v0.1.
- Work as one Sol agent at medium reasoning. Do not spawn subagents or introduce
  Ultra coordination. Use the implementation graph as dependency guidance, not
  as a mandate for ceremony or tiny tasks.
- Preserve unrelated work and avoid destructive Git operations. Feature-branch
  commits, pushes, and a pull request to `RLTree/research-run` are authorized.
  Merging, releasing, repository-setting changes, and writes outside this project
  require explicit approval.

## Output

Build the product through a thin vertical slice first, then close the remaining
v0.1 acceptance gaps. Keep the evidence/decision document and a living ExecPlan
current when discoveries materially change the product, architecture, or claim
ceiling.

Finish with the user-visible result; changed files and commits; exact checks and
outcomes; the pull request URL; supported and withheld claims; residual risks; and
the smallest credible next-version candidates.

## Verification

Map each success condition to fresh evidence and keep these proof surfaces
distinct:

- domain and schema tests for valid and invalid records;
- public CLI commands against real temporary workspaces;
- the complete clean-clone researcher journey;
- interruption, retry, path-confinement, symlink, malformed-input, reference, and
  diagnostic-redaction cases;
- proof that evidence, generated output, and retry cannot silently promote claims;
- build, test, lint, format, packaging smoke, CI, and repository secret scans; and
- the expected remote feature branch and open pull request.

Deterministic checks precede model judgment. Unit tests, source inspection,
documentation, packaging, GitHub state, and real product use prove different
claims. Do not complete the goal while a required surface is planned, inferred,
stale, or supported only by a self-authored review.

Continue through ordinary implementation and review failures. Stop for a
destructive action, unauthorized external write, missing required access, exposed
sensitive material, or a material product decision with no safe default.

## Living progress

- 2026-07-17: A new security/coverage/standards hardening goal superseded the
  completed v0.1 build goal without reopening accepted commits. Initial Codex
  Security scan `d6d92c2b-242a-4232-bcb1-1c9b8f96ed4c` sealed 11 low findings
  and 17 mandatory repair boundaries on exact commit `8949454`. One coherent
  repair is in progress; final security, coverage, package/install, CI, and
  product claims remain withheld until exact-candidate proof passes.

- 2026-07-17: Durable goal created; implementation is running on
  `codex/research-run-v0.1-build` from the grounded kickoff commit.
- 2026-07-17: Tree superseded the initial uncommitted Python decision with the
  layered-core plus Rust-overlay gold stack. Python files and caches created by
  the implementation attempt were removed before commit. The behavioral journey
  and retry scenarios were retained as Rust acceptance tests.
- 2026-07-17: Rust implementation, package, adversarial cases, recovery repair,
  and clean-clone installed-binary journey passed their distinct local surfaces.
  Recovery commit `9e1d411b670eec4d2073cc775d8bf5081f841154` passed final bounded Round 2
  signoff; no Round 3 was opened.
- 2026-07-17: UltraGoal source manifest 0.0.12 at
  `69787f20adcf0b99c7f3a26f71b35a215fda28d6` produced a conflicting 70-mutation
  fit plan on the clean recovery candidate, with one conflict at Research Run's
  `AGENTS.md`. The exact source commit also failed an independent build under its
  warnings-as-errors policy, while the observed installed cache remains 0.0.11.
  The plan was not applied; a compact repository-owned source-governance profile
  is the bounded retrofit.
- Current claim ceiling: local Rust source, package, recovery, and installed CLI
  journey are supported on their named candidates. GitHub PR, release, real use,
  scientific truth, exact 100% source coverage, and installed/discovered/runtime
  UltraGoal activation remain unproven.
