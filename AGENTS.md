# Research Run Agent Router

This file stays compact. For non-trivial work, read only the routed authority
needed for the task:

1. `STANDARD.md` and `policy.toml` for product law and executable acceptance;
   `ARCHITECTURE.md` for command, domain, storage, and proof boundaries.
2. `AGENT_STANDARDS.md` for workflow routing.
   `agent-standards/obligations.json` and `scripts/check-standards` are the
   machine-readable closure and namespace authority.
3. The active ExecPlan under `docs/exec-plans/active/` and
   `docs/research/2026-07-17-v0.1-evidence-and-decisions.md` for current scope,
   decisions, and claim ceiling.

Hard rules:

- Preserve one semantic authority: parse typed records, validate references and
  confinement, then publish canonical state atomically.
- Only an explicit human `ReviewDecision` may promote a claim assessment.
  Status, receipts, examples, generated prose, evidence, and retries cannot.
- Keep observations distinct from interpretation; retain negative and ambiguous
  outcomes.
- Fail closed on malformed input, unknown versions or references, symlinks,
  path escape, ambiguous prior effects, exhausted budgets, and identity conflict.
- Keep source, package, install, cache, discovery, runtime, journey, GitHub,
  release, and real-use proof distinct. Validation never proves scientific truth.
- Rust is the v0.1 product language. New services, UI, databases, telemetry, LLM
  providers, or scale machinery require a measured trigger and approved contract.
- Preserve unrelated work. Use coherent feature-branch commits. Do not merge or
  release. Run the routed checks before moving a claim.
