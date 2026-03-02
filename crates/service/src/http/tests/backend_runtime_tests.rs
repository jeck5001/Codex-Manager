use super::{
    http_queue_size, http_stream_queue_size, http_stream_worker_count, http_worker_count,
    HTTP_QUEUE_MIN, HTTP_STREAM_QUEUE_MIN, HTTP_STREAM_WORKER_MIN, HTTP_WORKER_MIN,
};

#[test]
fn worker_count_has_minimum_guard() {
    assert!(http_worker_count() >= HTTP_WORKER_MIN);
    assert!(http_stream_worker_count() >= HTTP_STREAM_WORKER_MIN);
}

#[test]
fn queue_size_has_minimum_guard() {
    assert!(http_queue_size(0) >= HTTP_QUEUE_MIN);
    assert!(http_stream_queue_size(0) >= HTTP_STREAM_QUEUE_MIN);
}
