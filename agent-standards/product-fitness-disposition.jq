def exact_keys($expected): (keys | sort) == ($expected | sort);
def bounded_text:
  type == "string" and length > 0 and length <= 1024;
def sha256: type == "string" and test("^[0-9a-f]{64}$");
def authority_id: type == "string" and test("^[a-z0-9]+(?:-[a-z0-9]+)*$");

exact_keys([
  "schema_version", "kind", "candidate", "observation", "reviewer",
  "claims", "claim_ceiling"
]) and
.schema_version == 1 and
.kind == "product-fitness-disposition" and
(.candidate | exact_keys(["git_commit", "installed_binary"])) and
.candidate.git_commit == $candidate and
(.candidate.installed_binary |
  exact_keys(["platform", "normalized_sha256", "normalized_bytes"])) and
.candidate.installed_binary == {
  platform: $platform,
  normalized_sha256: $normalized_sha256,
  normalized_bytes: $normalized_bytes
} and
(.observation | exact_keys(["kind", "receipt_sha256", "completed"])) and
.observation == {
  kind: "product-fitness-observation",
  receipt_sha256: $receipt_sha256,
  completed: true
} and
(.reviewer | exact_keys([
  "authority_id", "role", "actor_disjoint_from_participant",
  "actor_disjoint_from_observer"
])) and
(.reviewer.authority_id | authority_id) and
(.reviewer.role | bounded_text) and
.reviewer.actor_disjoint_from_participant == true and
.reviewer.actor_disjoint_from_observer == true and
(.claims | type == "array" and length == 2) and
(.claims | map(.claim_id)) == [
  "research-run-v0.1-complete-unfamiliar-project-workflow",
  "research-run-v0.1-real-use-fitness"
] and
all(.claims[];
  exact_keys([
    "claim_id", "status", "audience", "job", "context", "outcome",
    "accessibility", "cognitive_load", "recovery_burden", "continuance",
    "real_use_evidence", "non_substitutes_rejected", "reason"
  ]) and
  (.audience | type == "array" and length > 0 and all(.[]; bounded_text)) and
  (.job | bounded_text) and
  (.context | bounded_text) and
  (.outcome | exact_keys(["completed", "limitations"])) and
  .outcome.completed == true and
  (.outcome.limitations | type == "array" and all(.[]; bounded_text)) and
  (.accessibility | exact_keys(["assistive_mode", "blocker_count"])) and
  (.accessibility.assistive_mode | bounded_text) and
  (.accessibility.blocker_count |
    type == "number" and floor == . and . >= 0) and
  (.cognitive_load | exact_keys(["rating_1_to_5", "confusing_step_count"])) and
  (.cognitive_load.rating_1_to_5 |
    type == "number" and floor == . and . >= 1 and . <= 5) and
  (.cognitive_load.confusing_step_count |
    type == "number" and floor == . and . >= 0) and
  (.recovery_burden | exact_keys([
    "error_count", "command_takeovers", "abandoned_attempts", "recovery_seconds"
  ])) and
  all(.recovery_burden[]; type == "number" and . >= 0 and floor == .) and
  .continuance == {status: "not_observed", voluntary_sessions: 1} and
  .real_use_evidence == {
    kind: "product-fitness-observation",
    receipt_sha256: $receipt_sha256
  } and
  .non_substitutes_rejected == [
    "source tests", "coverage or mutation proof", "installed journey",
    "synthetic signing", "package proof", "benchmark", "generic reviewer approval"
  ] and
  (.reason | bounded_text)
) and
(.claims[0].status == "supported" or
 .claims[0].status == "limited" or
 .claims[0].status == "withheld") and
# A single first-use receipt cannot transition general Product Fitness.
.claims[1].status == "withheld" and
.claim_ceiling ==
  "Actor-disjoint disposition of one exact-candidate first-use observation; not personhood, continuance, general Product Fitness, scientific truth, scientific impact, or outcome improvement."
