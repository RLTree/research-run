# Approved local package verification stage

Terry approved a feature branch, local commits, the measured package identity
update and isolated package verification. Push, PR, merge, live installation,
release, canonical research changes and monitor closure remain outside scope.

The reviewed implementation/evidence pack is committed as
`b16f1cbb708ab8b6d43574b5ea75cd71d135e554` on
`codex/tail-text-discovery` (tree
`a365df9d31576ea70da052d0cb6077436f325274`). The earlier experiment report and
manifest describe the preceding uncommitted source stage and remain historical.

## Measured declaration

`cargo package --locked` from that clean commit produced archive SHA-256
`0ff39e1aed8ff8decc2fec48d60d9a4434c78f17966515fb2e9eae31b485cc3e`.
Its embedded `.cargo_vcs_info.json` identifies that same commit. An isolated
`cargo install --path ... --locked` built the extracted package. The installed
executable passed `codesign --verify --strict` and measured:

- Raw executable: 2,822,224 bytes; SHA-256
  `641570301822aafe701f92c835292bab74d44a20553999a8d3cf4cacaa6eb7de`.
- After removing the ad-hoc signature and zeroing Mach-O LC_UUID:
  2,800,192 bytes; SHA-256
  `1b061fa5b2e657d3f34233fb2a7fefb600cd1a2475cf6539726d96cd0816f505`.

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
