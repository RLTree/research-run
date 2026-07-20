# Product Fitness Observation Protocol

## Purpose

Observe whether a consenting target researcher can complete the critical
unfamiliar-project journey with the exact installed candidate. This is an
evaluation of quality in use, not a usability test of a mock and not evidence
that any scientific claim is true.

## Privacy and consent

- Explain the session, retained fields, withdrawal option, and claim ceiling
  before starting.
- Use a disposable or researcher-approved deidentified project. Do not retain
  names, contact details, raw datasets, private messages, unpublished
  human-subject material, or biosafety-sensitive operational detail.
- Store the receipt outside the repository. The repository retains only a
  deidentified disposition if an actor-disjoint reviewer later accepts it.
- Stop and mark the session incomplete if consent is withdrawn.

## Candidate binding

Record the clean Git commit plus the platform-qualified normalized installed
binary identity declared in
`agent-standards/product-fitness-disposition.json`. The participant must operate
that installed binary; source tests and an agent replay are non-substitutes.

## Critical journey

Without a manual command-by-command takeover, ask the participant to:

1. install or locate the candidate and initialize or retrofit an unfamiliar
   project with their chosen public review key;
2. record one source, scoped claim, specific evidence link, and negative or
   ambiguous experiment;
3. inspect status and retrieve the most relevant context;
4. prepare a review request, approve its detached signature outside Research
   Run, and import it;
5. diagnose one representative interrupted publication and recover it;
6. validate the workspace and explain the current claim ceiling in their own
   words.

The observer may clarify the task outcome but records every command takeover,
error, abandoned attempt, recovery step, and elapsed recovery time.

## Receipt and disposition

Copy `agent-standards/product-fitness-observation.template.json` outside the
repository, fill only the bounded deidentified fields, then run:

```console
scripts/check-product-fitness-receipt /absolute/path/to/receipt.json
```

A valid first-use receipt can support the unfamiliar-project workflow
observation. It cannot establish continuance. Product Fitness remains withheld
until an actor-disjoint reviewer evaluates the receipt and records a disposition
bound to the exact candidate.
