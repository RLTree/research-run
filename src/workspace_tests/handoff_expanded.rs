use super::*;

#[test]
fn handoff_validation_rejects_every_identity_boundary() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Handoff validation").expect("initialize");
    let bundle = workspace
        .handoff("handoff-one", "2026-07-18T20:10:00Z", None, 10)
        .expect("handoff");
    for mutate in [
        |value: &mut crate::workspace::HandoffBundle| value.schema_version = 2,
        |value: &mut crate::workspace::HandoffBundle| value.kind = "unknown".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| value.context.kind = "unknown".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| value.id = "INVALID".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| value.generated_at = "bad".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| {
            value.context.project_id = "INVALID".to_owned()
        },
    ] {
        let mut invalid = bundle.clone();
        mutate(&mut invalid);
        assert!(invalid.validate().is_err());
    }
    fs::remove_dir_all(root).expect("remove fixture");
}
