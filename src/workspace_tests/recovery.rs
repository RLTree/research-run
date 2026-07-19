use super::*;

#[test]
fn pending_filename_recovers_exact_target() {
    let path = std::path::Path::new(".claim-one.json.123.4.tmp");
    assert_eq!(
        interrupted_target_name(path.file_name().expect("fixture filename")),
        Some("claim-one.json")
    );
    for invalid in [
        "claim-one.json.123.4.tmp",
        ".claim-one.json.123.4",
        ".claim-one.json.123.nope.tmp",
        ".claim-one.json.nope.4.tmp",
        ".claim-one.txt.123.4.tmp",
    ] {
        assert_eq!(interrupted_target_name(std::ffi::OsStr::new(invalid)), None);
    }
}

#[test]
fn valid_pending_records_recover_in_dependency_order() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Recovery matrix").expect("initialize");

    let source = source("source-one");
    let claim = claim("claim-one");
    let experiment = experiment("experiment-one");
    let mut evidence = evidence("evidence-one", &claim.id, Some(&source.id));
    evidence.experiment_id = None;
    let mut review = review("review-one", &claim.id);
    review.evidence_ids = vec![evidence.id.clone()];
    for (directory, filename, bytes) in [
        (
            "sources",
            "source-one.json",
            serde_json::to_vec_pretty(&source).expect("source"),
        ),
        (
            "claims",
            "claim-one.json",
            serde_json::to_vec_pretty(&claim).expect("claim"),
        ),
        (
            "experiments",
            "experiment-one.json",
            serde_json::to_vec_pretty(&experiment).expect("experiment"),
        ),
        (
            "evidence",
            "evidence-one.json",
            serde_json::to_vec_pretty(&evidence).expect("evidence"),
        ),
        (
            "reviews",
            "review-one.json",
            serde_json::to_vec_pretty(&review).expect("review"),
        ),
    ] {
        let pending = workspace
            .state
            .join(directory)
            .join(format!(".{filename}.9.1.tmp"));
        fs::write(&pending, bytes).expect("pending record");
    }
    let result = workspace.recover().expect("recover records");
    assert_eq!(result.recovered.len(), 5);
    assert!(workspace.validate().valid);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn injected_recovery_faults_preserve_pending_effects() {
    for point in [
        "pending record count",
        "recovery record count",
        "publish recovered record",
        "recovered references",
    ] {
        let root = temporary();
        let workspace = Workspace::initialize(&root, "Recovery fault").expect("initialize");
        let pending = workspace.state.join("sources/.source-one.json.9.1.tmp");
        fs::write(
            &pending,
            serde_json::to_vec_pretty(&source("source-one")).expect("source"),
        )
        .expect("pending source");
        inject_storage_failure(point);
        assert!(workspace.recover().is_err(), "fault {point} was ignored");
        assert!(pending.exists(), "fault {point} removed pending state");
        assert!(
            !workspace.state.join("sources/source-one.json").exists(),
            "fault {point} published canonical state"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    let root = temporary();
    let workspace = Workspace::initialize(&root, "Recovery conflict").expect("initialize");
    let target = workspace.state.join("sources/source-one.json");
    let bytes = serde_json::to_vec_pretty(&source("source-one")).expect("source");
    fs::write(&target, &bytes).expect("target source");
    let pending = workspace.state.join("sources/.source-one.json.9.1.tmp");
    fs::write(&pending, &bytes).expect("pending source");
    inject_storage_failure("recovery target conflict");
    assert!(workspace.recover().is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn post_mutation_recovery_sync_failures_are_ambiguous() {
    let published_root = temporary();
    let published =
        Workspace::initialize(&published_root, "Published sync").expect("initialize workspace");
    let published_pending = published.state.join("sources/.source-one.json.9.1.tmp");
    fs::write(
        &published_pending,
        serde_json::to_vec_pretty(&source("source-one")).expect("source"),
    )
    .expect("pending source");
    inject_storage_failure("sync record directory");
    let error = published.recover().expect_err("publication sync failure");
    assert!(matches!(error, crate::Error::AmbiguousEffect(_)));
    assert!(error.to_string().contains("directory sync failed"));
    assert!(published_pending.exists());
    assert!(published.state.join("sources/source-one.json").exists());
    fs::remove_dir_all(published_root).expect("remove fixture");

    let discarded_root = temporary();
    let discarded =
        Workspace::initialize(&discarded_root, "Discard sync").expect("initialize workspace");
    discarded
        .add_source(&source("source-one"))
        .expect("canonical source");
    let discarded_pending = discarded.state.join("sources/.source-one.json.9.1.tmp");
    fs::copy(
        discarded.state.join("sources/source-one.json"),
        &discarded_pending,
    )
    .expect("identical pending source");
    inject_storage_failure("sync record directory");
    let error = discarded.recover().expect_err("discard sync failure");
    assert!(matches!(error, crate::Error::AmbiguousEffect(_)));
    assert!(error.to_string().contains("directory sync failed"));
    assert!(!discarded_pending.exists());
    fs::remove_dir_all(discarded_root).expect("remove fixture");
}
