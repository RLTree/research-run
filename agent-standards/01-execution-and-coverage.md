# Execution And Coverage

- Use one outcome contract and one agent by default. Add coordination only for
  substantial independent streams with free integration capacity.
- Build an operator-visible vertical slice before adding machinery.
- Every behavior change needs an executable test at the affected boundary.
- Use `scripts/check fast` for iteration and `scripts/check full` only when a
  current source claim can move. Keep ordinary output ephemeral.
- `scripts/check coverage` is the source-coverage authority. Its 100% line,
  function, and region thresholds are required for an exact material
  source-coverage claim. It instruments the single `product` integration
  target so each production region is counted once; unit tests, nextest, and
  doctests remain separate correctness gates. The filename filter removes test
  sources only and leaves every production file in the denominator.
  Passing tests without that threshold is not coverage proof.
- Do not add tests, exclusions, or unreachable branches merely to inflate
  coverage. Delete or refactor dead behavior and test representative failures.
- Keep the active ExecPlan and affected docs current. A stale authority surface
  lowers or blocks its related claim.
