use super::{
    cancel_local_register_batch_for_test, read_local_register_batch_for_test,
    start_local_register_batch_for_test, LocalRegisterBatchInput,
};

#[test]
fn register_batch_runtime_creates_multiple_local_tasks() {
    let batch = start_local_register_batch_for_test(LocalRegisterBatchInput {
        email_service_type: "generator_email".to_string(),
        register_mode: "standard".to_string(),
        proxy: None,
        count: 3,
        interval_min: 0,
        interval_max: 0,
        concurrency: 1,
        mode: "pipeline".to_string(),
    })
    .expect("start batch");

    assert_eq!(batch.total, 3);
    assert_eq!(batch.task_uuids.len(), 3);
}

#[test]
fn register_batch_runtime_cancel_prevents_new_tasks_from_starting() {
    let batch = start_local_register_batch_for_test(LocalRegisterBatchInput {
        email_service_type: "generator_email".to_string(),
        register_mode: "standard".to_string(),
        proxy: None,
        count: 3,
        interval_min: 0,
        interval_max: 0,
        concurrency: 1,
        mode: "pipeline".to_string(),
    })
    .expect("start batch");
    cancel_local_register_batch_for_test(&batch.batch_id).expect("cancel batch");
    let snapshot = read_local_register_batch_for_test(&batch.batch_id).expect("batch snapshot");
    assert_eq!(snapshot.status, "cancelled");
}
