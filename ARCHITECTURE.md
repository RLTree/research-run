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
claim, evidence, experiment, and human-review JSON records. A review decision is
the only semantic authority that changes a claim assessment.

## Proof and governance

- `tests/product.rs` routes product journeys, failure/recovery behavior, and
  repository-law tests.
- `scripts/check*` owns fast, full, coverage, standards, dependency, mutation,
  package/install, and observation commands.
- `AGENTS.md` routes the current contract; `STANDARD.md`, `policy.toml`, and
  `agent-standards/obligations.json` hold human and machine-readable law.
- `schemas/v1/` and `examples/` describe the current portable data contract.

The repository has no network runtime, service, database, async executor,
telemetry backend, user interface, Python product code, or release machinery.
Those absences are architectural constraints until a measured product need and
new authority contract justify them.
