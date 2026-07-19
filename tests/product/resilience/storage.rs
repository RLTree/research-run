use super::*;

#[path = "storage/directory_discovery.rs"]
mod directory_discovery;

#[test]
fn installed_process_exercises_read_and_lock_fault_boundaries() {
    for fault in [
        "lock identity",
        "open workspace write lock",
        "inspect record",
        "read record",
        "record grew",
        "record identity",
        "inspect workspace path",
        "inspect path component",
        "record count",
    ] {
        let root = initialize(fault);
        if fault == "record count" {
            assert!(add_source(&root.0, "source-one", None).status.success());
        }
        let output = if matches!(fault, "lock identity" | "open workspace write lock") {
            add_source(&root.0, "source-one", Some(fault))
        } else {
            run(&root.0, &["status", "--json"], Some(fault))
        };
        assert!(!output.status.success(), "fault {fault} was ignored");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("Citation"),
            "fault {fault} leaked record content"
        );
    }
}

#[test]
fn installed_process_exercises_atomic_publication_fault_boundaries() {
    let identical = initialize("identical-race");
    let output = add_source(&identical.0, "source-one", Some("publish identical race"));
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Already present"));

    for fault in [
        "create pending record",
        "open pending record",
        "publish conflicting race",
        "publish canonical record",
        "hard link canonical record",
        "sync record directory",
        "remove pending record",
        "remove abandoned pending record",
        "sync abandoned pending directory",
        "abandoned pending path is directory",
    ] {
        let root = initialize(fault);
        let output = add_source(&root.0, "source-one", Some(fault));
        assert!(!output.status.success(), "fault {fault} was ignored");
        if fault == "hard link canonical record" {
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("publish canonical record"),
                "hard-link failure was misclassified: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let canonical = root.0.join(".research-run/sources/source-one.json");
        if matches!(
            fault,
            "publish conflicting race"
                | "sync record directory"
                | "remove pending record"
                | "remove abandoned pending record"
                | "sync abandoned pending directory"
        ) {
            assert!(canonical.exists(), "ambiguous effect was not preserved");
        } else {
            assert!(!canonical.exists(), "failed publication became canonical");
        }
    }
}

#[cfg(unix)]
#[test]
fn installed_process_rejects_publication_and_read_identity_races() {
    for fault in [
        "inspect publication symlink race",
        "inspect publication pending race",
        "inspect publication unreadable race",
    ] {
        let root = initialize(fault);
        let output = add_source(&root.0, "source-one", Some(fault));
        assert!(!output.status.success(), "fault {fault} was ignored");
    }

    let root = initialize("record-symlink-after-read");
    assert!(
        !run(
            &root.0,
            &["status", "--json"],
            Some("record symlink after read")
        )
        .status
        .success()
    );
}

#[cfg(not(unix))]
#[test]
fn installed_process_rejects_publication_and_read_identity_races() {}
