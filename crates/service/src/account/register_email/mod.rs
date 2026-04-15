use serde::{Deserialize, Serialize};

pub(crate) mod generator_email;
pub(crate) use generator_email::GeneratorEmailProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterMailboxLease {
    pub email: String,
    pub credential: String,
}

pub(crate) trait RegisterEmailProvider {
    fn create_mailbox(&self) -> Result<RegisterMailboxLease, String>;
    fn fetch_code(&self, credential: &str) -> Result<Option<String>, String>;
}

#[cfg(test)]
pub(crate) fn parse_generator_email_address_for_test(html: &str) -> Option<String> {
    generator_email::parse_generator_email_address(html)
}

#[cfg(test)]
pub(crate) fn generator_email_surl_for_test(email: &str) -> Option<String> {
    generator_email::build_generator_email_surl(email)
}

#[cfg(test)]
pub(crate) fn extract_generator_email_code_for_test(html: &str) -> Option<String> {
    generator_email::extract_generator_email_code(html)
}

#[cfg(test)]
pub(crate) fn normalize_generator_email_proxy_for_test(proxy: Option<&str>) -> Option<String> {
    generator_email::normalize_proxy_url_for_test(proxy)
}

#[cfg(test)]
#[path = "../tests/register_email_generator_tests.rs"]
mod tests;
