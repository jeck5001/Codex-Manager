use super::{
    extract_generator_email_code_for_test, generator_email_surl_for_test,
    parse_generator_email_address_for_test,
};

#[test]
fn generator_email_parses_address_from_homepage_html() {
    let html = r#"<span id="email_ch_text">alpha123@generator.email</span>"#;
    assert_eq!(
        parse_generator_email_address_for_test(html),
        Some("alpha123@generator.email".to_string())
    );
}

#[test]
fn generator_email_builds_surl_from_email() {
    assert_eq!(
        generator_email_surl_for_test("Alpha.123@generator.email"),
        Some("generator.email/alpha.123".to_string())
    );
}

#[test]
fn generator_email_extracts_openai_code_from_mailbox_html() {
    let html = "<html><body>Your ChatGPT code is 123456</body></html>";
    assert_eq!(
        extract_generator_email_code_for_test(html),
        Some("123456".to_string())
    );
}
