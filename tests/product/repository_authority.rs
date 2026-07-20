use std::fs;
use std::path::{Path, PathBuf};

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
}
