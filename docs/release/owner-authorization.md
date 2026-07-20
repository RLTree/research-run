# Owner Release-Candidate Authorization

This ceremony authorizes one exact candidate for material review. It does not
authorize merge, tag, publication, personhood, Product Fitness, or scientific
truth. Run it from a clean durable worktree; keep the private key and approval
outside Research Run and the agent process.

1. Choose the canonical comment-free Ed25519 public key and authority slug.
   Store the public key in durable repository Git metadata, for example
   `.git/release-evidence/owner.pub`. Never store the private key in the repo.
2. Freeze a clean candidate and inspect its commit, tree, version, and lockfile
   digest. Generate the canonical request:

   ```console
   git status --short
   git rev-parse HEAD 'HEAD^{tree}'
   scripts/prepare-release-authorization \
     "$(git rev-parse --git-common-dir)/release-evidence/owner.pub" \
     repository-owner > request.json
   scripts/validate-canonical-json.py request.json
   jq . request.json
   ```

3. Confirm `publishing_authorized` and all overbroad claims are false. Approve
   the detached signature through the separately controlled SSH agent:

   ```console
   ssh-keygen -Y sign -f owner.pub -n research-run-release-v1 request.json
   ```

4. Retain `request.json` and `request.json.sig` beneath
   `.git/release-evidence/<candidate>/`, then verify using absolute paths:

   ```console
   scripts/verify-release-authorization \
     "$PWD/.git/release-evidence/<candidate>/request.json" \
     "$PWD/.git/release-evidence/<candidate>/request.json.sig" \
     "$PWD/.git/release-evidence/owner.pub" repository-owner
   ```

Any tracked-byte, commit, tree, version, lockfile, request, signature, or key
change invalidates the receipt. Freeze the new candidate, generate and inspect a
new request, approve a new signature, and verify again. A failed or interrupted
attempt is safe to retry because verification snapshots bounded inputs before
parsing, signature verification, and hashing. Update, uninstall, publication
channel, and rollback remain deferred to the later publication contract.
