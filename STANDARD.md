# Research Run Standard

## Authority and claim ceiling

Canonical, versioned JSON records under `.research-run/` are the only product
authority. Human and JSON status are deterministic projections. Evidence links
describe relationships but do not promote claims. Only an explicit human review
decision may set a claim assessment to `supported`, `limited`, `contradicted`, or
`unsupported`. `supported` means reviewed support within this workspace and
scope—not general scientific truth.

## Current stack

Research Run v0.1 is a correctness-critical Rust CLI on the pinned stable 1.95.0
toolchain. Cargo is the build, test, package, and distribution substrate. The
runtime dependency budget is deliberately small:

- `clap`: typed public CLI parsing and generated help;
- `serde`: typed domain serialization and deserialization;
- `serde_json`: deterministic, human-readable canonical JSON.

No async runtime, database, network client, logging/telemetry stack, temporary
file crate, schema framework, or UI dependency is justified by the v0.1 job.
Standard-library filesystem primitives provide bounded reads and atomic same-
directory publication.

## Product and security invariants

- Parse before effects; reject unknown fields and versions.
- Stable IDs are lowercase ASCII slugs and filenames must match IDs.
- Reject absolute workspace pointers, parent traversal, and symlinks in workspace
  authority paths. Raw data is referenced, not copied.
- Publication uses a new same-directory temporary file, flush and `sync_all`, an
  atomic no-clobber hard link, cleanup, then directory sync. Identical retry is a
  no-op; conflicting identity fails closed. Startup/validation removes no
  ambiguous state automatically; explicit `recover` validates pending content
  before completing or reconciling an interrupted publication.
- Each record is at most 1 MiB; canonical collections are bounded to 10,000
  records per kind; user text is bounded; status is derived in deterministic ID
  order. These are safety and memory/storage budgets, not scale claims.
- Diagnostics name record paths and invariant failures but never echo record
  bodies, environment variables, or secrets.

## Required gates

Run from the repository root:

```console
cargo fmt --check
cargo check --locked --all-targets
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo package --locked --allow-dirty
cargo audit
cargo deny check
gitleaks detect --source . --no-banner --redact
```

`cargo audit`, `cargo deny`, and `gitleaks` are stronger local gates when the
tools are available; CI runs deterministic Cargo gates and a repository secret
scan. Packaging, install smoke, the public CLI journey, GitHub branch/PR state,
release, and real-user use remain separate proof surfaces.
