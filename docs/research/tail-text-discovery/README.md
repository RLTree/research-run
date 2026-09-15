# Tail-text discovery experiment

Opportunity: `RROPP-tail-text-discovery`.

**Outcome:** [Retain recommendation, checks and holds](results.md).

## Scope and reproduction

This is a bounded local source experiment based on
`71ad5fd8f0533a7ab34b35ffeb3f86d88a92a2c5`. Remote default `main` and the
clean detached worktree matched before edits. No commits, pushes, installation,
release, settings, automation, or live canonical research changes are authorized.

The baseline release executable is retained at
`target/tail-text-evidence/baseline`; the candidate at
`target/tail-text-evidence/candidate`. Both use the same locked dependencies,
release profile, pinned Rust toolchain, and host. Rust build/check commands are
serialized with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_TEST_DEBUG=0`; test execution uses one test thread.

`verify.py` exercises actual baseline and candidate executables, including both
commands, handoff create/inspect in both reader directions, synthetic v1
compatibility, validation, unchanged projections, and canonical byte digests.
It uses only the synthetic `red-workspace` retained beside the binaries. The
baseline reproduced prefix=1, tail search=0, tail context=0 before source edits.
The candidate must produce both tail hits without altering existing matches.

`measure.py synthetic` creates two synthetic workspaces through CLI initialization
and typed canonical fixture construction, validates them, and measures both
commands. `measure.py PATH` reads an existing research workspace without writing
its canonical state. It uses the longest knowledge body's first/last 16
characters, an absent control and a common term. Query strings, record IDs,
scientific content and command outputs are not retained in timing evidence.

For each workload, command and query class, timing records the first invocation
separately, three warmup pairs and 15 alternating baseline/candidate pairs. Pair
order alternates to reduce order bias. p95 is the nearest-rank 15th observation
(the maximum at this sample count). Timing includes process launch, complete
snapshot validation/projection, and captured output. First invocations have
uncontrolled filesystem-cache state; no cold-cache guarantee or cache flushing
is claimed. Snapshot validation during fixture setup may already warm caches.
Output bytes and returned/total counts expose the unequal work of newly found
matches. Canonical digests are compared before and after each workload.

The provisional review triggers are warm median increasing by both >20% and
>25 ms, or p95 increasing by both >25% and >50 ms. These are experiment review
triggers, not user tolerances or service guarantees. Peak resident memory was
attempted separately with macOS `/usr/bin/time -l`; the host denies
`sysctl kern.clockrate`. The final run instead uses a fresh Python parent for
one CLI subprocess and reads `resource.getrusage(RUSAGE_CHILDREN).ru_maxrss`
(macOS bytes). These separate peak-memory observations are outside timing pairs.

Measurement helpers operate on a quiescent, cooperative workspace. They reject
static symlinks, path escape and special files before reading, then use
no-follow, nonblocking regular-file descriptors with `fstat`. They do not defeat
an adversarial concurrent replacement of a parent directory between validation
and open; no universal concurrent-writer confinement claim is made.
The helper's `entry-test` exercises the actual command-line path under normal
and optimized Python against workspace-root, ancestor, state-root and dangling
symlink inputs, and confirms no timing evidence is written on rejection.
Reads also enforce the existing canonical budgets: 1 MiB ordinary records,
32 MiB inventory records and 64 MiB per complete state pass, including a
one-byte sentinel for files that grow after metadata inspection.

Current synthetic evidence records qualification status per query. The
many-short tail control is `NOT_APPLICABLE` because the baseline result set is
saturated with 1,000 prefix-visible records, so tail absence cannot be proven
at the bounded limit. The near-budget tail control remains qualified; historical
performance tables retain their original candidate-stage scope.

## Implementation

`retrieval_match` shares one matcher between search and queried context. It
preserves existing projection matches, then falls back to complete knowledge
bodies borrowed from the same snapshot using typed kind plus ID. The full body
is lowercased before substring matching. Normalized offsets map back to original
Unicode scalars; expanding U+0130 and contextual Greek sigma retain original
spelling. ASCII bodies use equivalent direct offsets/slices. Excerpts reserve
space for omission ellipses within 512 scalars. A long combining-mark run can
change sigma's lowercase form when the excerpt is isolated; the complete body
remains the match authority.

Wire fields, five match labels, handoff versions and digest construction remain
unchanged. Body fallback uses `matched_by: ["summary"]`; human output explains
`summary (knowledge body)`. No index, dependency, schema migration or canonical
reread was added.

## Investigation and independent review

The first complete synthetic timing run found near-budget tail search at about
74 ms baseline versus 200 ms candidate. Mapping every ASCII character through
Unicode scalar lowercase dominated the added work. The equivalent ASCII path
removed this cost; both complete synthetic workloads were measured again for
the final executable. Initial raw timings are retained separately.

One targeted independent reviewer examined matching, Unicode excerpts and
handoff compatibility. The review found missing explicit preservation tests for
shared IDs across kinds and ID/title matches also present in tail bodies. Those
cases were added and passed. The same reviewer checked that repair and the
ASCII optimization and found no material source issue. Static review does not
replace the executable compatibility, timing or coverage results.

The human label `summary (knowledge body)` describes the source field for both
projected prefix summaries and fallback excerpts. It does not claim that the
match was necessarily a fallback; the JSON reason remains `summary`.

The broad source check caught a 251-line workspace router after the new module
declaration. Nesting the matcher under retrieval restored the 250-line cap
without changing its behavior. The standards gate also required keeping exactly
one named active ExecPlan; this experiment is now linked from that plan. Its
Git-directory scratch files required sandbox approval, after which standards
passed. No gate, threshold or historical evidence was weakened.

## Proof limits

`scripts/check fast` rejects the dirty candidate at entry. Its executable
clean-commit requirement differs from STANDARD.md's dirty-source description;
neither the gate nor the policy was weakened. Underlying formatting, compile,
repository-authority and standards checks are run separately. Full aggregate
and clean-commit package/install gates remain held because commits are outside
this experiment's authority. The broad historical mutation campaign is not
rerun. Source-built CLI evidence is not package, installed, release, universal
Unicode performance, user usefulness or scientific evidence.
