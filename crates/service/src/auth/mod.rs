#[path = "auth_account.rs"]
pub(crate) mod account;
#[path = "auth_callback.rs"]
pub(crate) mod callback;
#[path = "auth_login.rs"]
pub(crate) mod login;
pub(crate) mod management_access;
pub(crate) mod rpc;
pub(crate) mod secret_store;
#[path = "auth_tokens.rs"]
pub(crate) mod tokens;
pub(crate) mod web_access;
pub(crate) mod web_access_2fa;

pub use management_access::{
    current_remote_management_secret_hash, remote_management_secret_configured,
    set_remote_management_secret, verify_remote_management_secret,
};
pub use rpc::{rpc_auth_token, rpc_auth_token_matches};
pub use web_access::{
    build_web_access_session_token, current_web_access_password_hash, set_web_access_password,
    verify_web_access_password, web_access_password_configured, web_auth_status_value,
};
pub use web_access_2fa::{
    clear_web_access_two_factor, verify_web_access_second_factor, web_auth_two_factor_disable,
    web_auth_two_factor_enabled, web_auth_two_factor_setup, web_auth_two_factor_verify,
    web_auth_two_factor_verify_current,
};
