# Execution And Coverage

- Use one outcome contract and one agent by default. Add coordination only for
  substantial independent streams with free integration capacity.
- Build an operator-visible vertical slice before adding machinery.
- Every behavior change needs an executable test at the affected boundary.
- Use `scripts/check fast` for iteration and `scripts/check full` only when a
  current source claim can move. Keep ordinary output ephemeral.
- `scripts/check coverage` is the source-coverage authority. Its current 100%
  line threshold is required for an exact material source-coverage claim.
  Passing tests without that threshold is not coverage proof.
- Do not add tests, exclusions, or unreachable branches merely to inflate
  coverage. Delete or refactor dead behavior and test representative failures.
- Keep the active ExecPlan and affected docs current. A stale authority surface
  lowers or blocks its related claim.
