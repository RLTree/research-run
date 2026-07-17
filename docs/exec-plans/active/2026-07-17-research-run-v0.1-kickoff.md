# Research Run v0.1 Development Kickoff

Work in the connected `RLTree/research-run` repository. Turn the placeholder into
a strong, usable first product, not a framework demo or a cathedral.

Immediately create one durable goal with this exact objective:

> Ship Research Run v0.1 as a local-first, provenance-bound research workspace
> that lets a researcher initialize a project, connect sources to claims, record
> experiments including negative results, validate the evidence graph, and emit a
> truthful current-state brief for humans and agents.

Keep the goal active until every required proof surface below passes.

## Goal

Build a local CLI and Git-friendly workspace format that helps a researcher answer:

1. What do we currently claim?
2. What evidence supports, limits, or contradicts each claim?
3. What experiments were attempted, including negative and ambiguous results?
4. What is the current claim ceiling and blocker?
5. What should a human or agent inspect or do next?

## Success

A new user can follow a five-minute quickstart from a clean clone to:

- initialize a project;
- add a source;
- add a claim and connect bounded evidence;
- record an experiment with its design or protocol reference, observations,
  artifact pointers, interpretation, limitations, and outcome;
- preserve a negative or ambiguous result;
- validate the workspace; and
- obtain deterministic human and JSON status showing claim ceilings, blockers,
  unsupported claims, unreviewed AI drafts, and next actions.

The same journey must survive interruption and retry without corrupting,
duplicating, or silently promoting authoritative state. Ship a synthetic or fully
deidentified example, focused documentation, continuous integration, a versioned
data contract, and a pushed feature branch with a pull request. Do not merge or
release.

## Context

The audience is graduate students, research engineers, and small scientific teams
already using files, notebooks, Git, literature managers, instruments, and AI
assistants. Research Run should connect those tools, not replace an electronic lab
notebook, laboratory information management system, raw-data store, statistical
package, reference manager, or scientific judgment.

Use live evidence from these sources before freezing the design:

- `/Users/terrynoblin/Projects/2026-Nathan-Summer-Project` for evidence surfaces,
  claim ceilings, experiment receipts, negative results, and researcher handoffs;
- `/Users/terrynoblin/Projects/harness-ultragoal-plugin-proposal` for singular
  authority, bounded effects, recovery, receipts, and proof-surface separation;
- session `019f669f-a52e-7782-ac24-867cb50ed35b` for tailored user journeys,
  meaningful feedback, examples and anti-examples, and real-use quality;
- session `019f50d9-7485-7080-be1a-c58de6747989` for one outcome contract,
  route-to-model selection, requested versus effective routing, compact handoffs,
  and representative evaluation before escalation;
- relevant recent memory and session evidence for harness, loop, systems, and
  agentic engineering. Treat memory as routing context and verify current claims
  from live files, Git, tests, runtime, or GitHub.

The core research distinction is between literature, design rationale,
observation, analysis, and interpretation. The core systems distinction is between
canonical records and derived status, receipts, or summaries. The core agentic
principle is that autonomy comes from verifiable state, bounded effects,
restartable operations, and truthful handoffs, not from embedding an LLM.

## Constraints

- Keep every v0.1 operation local-first and offline-capable.
- Keep canonical records human-readable, deterministic, versioned, and easy to
  review in Git.
- Give records stable identities and explicit typed references.
- Preserve observations separately from interpretations and negative results as
  first-class records.
- Require qualifying evidence before a claim can be supported; AI-generated prose
  cannot promote a claim without explicit human review.
- Keep one canonical authority. Status, indexes, receipts, and context packets are
  reconstructable projections.
- Use atomic, workspace-confined writes and fail closed on path, symlink, effect,
  schema, or reference ambiguity.
- Do not copy raw data by default. Store bounded artifact pointers, descriptions,
  provenance, and optional digests.
- Never persist or expose secrets, private collaborator messages, institutional
  forms, unpublished raw datasets, human-subject data, or biosafety-sensitive
  operational detail.
- Choose the smallest stack that delivers typed parsing, deterministic storage,
  safe local writes, and a distributable CLI. Record the decision; do not assume a
  language solely from neighboring projects.
- Do not build a GUI, network service, account system, telemetry, cloud sync,
  hosted database, LLM provider, vector store, plugin platform, workflow engine,
  collaboration server, or generic scientific ontology for v0.1.
- Preserve unrelated work and avoid destructive Git operations.
- Commits and feature-branch pushes to `RLTree/research-run` are authorized.
  Merging, releasing, changing repository settings, or writing elsewhere requires
  explicit approval.

## Execution

Use this same outcome contract regardless of model. Leave the user's selected
model and reasoning setting unchanged. Use Luna only for narrow typed
transformations, Terra for routine implementation and review, Sol for difficult
root judgment, and Ultra only when the current mode and genuinely independent
work justify parallel agents.

Keep one root integrator responsible for the goal, shared schemas, status
vocabulary, public CLI, dependencies, compatibility, Git state, reconciliation,
and final acceptance. Root must close the serial foundation node before
implementation fan-out: inspect live sources, choose the product cut and stack,
freeze canonical interfaces and the golden journey, define adversarial fixtures,
and establish a clean baseline.

After that freeze, derive a few substantial dependency-closed lanes with disjoint
path and semantic ownership. The natural candidates are:

- workspace discovery, storage, atomic publication, and recovery;
- sources, claims, evidence links, and status transitions;
- experiment receipts, outcome classes, and artifact references;
- CLI journey, quickstart, synthetic example, and stable output; and
- adversarial validation for malformed state, unsafe paths, interruption,
  unsupported promotion, and secret leakage.

Do not parallelize changes to the manifest, lockfile, canonical schema, public CLI
entry point, shared fixtures, or compatibility policy. Root owns or explicitly
serializes those files. Let the live design determine exact packages and paths;
do not create tiny leases or agents for work one implementer can hold coherently.

Each lane returns owned paths and commits, exact verification evidence, unmet
requirements, and any requested shared-interface change. Root integrates in
dependency order and reruns affected checks on the combined tree. Use one
independent milestone review after the walking slice works; repeat review only
after a material behavior, authority, dependency, claim-surface, or contradiction
change.

Use the relevant Superpowers procedures for planning, managed worktrees,
test-driven development, systematic debugging, review, and verification. Use
Product Design for the walking-slice journey, information hierarchy, command
ergonomics, error recovery, and truthful status presentation. Do not invoke visual
prototype workflows without a real visual target or evidence that a GUI is needed.

## Verification

Map every success requirement to fresh evidence and prove these surfaces
separately:

- source and schema behavior in current code;
- focused valid and invalid domain tests;
- public commands against a real temporary workspace;
- the clean-clone quickstart and complete researcher journey;
- interrupted-write and retry recovery;
- path confinement, symlink handling, untrusted input, diagnostic redaction, and
  repository secret scans;
- clean build, test, lint, format, and packaging smoke checks; and
- the expected GitHub feature branch and pull request.

Deterministic checks precede model judgment. A review artifact or passing unit test
does not prove the integrated journey. Do not mark the goal complete while a
required surface is planned, inferred, or supported only by stale evidence.

## Output

Finish with the user-visible result, changed files and commits, exact commands and
outcomes, pull request URL, supported and withheld claims, residual risks, and the
smallest credible next-version candidates.

Continue through ordinary implementation failures and review findings. Stop only
for a destructive action, an unauthorized external write, missing required
access, exposed sensitive material, or a product-boundary decision with no safe
default.
