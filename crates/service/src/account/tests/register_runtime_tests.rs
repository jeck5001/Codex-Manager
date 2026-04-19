use super::{
    append_register_task_log_for_test, create_local_register_task_for_test,
    read_local_register_task_for_test, set_register_task_status_for_test, LocalRegisterTaskInput,
};

#[test]
fn register_runtime_creates_pending_task_snapshot() {
    let task = create_local_register_task_for_test(LocalRegisterTaskInput {
        email_service_type: "generator_email".to_string(),
        register_mode: "standard".to_string(),
        proxy: None,
    });

    assert_eq!(task.status, "queued");
    assert_eq!(task.email_service_type, "generator_email");
    assert!(task.task_uuid.starts_with("reg-"));
}

#[test]
fn register_runtime_appends_logs_and_updates_status() {
    let task_uuid = create_local_register_task_for_test(LocalRegisterTaskInput {
        email_service_type: "generator_email".to_string(),
        register_mode: "standard".to_string(),
        proxy: None,
    })
    .task_uuid;

    append_register_task_log_for_test(&task_uuid, "signup email submitted");
    set_register_task_status_for_test(&task_uuid, "submitting_signup", None);

    let snapshot = read_local_register_task_for_test(&task_uuid).expect("task snapshot");
    assert_eq!(snapshot.status, "submitting_signup");
    assert!(snapshot
        .logs
        .iter()
        .any(|line| line.contains("signup email submitted")));
}
