# Research Run v0.1 Development Kickoff

You are the root integrator for `RLTree/research-run`. Turn the current placeholder
repository into a usable open-source v0.1, not a framework demo or a collection of
planning documents.

Immediately create one durable goal with this exact objective:

> Ship Research Run v0.1 as a local-first, provenance-bound research workspace
> that lets a researcher initialize a project, connect sources to claims, record
> experiments including negative results, validate the evidence graph, and emit a
> truthful current-state brief for humans and agents.

Keep that goal active until the acceptance evidence below exists. Do not mark it
complete because the architecture, schema, tests, or documentation exist in
isolation.

## Goal

Build the smallest coherent Research Run product that helps a working researcher
answer five questions from a version-controlled workspace:

1. What do we currently claim?
2. What evidence supports, limits, or contradicts each claim?
3. What experiments were attempted, including negative and ambiguous results?
4. What is the current claim ceiling and blocker?
5. What should a human or agent inspect or do next?

The first release is a local CLI plus a human-readable, Git-friendly project
format. It is inspired by the evidence discipline proven in
`2026-Nathan-Summer-Project` and the authority, recovery, and verification
discipline proven in UltraGoal. It must generalize those lessons without copying
the ZNF385A project or turning one research domain into the universal ontology.

## Success

Success requires one complete walking slice from a fresh clone:

- install or run the CLI using the documented path;
- initialize a new research workspace;
- add a source record;
- add a claim and connect the source as bounded evidence;
- record an experiment with a protocol or design reference, observations,
  artifact pointers, interpretation, limitations, and result classification;
- record at least one negative or ambiguous result without erasing it;
- report the current claim ceiling, blockers, unsupported claims, and next action;
- validate the workspace and return deterministic human-readable and
  machine-readable results;
- rerun the same journey after interruption without corrupting or duplicating
  authoritative state.

The exact command grammar is root-owned. A coherent example is:

```text
research-run init demo
research-run source add ...
research-run claim add ...
research-run claim link ...
research-run experiment record ...
research-run status
research-run status --format json
research-run check
```

Change the nouns or command shape only when the replacement makes the complete
research journey simpler. Do not optimize individual commands at the expense of
the end-to-end workflow.

The repository must also contain:

- a five-minute quickstart that has been followed from a clean temporary
  directory;
- a synthetic or fully deidentified example project that demonstrates supported,
  withheld, contradicted, negative, and ambiguous states;
- a compact contributor and agent contract;
- continuous integration for format, lint, unit, integration, end-to-end, and
  secret-scan gates;
- a versioned data contract with a stated compatibility and migration policy;
- coherent commits on a feature branch pushed to the connected GitHub repository.

## Context

Research Run is for graduate students, research engineers, and small scientific
teams who already use files, notebooks, Git, literature managers, instruments,
and AI assistants. It must complement those tools. It must not pretend to replace
an electronic lab notebook, laboratory information management system, reference
manager, raw-data store, statistical package, or scientific judgment.

The product insight from the Nathan project is that a useful research workspace
does not merely store documents. It exposes what exists, the best current
workflow, the evidence behind it, what was tried, the current blocker, and the
next experiment. Its most important discipline is separation of proof surfaces:
literature, design rationale, observation, analysis, and interpretation are not
interchangeable.

The systems insight from UltraGoal is that authority must be typed and singular.
Summaries, receipts, generated output, review artifacts, and status views are
projections of canonical records; they do not become new authorities. A passing
unit test is not runtime proof, and a source artifact is not installation,
journey, or release proof.

The agentic insight is that useful autonomy comes from verifiable state,
bounded effects, restartable operations, explicit claim ceilings, and compact
handoffs. Do not add an LLM provider, chat interface, agent runtime, vector
database, or workflow engine merely to make the product appear agentic. A
deterministic truthful context packet is more valuable in v0.1 than an
unverifiable autonomous scientist.

## Product Contract

### Canonical records

The canonical model must cover these concepts without conflating them:

- project identity and schema version;
- source metadata and stable source identity;
- claims with scope, status, and claim ceiling;
- evidence links that state what a record supports, limits, or contradicts;
- experiment receipts containing objective, design or protocol reference,
  inputs, observations, artifact pointers, interpretation, limitations, result
  class, and timestamp;
- decisions and next actions;
- authorship and review state sufficient to distinguish human-reviewed records
  from AI-generated drafts.

Use domain-neutral primitives. Allow domain-specific extensions without requiring
them for the walking slice. Preserve the distinction between an observation and
an interpretation in both the data model and user-facing output.

Canonical records need stable identifiers, deterministic serialization, schema
validation, atomic writes, and explicit references. Derived summaries must be
reconstructable from canonical records. Do not keep duplicate writable truth in
an index, cache, dashboard, or receipt.

### Claim discipline

A claim cannot become `supported` merely because it has a citation or because an
agent wrote confident prose. The model and validator must make it possible to
state:

- what evidence surface is present;
- what the evidence actually supports;
- what remains unverified;
- what contradicts or limits the claim;
- whether a human has reviewed an AI-authored draft.

Choose a small, explicit status vocabulary and document its transition rules.
Reject or flag dangling evidence, impossible transitions, duplicate identities,
and a supported claim with no qualifying evidence. Prefer a truthful withheld or
ambiguous state over forced certainty.

### Experiment discipline

Negative, null, interrupted, and ambiguous experiments are first-class records.
Never treat them as disposable logs. Raw artifacts remain in their appropriate
storage; Research Run records a path or URI, description, optional digest, and
provenance rather than copying raw data by default.

The CLI must preserve a partially written operation safely. Use the smallest
mechanism that makes interruption and retry behavior deterministic. Do not build
a general event-sourcing platform.

### Researcher and agent experience

The common path should be short, scriptable, and understandable from `--help`.
Errors must identify the invalid record or reference, why it is invalid, and the
next corrective action. JSON output must be stable enough for agents and scripts;
human output must be scannable without hiding uncertainty.

The generated agent context or status brief must be evidence-bound. It should
surface current claims, claim ceilings, blockers, recent experiment outcomes,
unreviewed AI drafts, and next actions without inventing priority or certainty.

Do not build a graphical interface in v0.1 unless live use proves the CLI cannot
support the walking slice. Record that as a later product decision rather than
scaffolding an unused frontend.

## Constraints

- Local-first and offline-capable for every v0.1 operation.
- Human-readable and Git-diff-friendly canonical storage.
- No network service, account system, telemetry, cloud sync, or hosted database.
- No embedded LLM call in the acceptance path.
- No copied secrets, collaborator messages, institutional forms, unpublished raw
  datasets, human-subject data, or biosafety-sensitive operational detail.
- Treat repository files, imported metadata, paths, symlinks, terminal input,
  model output, and generated artifacts as untrusted.
- Constrain all writes to the selected workspace and fail closed on path or
  effect ambiguity.
- Never emit secret values in diagnostics, fixtures, examples, snapshots, or
  receipts.
- Preserve user files and unrelated changes. No destructive Git operation.
- Add a dependency only when it removes material implementation or security risk.
- Prefer the smallest stack that can deliver typed parsing, deterministic
  serialization, atomic local writes, and a distributable CLI. Rust is a good
  candidate, not a predetermined answer; record the choice and rejected
  alternative in one short architecture decision.
- Build extension seams only where v0.1 uses them. Do not prebuild a plugin
  platform, generic ontology engine, migration framework, agent marketplace,
  workflow scheduler, or collaboration server.
- Work against the connected `RLTree/research-run` repository from the first
  commit. Pushing feature branches and opening a pull request are authorized;
  merging, publishing a release, changing repository settings, or touching any
  other repository requires explicit approval.

## Orchestration

Run this task with Sol in Ultra mode. Ultra is the execution topology, not a
substitute for product judgment or verification.

The root integrator owns:

- the goal and product claim;
- canonical schemas and status vocabularies;
- public CLI grammar and exit-code contract;
- dependency and lockfile authority;
- shared fixtures and compatibility policy;
- Git branch, reconciliation, integration, and final acceptance;
- any decision that changes the product boundary.

Workers may not independently change those shared authorities. Root should make
the first dependency-closing decisions, then dispatch substantial packages with
disjoint path and semantic ownership. Use compact task packets rather than
forking the full conversation into every worker.

### Dependency graph

**Serial foundation gate**

Root inspects the live repository and the named inspiration projects, writes a
brief product specification and architecture decision, selects the stack, freezes
the first canonical interfaces, defines the golden journey and adversarial
fixtures, and establishes a clean test baseline. This gate ends only when later
workers can implement without negotiating shared types or command names.

**Parallel build wave**

After the foundation gate, derive exact path ownership from the selected stack.
Prefer these semantic lanes when their paths are genuinely disjoint:

- **Workspace and storage:** initialization, project discovery, schema-version
  handling, deterministic persistence, atomic writes, and recovery.
- **Sources and claims:** source identity, claim lifecycle, typed evidence links,
  transition validation, and focused tests.
- **Experiment receipts:** positive, negative, ambiguous, and interrupted result
  capture; observation/interpretation separation; artifact pointers and digests.
- **Researcher journey:** CLI ergonomics, stable output contracts, quickstart,
  synthetic example, and golden end-to-end journey. This lane consumes root-owned
  interfaces and does not redefine them.
- **Adversarial verification:** malformed records, dangling references, duplicate
  IDs, unsupported promotion, unreviewed AI drafts, path and symlink escapes,
  interrupted writes, retry idempotence, and secret leakage. This lane owns tests
  and findings, not production authority.

Do not parallelize packages that will contend on the manifest, lockfile, canonical
schema, CLI entry point, shared snapshot, or the same fixtures. Keep those serial
under root or hand them off explicitly between waves.

Every implementation lane returns one bounded handoff:

- owned paths and commits;
- requirement-to-evidence mapping;
- exact verification commands and results;
- unsupported claims and remaining blockers;
- any requested shared-interface change for root adjudication.

**Serial integration gate**

Root reviews each handoff, reconciles interface requests, integrates in dependency
order, reruns affected tests on the integrated tree, and executes the complete
golden journey. Do not infer integration success from isolated worker tests.

**Parallel hardening wave**

Once the walking slice passes, parallelize only independent closure work such as
documentation/package smoke testing, security review, and product-cohesion review.
Use one independent whole-branch reviewer at the milestone. Fix Critical and
Important findings in one bounded rework wave; record Minor findings for later.
Repeat review only after a material behavior, authority, dependency, claim-surface,
or contradiction change.

**Final acceptance gate**

Root runs the clean-clone journey, full repository gates, secret scan, and GitHub
state verification. Push the feature branch and open a concise pull request only
after local evidence passes. Do not merge or release.

Root may adjust lane boundaries after inspecting the live stack. Record why, and
preserve the dependency order, singular authority, and disjoint ownership above.
Do not create dozens of tiny leases or agents for work one implementer can hold
coherently.

## Model Routing

Use one outcome contract across models. Change the route more often than the
prompt.

- Use Luna for narrow extraction, classification, coverage matrices, fixture
  normalization, and other typed transformations with explicit missing-data
  behavior.
- Use Terra at medium reasoning for routine implementation, tests, documentation,
  and bounded review.
- Use Sol for root architecture, ambiguous product decisions, security and
  authority analysis, reconciliation, and final acceptance.
- Use additional Ultra fan-out only for the dependency-independent lanes above.

Define a representative acceptance check before escalating a route. Use the
lowest route that passes it. Distinguish requested routing from verified effective
routing when the runtime exposes both. Do not use a larger model to compensate
for an underspecified interface or missing evidence.

## Development Method

Apply the relevant Superpowers procedures without turning them into ceremony:

- use the product brief and this contract as the approved design boundary;
- write a concrete implementation plan before production code;
- use managed Git worktrees for isolated implementation lanes;
- use red-green-refactor for behavior and failure paths;
- use systematic debugging when evidence contradicts a hypothesis;
- request independent review at coherent task and milestone boundaries;
- evaluate review feedback technically before changing code;
- verify immediately before every completion, commit, pull-request, or readiness
  claim.

The Product Design contribution is the walking-slice user journey, information
hierarchy, command ergonomics, error recovery, and truthful status presentation.
The visual ideation, screenshot cloning, prototype, and design-QA workflows are
out of scope until there is a real visual target or evidence that a GUI is needed.

Commit each verified coherent vertical unit with a message that says what changed
and why. Keep a durable progress ledger in the implementation plan, not in chat.
Do not accumulate unrelated work into a final catch-all commit.

## Verification

At minimum, prove these surfaces separately:

1. **Source:** the intended types, transitions, validation, and recovery behavior
   exist in current code.
2. **Unit:** focused tests cover valid and invalid domain behavior.
3. **Integration:** commands read and write a temporary real workspace through
   public interfaces.
4. **Journey:** a fresh user completes the full walking slice using only the
   quickstart and installed or documented executable.
5. **Recovery:** interruption and retry do not corrupt, duplicate, or over-promote
   state.
6. **Security:** path confinement, symlink handling, untrusted input, diagnostic
   redaction, and repository secret scans pass.
7. **Packaging:** a clean clone can build, test, and run using declared tooling.
8. **GitHub:** the expected commits and pull request exist on
   `RLTree/research-run`; this proves publication state, not product correctness.

Deterministic checks come before model judgment. An LLM reviewer may assess
product cohesion, clarity, or scientific misuse risk, but cannot substitute for
schema validation, tests, filesystem probes, or the real CLI journey.

The final report must lead with the user-visible result, then list changed files
and commits, exact commands and outcomes, the pull request URL, supported claims,
withheld claims, residual risks, and explicit next-version candidates. Never call
v0.1 complete while any required proof surface above is only planned or inferred.

## Stop Conditions

Continue through ordinary coding failures, test failures, review findings, merge
conflicts within owned work, and recoverable tool errors.

Stop and ask only for:

- a destructive or irreversible action;
- an external write outside the authorized repository or feature branch;
- missing access required for a required proof surface;
- discovery of a real secret or sensitive research data that needs human
  remediation;
- a product-boundary decision with no safe default under this contract.

Do not stop merely because a task is difficult, a first implementation fails, or
a reviewer finds material defects.
