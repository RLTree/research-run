# Security

## Supported version

Security fixes currently target the unreleased `0.1.x` line.

## Reporting

Report suspected vulnerabilities privately through
[GitHub Security Advisories](https://github.com/RLTree/research-run/security/advisories/new).
Do not include private keys, unpublished human-subject data, raw research
datasets, or other sensitive material beyond the minimum needed to reproduce
the issue.

## Product boundary

Research Run is an offline local CLI. It does not provide network transport,
account authentication, remote synchronization, or secret storage. The review
authority is a public key; private-key custody and each signing approval remain
outside the product process.
