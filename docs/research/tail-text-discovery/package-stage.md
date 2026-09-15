# Approved local package verification stage

Terry approved a feature branch, local commits, the measured package identity
update and isolated package verification. Push, PR, merge, live installation,
release, canonical research changes and monitor closure remain outside scope.

The reviewed implementation/evidence pack began at
`b16f1cbb708ab8b6d43574b5ea75cd71d135e554`; the current review-fix candidate is
`6eea87ca59f62099690a44b6b2d010a657d40315` on `codex/tail-text-discovery`.
The earlier experiment report and manifest entries remain historical where they
name their preceding candidate.

## Measured declaration

`cargo package --locked` from the final review-fix candidate is checked by the
external completion receipt, including its archive SHA-256 and embedded
`.cargo_vcs_info.json` commit identity. An isolated
`cargo install --path ... --locked` built the extracted package. The installed
executable passed `codesign --verify --strict` and measured:

- Raw executable: 2,822,352 bytes; SHA-256
  `a32317f5fb2bf4341e15425a58084e6c1f4bd39994163446991f020e2edfc3d3`.
- After removing the ad-hoc signature and zeroing Mach-O LC_UUID:
  2,800,312 bytes; SHA-256
  `90eb4d2e130b0c6ce930d5cbd41761b9074dfc602444b855433c65e39b4af987`.

This measurement supplies the tracked expected identity. It is not final
candidate verification: the declaration and gate extension must first be
committed, then the unchanged identity guards must pass against a fresh isolated
package/install run from that final clean candidate.

## Final verification route

Run `scripts/check fast` and separate `scripts/check artifacts` serially with
`CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0` and serialized
tests. The artifact gate now seeds a synthetic tail-only term beyond the prefix
projection, verifies installed search and queried context, creates and inspects
a tail-query v2 handoff, and compares canonical fingerprints before/after.
Its existing identity, signature, installed Append journey, validation and
handoff guards remain in force. The gate receipt includes the tail handoff
digest and the completed tail checks.

Preserve final commit/tree, command statuses, archive/binary identity, gate
receipt and cleanup observations outside tracked frozen bytes under
`target/tail-text-package-proof/`. The gate's disposable install/workspace is
removed by its existing trap. Retain failed attempts separately. Do not edit
this tracked declaration merely to record a later successful run; use the
external receipt bound to its clean candidate.

Repository-wide 100% coverage remains HOLD (511 missed regions, 156 lines and
48 functions, outside the four changed production files). Separate package
verification does not waive aggregate coverage, mutation, release or real-use
fitness gates.

## PR review corrections

After publication, CodeRabbit identified an identity-selection defect reachable
from body matches: queried context could select a relationship by ID while
ignoring endpoint kind. The fix compares typed `(kind, id)` pairs and adds
outgoing/incoming knowledge relationships plus a source-ID collision regression.
The same review identified benchmark evidence drift and symlink/FIFO risks in
the local measurement helpers. Those helpers now validate every expected
synthetic record, reject unexpected or modified corpus entries before reuse, and
use confined no-follow, nonblocking regular-file reads. Historical measurements
remain preserved; refreshed synthetic outputs are separately hash-bound in the
manifest. The human label `summary (knowledge body)` remains intentionally
source-field semantics for both projected and fallback knowledge matches, so no
hidden serialized discriminator was added.

## Portability correction before final gate

The clean-gate preparation found host-specific checkout paths in six archived
experiment logs. Those paths are now represented as `<worktree>` without
changing command outcomes, and the evidence manifest hashes are refreshed.
The original exact bytes and manifest remain in the first implementation commit.
This repairs the existing authority-portability rule; it does not relax it.

The subsequent traced artifact run built and signed the expected-size binary,
then stopped in the existing `otool | awk` normalization pipeline. Early `awk`
exit left the producer with a broken pipe under `pipefail`. The extractor now
records the first UUID offset while consuming all output. It retains the same
offset calculation and missing-UUID rejection, without suppressing pipeline
errors or changing the expected identity.
