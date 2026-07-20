use super::*;
use crate::domain::{EntityKind, EntityRef, KnowledgeKind, KnowledgeRecord, KnowledgeState};
use crate::workspace::knowledge::relationship_reference_errors;
use crate::workspace::references::current_review_bindings;

#[test]
fn history_traversal_enqueues_each_newly_ready_record() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "History traversal").expect("initialize");
    for id in ["old", "middle-a", "middle-b", "new"] {
        workspace
            .add_knowledge(&KnowledgeRecord {
                schema_version: 1,
                kind: "knowledge".to_owned(),
                id: id.to_owned(),
                record_type: KnowledgeKind::Observation,
                title: id.to_owned(),
                body: "Retained history".to_owned(),
                occurred_at: "2026-07-18T20:00:00Z".to_owned(),
                state: KnowledgeState::Open,
                authorship: Authorship::Human,
            })
            .expect("knowledge");
    }
    for (id, from, to) in [
        ("new-middle-a", "new", "middle-a"),
        ("new-middle-b", "new", "middle-b"),
        ("middle-a-old", "middle-a", "old"),
        ("middle-b-old", "middle-b", "old"),
    ] {
        workspace
            .add_relationship(&RelationshipRecord {
                schema_version: 1,
                kind: "relationship".to_owned(),
                id: id.to_owned(),
                relationship: RelationshipKind::Revises,
                from: EntityRef {
                    kind: EntityKind::Knowledge,
                    id: from.to_owned(),
                },
                to: EntityRef {
                    kind: EntityKind::Knowledge,
                    id: to.to_owned(),
                },
                rationale: "History".to_owned(),
                occurred_at: "2026-07-18T20:00:00Z".to_owned(),
                authorship: Authorship::Human,
            })
            .expect("relationship");
    }
    assert!(relationship_reference_errors(&workspace.load_snapshot().unwrap()).is_empty());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn authority_initialization_retry_restores_the_exact_anchored_record() {
    let root = temporary();
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    let mut invalid_header = authority.clone();
    invalid_header.schema_version = 2;
    assert!(
        Workspace::initialize_with_review_authority(
            &root.join("invalid-header"),
            "Invalid",
            &invalid_header,
        )
        .is_err()
    );
    let mut mismatched = authority.clone();
    mismatched.fingerprint = "SHA256:wrong".to_owned();
    let workspace = Workspace::initialize_with_review_authority(
        &root.join("direct-mismatch"),
        "Direct mismatch",
        &authority,
    )
    .expect("direct mismatch workspace");
    assert!(
        workspace
            .verify_initialized_authority(Some(&mismatched))
            .is_err(),
        "initialized authority accepted a different expected record"
    );
    assert!(
        Workspace::initialize_with_review_authority(&root.join("invalid"), "Invalid", &mismatched)
            .is_err()
    );
    inject_storage_failure("publish canonical record#2");
    assert!(Workspace::initialize_with_review_authority(&root, "Retry", &authority).is_err());
    assert!(root.join(".research-run/manifest.json").is_file());
    assert!(
        !root
            .join(".research-run/review-authorities/test-human.json")
            .exists()
    );
    Workspace::initialize_with_review_authority(&root, "Retry", &authority)
        .expect("idempotent retry");
    assert!(
        root.join(".research-run/review-authorities/test-human.json")
            .is_file()
    );
    fs::remove_file(root.join(".research-run/review-authorities/test-human.json"))
        .expect("remove authority");
    inject_storage_failure("publish canonical record");
    assert!(Workspace::initialize_with_review_authority(&root, "Retry", &authority).is_err());
    Workspace::initialize_with_review_authority(&root, "Retry", &authority)
        .expect("retry after publication failure");
    let rogue = ReviewAuthority::from_openssh("rogue-human".to_owned(), &test_review_public_key())
        .expect("rogue authority");
    let workspace = Workspace::discover(&root).expect("discover retry workspace");
    workspace
        .publish_record("review-authorities", &rogue)
        .expect("publish rogue fixture");
    assert!(
        Workspace::initialize_with_review_authority(&root, "Retry", &authority).is_err(),
        "retry accepted surplus review authority"
    );
    fs::remove_file(root.join(".research-run/review-authorities/rogue-human.json"))
        .expect("remove rogue authority fixture");

    let malformed_root = temporary();
    Workspace::initialize(&malformed_root, "Malformed retry").expect("initialize");
    fs::write(malformed_root.join(".research-run/manifest.json"), b"{").expect("corrupt manifest");
    assert!(Workspace::initialize(&malformed_root, "Malformed retry").is_err());
    fs::remove_dir_all(malformed_root).expect("remove fixture");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn unanchored_initialization_retry_rejects_a_rogue_authority() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Unanchored retry").expect("initialize");
    let rogue = ReviewAuthority::from_openssh("rogue-human".to_owned(), &test_review_public_key())
        .expect("rogue authority");
    workspace
        .publish_record("review-authorities", &rogue)
        .expect("publish rogue fixture");
    assert!(Workspace::initialize(&root, "Unanchored retry").is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn initialization_propagates_final_authority_snapshot_failure() {
    let root = temporary();
    inject_storage_failure("read record directory");
    assert!(Workspace::initialize(&root, "Snapshot failure").is_err());
    assert!(root.join(".research-run/manifest.json").is_file());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn unanchored_ai_claim_reports_the_irreversible_promotion_boundary() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Unanchored AI").expect("initialize");
    let mut record = claim("ai-claim");
    record.authorship = Authorship::Ai;
    workspace.add_claim(&record).expect("claim");
    workspace
        .add_claim(&claim("human-claim"))
        .expect("human claim");
    let status = workspace.status().expect("status");
    let claim = status
        .claims
        .iter()
        .find(|value| value.id == "ai-claim")
        .expect("claim status");
    assert!(
        claim
            .next_action
            .contains("new workspace with an anchored review authority")
    );
    let human = status
        .claims
        .iter()
        .find(|value| value.id == "human-claim")
        .expect("human claim status");
    assert!(
        human
            .next_action
            .contains("new workspace with an anchored review authority")
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn review_projection_rejects_invalid_duplicate_and_ambiguous_authorities() {
    let root = temporary();
    let workspace = review_workspace(&root, "Review projection");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    let subject = workspace.review_subject_binding("claim-one").unwrap().1;
    let mut first = review("review-one", "claim-one");
    first.subject_sha256 = Some(subject.clone());
    authorize_review(&workspace, &mut first);
    let mut second = review("review-two", "claim-one");
    second.subject_sha256 = Some(subject);
    authorize_review(&workspace, &mut second);

    let mut ambiguous = workspace.load_snapshot().expect("snapshot");
    ambiguous.reviews = vec![first, second];
    assert!(current_review_bindings(&workspace, &ambiguous).is_err());

    let mut duplicate = workspace.load_snapshot().expect("snapshot");
    duplicate
        .review_authorities
        .push(duplicate.review_authorities[0].clone());
    let errors = workspace.review_authorization_errors(&duplicate);
    assert!(errors.iter().any(|error| error.contains("exactly one")));
    assert!(workspace.status_from_snapshot(&duplicate).is_err());

    let mut invalid = workspace.load_snapshot().expect("snapshot");
    invalid.review_authorities[0].public_key = "not-an-openssh-key".to_owned();
    assert!(
        workspace
            .review_authorization_errors(&invalid)
            .iter()
            .any(|error| error.contains("review-authorities/test-human"))
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
