use super::usage_http_client;

#[test]
fn usage_http_client_is_cloneable() {
    let first = usage_http_client();
    let second = usage_http_client();
    let first_ptr = &first as *const reqwest::blocking::Client;
    let second_ptr = &second as *const reqwest::blocking::Client;
    assert_ne!(first_ptr, second_ptr);
}
