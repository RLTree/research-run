# Exact-Candidate Proof Manifest

Candidate-bound evidence is durable but untracked so recording it cannot change
the commit it proves. Store it under the Git common directory at
`.git/release-evidence/<40-hex-candidate>/` in the durable repository checkout.
Never place unique evidence in a temporary filesystem.

`manifest.json` records the candidate commit and tree, creation time, and one
entry per proof surface. Each entry names its status, command, regular-file path
relative to the candidate directory, SHA-256 digest, claim surface, and whether
the receipt is candidate-, tree-, or source-input-bound. Required surfaces are
source tests, standards, coverage, dependencies, secrets, workflow lint,
bounded mutation, package/install, release authorization, four independent
material reviews, and settled GitHub checks.

Every receipt file must be a bounded regular non-symlink file. Any tracked-byte,
commit, tree, tool-contract, key, request, or receipt change invalidates the
affected entry; a commit/tree change invalidates the whole manifest unless an
individual gate has an explicitly reviewed source-input binding. Missing or
stale entries remain withheld and must never be inferred from the ExecPlan,
terminal history, a green aggregate badge, or prose.
