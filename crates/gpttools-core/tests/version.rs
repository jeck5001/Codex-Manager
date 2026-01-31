#[test]
fn core_version_is_set() {
    assert!(!gpttools_core::core_version().is_empty());
}
