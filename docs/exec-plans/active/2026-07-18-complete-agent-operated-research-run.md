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
- Current delivery branch: `codex/pr-review-automation`, tracking PR #7 at
  `origin/codex/pr-review-automation`. The exact candidate is always the clean
  Git `HEAD` containing this plan and is resolved from Git at each proof gate;
  this plan does not embed its own impossible self-referential commit hash.

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
- 2026-07-18: The deterministic Gold Stack route returned `triggered: false` for
  the established Rust CLI, and retrofit preview proposed no stack mutation.
  No generated-profile or installed-profile claim is active.
- 2026-07-18: Implemented byte-preserving retrofit/reconciliation, typed
  knowledge and relationships, bounded retrieval, portable handoff, and lossless
  v0.1 migration in coherent commits through `209b12c`.
- 2026-07-18: Current source proof records 113/113 normal tests, 6,293/6,293
  unique production regions, 3,217/3,217 executable production lines, 413/413
  production functions, and 116/116 viable selected mutants killed; package,
  install, benchmark, review, GitHub, and completion claims remain pending.
- 2026-07-18: The requested pre-implementation four-persona round was not run
  before broad implementation. That milestone signoff is permanently withheld;
  final exact-candidate review remains required and cannot retroactively replace
  it.
- 2026-07-18: Clean candidate `d0f58cb7c6a2bb30fd80e13dc28d40b292860195`
  packaged 174 files (142,749-byte archive), installed in isolation, and passed
  the installed journey. The binary was 2,004,112 bytes; warm averages were
  2.4 ms first-use, 3.4 ms status, and 4.2 ms recovery on this machine. These are
  single-machine observations, not regression budgets.
- 2026-07-18: The disposable Nathan benchmark indexed 355 files / 117,816,938
  bytes in 0.24 s and applied in 0.27 s. The non-Research-Run byte digest stayed
  `b31884eaf00e682ae725329d926eb1d8fb99c2546522a1cb3ac88e290d233d94`;
  repeat apply returned `created: false`; the ZNF385A query returned 13 bounded
  matches; and a fresh directory validated the 13-item portable handoff without
  access to the source workspace. No research bodies were retained.
- 2026-07-18: Gold Stack kernel
  `b030c0e2b37509fead964a85e307cac3305c6655266ad63928635505f3a0cb45`
  returned `triggered: false`, `boundary: none`, no profiles, and no topics for
  the established Rust stack. Retrofit preview reported no stack decision
  boundary. No profile was applied; `check-profile` correctly reported the
  absent generated profile, so profile-materialization/check claims are
  inapplicable rather than passed.
- 2026-07-18: The live UltraGoal checkout is dirty and was preserved. A clean
  detached worktree at exact source commit
  `0355039bf621113e7089c235a298c7a8b085397f` built the CLI through locked offline
  Cargo. Canonical fit inspection and planning against clean candidate `80657ca`
  found 67 missing generated files and four conflicts with Research Run-owned
  authority (`AGENTS.md`, `AGENT_STANDARDS.md`, `ARCHITECTURE.md`, and
  `scripts/check`), plan digest
  `sha256:71dd2b2a2bb8246586596de234d9e5cf5b53827ea15446de78b0a0ba4efdd7a2`.
  The production adapter refuses conflicting plans before effects, so no apply
  was attempted. Source-built inspect/plan proof is current; package, install,
  discovery, runtime activation, applied fit, fit receipt, and full fitted
  governance remain withheld.
- 2026-07-18: Final-review adversarial findings against candidate `181411a`
  rejected completion. The coherent repair now binds reviews to a SHA-256 digest
  of the exact claim, evidence, referenced source or experiment, and artifact
  bytes; stale same-ID authority no longer promotes a claim or appears current
  in retrieval and handoff projections.
- 2026-07-18: The repair also rechecks inventory and migration authority under
  their write locks, rejects structured-input identity and length races,
  validates Gregorian calendar dates, deeply validates portable handoffs,
  derives claim blockers and next actions, and makes standards classifications
  exact and tamper-tested. The superseded kickoff moved to
  `docs/exec-plans/completed/`; this is the sole active plan.
- 2026-07-18: Pre-commit repair proof passes 132/132 normal nextest cases,
  7,077/7,077 unique production regions, 3,576/3,576 executable production
  lines, 455/455 production functions, and bounded mutation with 180 viable
  mutants killed, 18 compiler-unviable, and zero missed or timed out. Format,
  check, doc tests, Clippy, standards, dependency audit/deny, gitleaks, and
  actionlint also pass. Package, install, benchmark, final review, GitHub, and
  completion proof must be refreshed after the repair commit freezes exact
  bytes.
- 2026-07-18: A typed Product Fitness disposition withholds real-use fitness,
  daily-driver usefulness, continuance, and research-impact claims because no
  audience-bound human-use evidence exists. Source tests, installed journeys,
  package proof, benchmarks, and generic reviewer approval are explicitly
  non-substitutes.
- 2026-07-18: Human-only claim promotion remains an external authority blocker.
  This repository has no configured user-controlled signing or approval
  mechanism that can distinguish a human decision from an agent with the same
  filesystem and CLI authority. The product enforces explicit typed review and
  exact semantic binding, but human provenance and final human signoff remain
  withheld until that trust boundary is supplied.
- 2026-07-19: Candidate `33c2725` changed only the mutation-runner custody
  controls after the current production-source test repair. Its mutation command
  uses Cargo's test runner with one outer mutation worker and `cargo -j 1`, with
  the mutation-runner jobserver disabled. Separate live process checks observed
  one mutation parent, one Cargo child, and one `rustc` child at a time. The run
  ended before its selected set completed: its outcome ledger contains the
  passing baseline, 108 caught mutants, and 4 compiler-unviable mutants. This
  is incomplete mutation evidence, not a zero-survivor or package-delta claim;
  it must not be restarted without an explicit new decision boundary.
- 2026-07-19: The same clean candidate passed `scripts/check fast` and an
  isolated `scripts/check artifacts` package, install, installed-binary
  journey, and machine-local observation. The package archive was 157,104
  bytes, the installed binary was 2,036,848 bytes, and the installed footprint
  was 2,000 KiB. These observations remain machine-local only. They do not
  substitute for the incomplete mutation gate, final independent review, human
  provenance, or audience-bound Product Fitness proof.
- 2026-07-19: PR #4 merged candidate `33c2725` into `main` as merge commit
  `7b18fe8` outside this run's merge authority. The merged branch has no open
  PR or remaining delta. A subsequent follow-up PR may record this state, but
  cannot retroactively make the original delivery's open-PR condition true.
- 2026-07-19: Codex Security deep scan
  `194a70ad-2bda-4248-bb21-04d5abf3ec14` completed against immutable revision
  `acee890a4dcc9c1377a954961e782b8f685a5b58` after six discovery passes and
  642 independent review receipts. Validation and attack-path analysis retained
  six reportable findings: five bounded recovery/projection defects and one
  external human-authorization boundary.
- 2026-07-19: The bounded security repair on PR #7 rejects cyclic recovered
  history, recomputes recovered inventory and migration authority before any
  effect, validates an existing manifest before recovery effects, and rejects
  duplicate current review projections. The four original dynamic exploits,
  correct-identity forged-content variants, all 51 product tests, the fast gate,
  standards, formatting, and Clippy pass under serial single-job Rust
  verification. Mutation and coverage were not rerun because no new named
  decision boundary has authorized those resource-intensive surfaces.
- 2026-07-19: The human-review boundary now uses one immutable Ed25519 public
  key anchored during owner-controlled workspace initialization. Review
  preparation emits a canonical request bound to a random 256-bit workspace ID;
  publication, recovery, validation, and status require an exact detached
  `research-run-review-v1` SSHSIG. Post-initialization enrollment and rotation
  are absent, legacy unanchored workspaces cannot promote, and same-name
  cross-workspace replay fails. The provisioned 1Password-backed signer has
  public fingerprint `SHA256:HUFBTnkE5jqHd3zeT8AKDlk65I7HyMBAMc/cwfCwKbo`;
  its private key was not exposed to this run. Cryptography proves control of
  the configured key, not human personhood, and no agent-run signing is treated
  as human approval.
- 2026-07-19: Independent source-only review first found caller-selected
  enrollment and same-name replay, then request-identity and interrupted-init
  retry gaps. The repaired design anchors authority only at initialization,
  compares the submitted workspace identity, and idempotently republishes only
  the exact manifest anchor after interruption. The final independent recheck
  reported no actionable findings.
- 2026-07-19: The signed-authority candidate passes 115/115 library tests,
  171/171 normal nextest cases, semantic-tree standards, and exact production
  coverage at 8,110/8,110 regions, 4,119/4,119 lines, and 525/525 functions.
  Formatting, `cargo check`, doc tests, and Clippy also pass under constrained
  single-job builds. Dependency, repository, mutation, package, GitHub, and
  completion proof still require their final frozen-candidate boundaries.
- 2026-07-20: Material review found and the current repair closes surplus
  review-authority recovery, irreversible unanchored init/retrofit ambiguity,
  legacy unsigned-v0.1 compatibility, hidden authority status, incomplete
  mutation selection, stale UltraGoal fit language, under-specified Product
  Fitness withholding, and stale or invisible Codex PR-review reporting. The
  inventory apply command now returns its authority disposition from the same
  transaction instead of performing fallible post-effect rediscovery.
- 2026-07-20: The current pre-commit repair passes 121/121 library tests,
  54/54 normal product tests, 177/177 serial nextest cases, doc tests,
  formatting, check, Clippy, semantic-tree and package-law standards,
  dependency audit/deny and inventory validation, gitleaks, and actionlint.
  Exact authoritative production coverage is 8,352/8,352 regions,
  4,255/4,255 lines, and 535/535 functions. Package/install, the single bounded
  final mutation run, exact-commit review, push, and GitHub proof remain
  pending.
- 2026-07-20: Clean candidate `7607c23` passed package, isolated install,
  installed signing journey, and machine-local artifact observation. Its
  bounded one-worker/one-Cargo-job mutation run completed normally in 46
  minutes: 271 mutants evaluated, 233 caught, 33 compiler-unviable, zero timed
  out, and five survived. The survivors exposed missing independent checks for
  initialized-authority equality, manifest ID/fingerprint matching, SSHSIG
  namespace/reserved fields, and unanchored human-claim guidance. Focused
  regressions were added without changing runtime behavior; mutation closure
  remains withheld until those exact survivors are killed on the clean
  test-repair candidate.
- 2026-07-20: Exact candidate `abfe80b` closed all five prior survivors in a
  focused 22-mutant delta: 16 viable mutants caught, six compiler-unviable,
  zero missed or timed out. Exact coverage, normal-mode gates, dependency and
  secret checks, and package/install mechanics passed on that clean candidate.
  Final review then found an inventory plan race: a workspace created or whose
  review authority changed after planning could be accepted before inventory
  publication. The current repair binds plans to immutable workspace identity,
  verifies the full authority snapshot under lock before effects, and derives
  apply status only from that verified snapshot. The follow-up security review
  found two narrower variants: existing-workspace plans did not carry the
  plan-time authority, and initialization retries checked an appeared
  manifest's name and key but not its supplied workspace identity before
  creating subdirectories. The repair now binds every newly produced plan to
  the exact anchored key or explicit unanchored mode, validates an appeared
  workspace identity under lock before initialization effects, and retains
  legacy `(None, false)` acceptance only for an unanchored target. Dedicated
  no-effect race fixtures pass. Exact production coverage is 8,477/8,477
  regions, 4,328/4,328 lines, and 542/542 functions.
- 2026-07-20: The pending Product Fitness candidate declaration resolves
  `git:HEAD` only from a clean proof-gate worktree and expects the
  `aarch64-apple-darwin` installed binary content digest
  `ab5745af96a9aa588f8a4db34223c59d8db5431c3f4b38d5b66144a45398be7c`
  after first verifying and binding the exact empty-entitlement ad-hoc,
  linker-signed profile, then removing that code-signature blob and zeroing the
  nondeterministic Mach-O `LC_UUID` (2,253,600 normalized bytes; 2,271,376 raw
  bytes). The canonical signature-profile digest is
  `a21345461360d035ba5e371bd1d91c5ea98f60cfd8850a2bc157e3d4f01219bd`.
  `scripts/check-product-artifacts` must reproduce and verify these values from
  a clean exact `HEAD` while completing the installed journey; until that
  command succeeds, the values are a declaration rather than proof. The raw
  per-build hash remains observable in the receipt but is not misrepresented as
  stable; the normalized digest is platform-qualified, not a cross-platform
  reproducibility claim. This proves installed mechanics only. Accessibility,
  cognitive load, recovery burden, continuance, audience-bound real use,
  research impact, and actual owner-approved signing remain withheld.
- 2026-07-20: The first post-repair targeted mutation run at `2398903`
  exercised 52 mutants: 40 caught, ten compiler-unviable, two missed, and zero
  timed out. Both survivors were distinct legacy workspace-ID boolean
  combinations in `InventoryPlan::validate`; exact regression cases are now
  present and their focused rerun remains pending on the next clean candidate.
  Security review also found that normalized Mach-O content cannot substitute
  for runtime-signature authority. The artifact gate now verifies and binds the
  valid ad-hoc linker-signature profile, empty entitlements, absent Team ID and
  authorities, absent internal requirements, and expected CodeDirectory flags
  before removing only that verified signature for content normalization.
- 2026-07-20: PR review exposed an irreversible retrofit bootstrap gap before
  inventory publication. The repair now serializes bootstrap-state
  classification under the workspace write lock, accepts only an empty or
  exact-plan pending scaffold, validates the canonical marker before cleanup,
  preserves conflicting pending evidence, and prevents a losing plan from
  republishing after a concurrent winner completes. Follow-up review required
  strict unknown-field rejection for the durable marker and exact-plan recovery
  guidance for a post-bootstrap publication-lock failure. Those repairs retain
  ordinary existing-workspace errors as a distinct class. Exact production
  coverage is 8,842/8,842 regions, 4,506/4,506 lines, and 561/561 functions.
  The final named bootstrap mutation boundary exercised 49 mutants: 41 caught,
  eight compiler-unviable, zero missed, and zero timed out.
- 2026-07-20: Current-head Codex review found two pre-effect authority gaps.
  Inventory apply now parses and fingerprint-checks an embedded authority before
  scanning or creating bootstrap state, and review preparation validates both
  the enrolled key and immutable manifest anchor before producing a signable
  request. Red fixtures cover malformed plan keys with no `.research-run`
  effect and tampered preparation keys and anchors. The focused call-site
  mutation boundary exercised five mutants: three caught, two
  compiler-unviable, zero missed, and zero timed out.

## Current claim ceiling

No completion claim is active for this plan. Exact-current package/install,
installed journey, and targeted mutation proof remain pending for the clean
candidate resolved as `git:HEAD`; the `7607c23` full mutation run and `abfe80b`
survivor delta are historical inputs only and cannot substitute for the
post-repair delta. The clean candidate may claim only the exact local source,
dependency/security, standards, and coverage observations already run against
its unchanged production bytes. GitHub and CodeRabbit proof remain pending
until the branch is pushed and the PR settles. The product has an external
configured-key authorization mechanism and a separately provisioned 1Password
signer. That closes the self-asserted-review defect at the source boundary; it
does not prove personhood, owner bootstrap, private-key custody, or an actual
human-approved signing journey. The prior UltraGoal fit record is historical
and non-authoritative; current fit classification, applied fit, receipt, and
fitted-governance readiness remain withheld because no current reprobe has run.
Audience-bound Product Fitness proof remains withheld on its own surface.
Release, scientific truth, scientific impact, continuance, and unobserved real
researcher usefulness are outside this run.
