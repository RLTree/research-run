use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn read(relative: &str) -> String {
    fs::read_to_string(root().join(relative))
        .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"))
}

#[test]
fn router_reaches_every_current_authority_surface() {
    let agents = read("AGENTS.md");
    for required in [
        "STANDARD.md",
        "ARCHITECTURE.md",
        "policy.toml",
        "AGENT_STANDARDS.md",
        "docs/exec-plans/active/",
        "docs/research/2026-07-17-v0.1-evidence-and-decisions.md",
    ] {
        assert!(
            agents.contains(required),
            "AGENTS.md does not route {required}"
        );
    }

    let router = read("AGENT_STANDARDS.md");
    for module in [
        "01-execution-and-coverage.md",
        "02-product-and-security.md",
        "03-review-and-completion.md",
        "04-proof-surfaces.md",
    ] {
        assert!(
            root().join("agent-standards").join(module).is_file(),
            "routed standard is missing: {module}"
        );
        assert!(router.contains(module), "router does not name {module}");
    }
}

#[test]
fn policy_preserves_review_and_proof_claim_ceilings() {
    let policy = read("policy.toml");
    for required in [
        "material_rounds_max = 2",
        "bounded_targeted_reviews_max = 1",
        "manifest_version = \"0.0.12\"",
        "installed_cache_version_observed = \"0.0.11\"",
        "fit_classification = \"conflicting\"",
        "fit_record_status = \"historical_non_authoritative\"",
        "current_fit_classification = \"withheld_not_reprobed\"",
        "fit_receipt = \"absent\"",
        "fit_missing_files = 67",
        "fit_conflicts = [\"AGENTS.md\", \"AGENT_STANDARDS.md\", \"ARCHITECTURE.md\", \"scripts/check\"]",
        "exact_source_build = \"passed_locked_offline_source_only\"",
        "fit_apply = \"withheld_conflicting_authority\"",
        "full_activation = false",
        "class = \"technical_preview\"",
        "publication_authorized = false",
        "personhood_claim = false",
        "scientific_truth_claim = false",
        "transaction_version = 3",
        "previous_transaction_versions = [2]",
        "pre_exchange_anchor_checks = [\"project_root\", \"relative_transaction\"]",
        "post_transaction_create_authority_failure = \"metadata_permission_parent_sync_or_handle_failure_is_ambiguous_effect_retain_evidence\"",
        "transaction_witnesses = [\"version\", \"original\", \"reviewed\", \"exchange\", \"completion\"]",
        "transaction_child_budget = 5",
        "staging_cleanup = \"held_descriptor_fd_relative_closed_witness_set\"",
        "completion_publication = \"fd_relative_create_only_hard_link\"",
        "recovery_without_completion = \"exact_full_states_only\"",
        "coverage_line_floor = 100",
        "rust_toolchain = \"1.97.1\"",
        "artifact = \"harness-ultragoal-governance-complete.zip\"",
        "sha256 = \"089382f27b64c6eb219b3a032296917dc8984129a52e56e63cb77ba66ba72377\"",
    ] {
        assert!(policy.contains(required), "policy omits: {required}");
    }

    let review = read("agent-standards/03-review-and-completion.md");
    assert!(review.contains("do not start a third round"));
    assert!(review.contains("reserved for the claim-bearing material"));
    assert!(!review.contains("Every material round uses all four"));
}

#[test]
fn active_execplan_scopes_the_pr12_merge_exception_without_waiving_coverage() {
    let plan = read("docs/exec-plans/active/2026-07-20-v0.1-release-readiness.md");
    let plan = plan.split_whitespace().collect::<Vec<_>>().join(" ");
    for required in [
        "Historical delivery branch: `codex/pr-review-automation`, represented by PR #7",
        "Current bounded delivery candidate: PR #12 on `codex/fix-agent-activation-enforcement`",
        "supersedes every earlier PR #12-specific no-merge prohibition in this ExecPlan and the generic no-merge rule in `AGENTS.md`",
        "The inherited repository-wide 100% coverage threshold remains red and continues to withhold exact source coverage",
        "does not block this one PR #12 merge",
        "Complete patch-local proof must execute every material new or changed decision boundary",
        "Precisely enumerated defensive, unreachable, or low-value missed coordinates may remain disclosed under the existing claim ceiling",
        "Before the final candidate is frozen, every repair change requires fresh exact-head proof",
        "After the final candidate is frozen, any head drift requires fresh authority",
    ] {
        assert!(
            plan.contains(required),
            "active ExecPlan leaves the PR #12 merge exception ambiguous: {required}"
        );
    }
}

#[cfg(unix)]
#[test]
fn repository_check_entrypoints_are_executable() {
    use std::os::unix::fs::PermissionsExt;

    for path in [
        "scripts/check",
        "scripts/check-coverage",
        "scripts/check-dependencies",
        "scripts/check-mutations",
        "scripts/check-product-fitness-receipt",
        "scripts/check-product-artifacts",
        "scripts/check-release-candidate",
        "scripts/read-cargo-package-version.py",
        "scripts/prepare-release-authorization",
        "scripts/check-standards",
        "scripts/verify-release-authorization",
    ] {
        let mode = fs::metadata(root().join(path))
            .unwrap_or_else(|error| panic!("missing {path}: {error}"))
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0, "{path} must be executable");
    }

    let coverage = read("scripts/check-coverage");
    assert!(
        coverage.contains("export RUST_TEST_THREADS=\"${RUST_TEST_THREADS:-1}\""),
        "coverage must serialize tests that share process-global fault state"
    );
}

#[test]
fn release_version_reader_uses_the_package_table() {
    let fixture = std::env::temp_dir().join(format!(
        "research-run-version-fixture-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&fixture);
    fs::write(
        &fixture,
        "version = \"9.9.9\" # decoy\n[package]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .expect("write version fixture");
    let output = Command::new(root().join("scripts/read-cargo-package-version.py"))
        .arg(&fixture)
        .output()
        .expect("read package version");
    fs::remove_file(fixture).expect("remove version fixture");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "0.1.0");
}
