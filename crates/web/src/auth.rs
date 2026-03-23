use super::*;

use axum::extract::Query;
use serde::Deserialize;

const WEB_AUTH_TAB_SESSION_STORAGE_KEY: &str = "codexmanager_web_auth_tab";
const WEB_AUTH_PENDING_COOKIE_NAME: &str = "codexmanager_web_auth_pending";

#[derive(Debug, Deserialize)]
pub(super) struct LoginForm {
    password: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct LoginQuery {
    force: Option<String>,
}

fn current_web_access_password_hash() -> Option<String> {
    codexmanager_service::current_web_access_password_hash()
}

pub(super) fn generate_web_auth_session_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub(super) fn build_web_auth_cookie_value(
    password_hash: &str,
    rpc_token: &str,
    session_key: &str,
) -> String {
    let scoped_rpc_token = format!("{rpc_token}:{session_key}");
    codexmanager_service::build_web_access_session_token(password_hash, &scoped_rpc_token)
}

pub(super) fn build_web_auth_pending_cookie_value(
    password_hash: &str,
    rpc_token: &str,
    session_key: &str,
) -> String {
    let scoped_rpc_token = format!("pending:{rpc_token}:{session_key}");
    codexmanager_service::build_web_access_session_token(password_hash, &scoped_rpc_token)
}

pub(super) fn parse_cookie_value(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|segment| {
        let (name, value) = segment.trim().split_once('=')?;
        if name.trim() == cookie_name {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

fn set_cookie_header_value(value: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{WEB_AUTH_COOKIE_NAME}={value}; Path=/; HttpOnly; SameSite=Lax"
    ))
    .ok()
}

fn clear_cookie_header_value() -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{WEB_AUTH_COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"
    ))
    .ok()
}

fn set_pending_cookie_header_value(value: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{WEB_AUTH_PENDING_COOKIE_NAME}={value}; Path=/__login; HttpOnly; SameSite=Lax"
    ))
    .ok()
}

fn clear_pending_cookie_header_value() -> Option<HeaderValue> {
    HeaderValue::from_str(&format!(
        "{WEB_AUTH_PENDING_COOKIE_NAME}=; Path=/__login; HttpOnly; SameSite=Lax; Max-Age=0"
    ))
    .ok()
}

fn append_no_store_headers(response: &mut Response) {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(header::EXPIRES, HeaderValue::from_static("0"));
}

fn current_auth_status_value() -> serde_json::Value {
    codexmanager_service::web_auth_status_value().unwrap_or_else(|_| {
        serde_json::json!({
            "passwordConfigured": current_web_access_password_hash().is_some(),
            "twoFactorEnabled": false,
            "recoveryCodesRemaining": 0,
        })
    })
}

fn two_factor_enabled() -> bool {
    current_auth_status_value()
        .get("twoFactorEnabled")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn login_force_requested(query: &LoginQuery) -> bool {
    query
        .force
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn request_has_pending_second_factor(headers: &HeaderMap, state: &AppState) -> bool {
    let Some(password_hash) = current_web_access_password_hash() else {
        return false;
    };
    let Some(cookie_value) = parse_cookie_value(headers, WEB_AUTH_PENDING_COOKIE_NAME) else {
        return false;
    };
    let expected = build_web_auth_pending_cookie_value(
        &password_hash,
        &state.rpc_token,
        &state.web_auth_session_key,
    );
    cookie_value == expected
}

fn request_is_authenticated(headers: &HeaderMap, state: &AppState) -> bool {
    let Some(password_hash) = current_web_access_password_hash() else {
        return true;
    };
    let Some(cookie_value) = parse_cookie_value(headers, WEB_AUTH_COOKIE_NAME) else {
        return false;
    };
    let expected = build_web_auth_cookie_value(
        &password_hash,
        &state.rpc_token,
        &state.web_auth_session_key,
    );
    cookie_value == expected
}

fn builtin_login_html(error: Option<&str>, two_factor_step: bool) -> String {
    let error_html = error
        .map(|text| format!(r#"<div class="error">{}</div>"#, escape_html(text)))
        .unwrap_or_default();
    let title = if two_factor_step {
        "输入二步验证码"
    } else {
        "访问受保护"
    };
    let description = if two_factor_step {
        "访问密码已验证，请输入 6 位 TOTP 验证码或恢复码以完成 Web 登录。"
    } else {
        "当前 CodexManager Web 已启用访问密码，请先验证后再进入管理页面。"
    };
    let label = if two_factor_step {
        "验证码或恢复码"
    } else {
        "访问密码"
    };
    let input_name = if two_factor_step { "code" } else { "password" };
    let input_type = if two_factor_step { "text" } else { "password" };
    let autocomplete = if two_factor_step {
        "one-time-code"
    } else {
        "current-password"
    };
    let placeholder = if two_factor_step {
        "请输入 6 位验证码或恢复码"
    } else {
        "请输入访问密码"
    };
    let button_text = if two_factor_step {
        "完成登录"
    } else {
        "进入控制台"
    };
    let foot = if two_factor_step {
        "若手机不可用，可输入恢复码完成登录。恢复码每个只能使用一次。"
    } else {
        "密码可在桌面端或 Web 端右上角的“密码”入口中修改。"
    };
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>CodexManager Web 登录</title>
    <style>
      :root {{
        color-scheme: light;
        --bg: #eef3f8;
        --panel: rgba(255,255,255,.92);
        --text: #142033;
        --muted: #627389;
        --accent: #0f6fff;
        --accent-strong: #0a57ca;
        --border: rgba(20,32,51,.12);
        --error-bg: rgba(193, 45, 45, .1);
        --error-fg: #b42318;
      }}
      * {{ box-sizing: border-box; }}
      body {{
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        padding: 24px;
        font-family: "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
        background:
          radial-gradient(circle at top left, rgba(15,111,255,.18), transparent 32%),
          radial-gradient(circle at bottom right, rgba(45,164,78,.14), transparent 26%),
          linear-gradient(160deg, #f6f9fc 0%, #e8eef6 100%);
        color: var(--text);
      }}
      .card {{
        width: min(100%, 420px);
        padding: 28px;
        border: 1px solid var(--border);
        border-radius: 20px;
        background: var(--panel);
        box-shadow: 0 24px 60px rgba(15, 23, 42, .12);
        backdrop-filter: blur(14px);
      }}
      .mark {{
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 44px;
        height: 44px;
        border-radius: 14px;
        background: linear-gradient(135deg, #0f6fff, #2bb673);
        color: #fff;
        font-weight: 700;
      }}
      h1 {{ margin: 16px 0 6px; font-size: 22px; }}
      p {{ margin: 0 0 18px; color: var(--muted); line-height: 1.6; }}
      label {{ display: block; margin-bottom: 10px; font-size: 14px; color: var(--muted); }}
      input {{
        width: 100%;
        border: 1px solid rgba(20,32,51,.16);
        border-radius: 14px;
        padding: 13px 14px;
        font-size: 15px;
        outline: none;
        background: rgba(255,255,255,.92);
      }}
      input:focus {{
        border-color: rgba(15,111,255,.58);
        box-shadow: 0 0 0 4px rgba(15,111,255,.12);
      }}
      button {{
        width: 100%;
        margin-top: 16px;
        border: 0;
        border-radius: 14px;
        padding: 13px 16px;
        font-size: 15px;
        font-weight: 600;
        color: #fff;
        background: linear-gradient(135deg, var(--accent), var(--accent-strong));
        cursor: pointer;
      }}
      button:hover {{ filter: brightness(.98); }}
      .error {{
        margin-bottom: 14px;
        padding: 12px 14px;
        border-radius: 12px;
        background: var(--error-bg);
        color: var(--error-fg);
        font-size: 14px;
      }}
      .foot {{
        margin-top: 14px;
        font-size: 12px;
        color: var(--muted);
        text-align: center;
      }}
    </style>
  </head>
  <body>
    <form class="card" method="post" action="/__login">
      <div class="mark">CM</div>
      <h1>{title}</h1>
      <p>{description}</p>
      {error_html}
      <label for="credential">{label}</label>
      <input id="credential" name="{input_name}" type="{input_type}" autocomplete="{autocomplete}" placeholder="{placeholder}" autofocus />
      <button type="submit">{button_text}</button>
      <div class="foot">{foot}</div>
    </form>
  </body>
</html>
"#
    )
}

fn login_success_html() -> String {
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>CodexManager Web 登录</title>
  </head>
  <body>
    <script>
      try {{
        window.sessionStorage.setItem("{WEB_AUTH_TAB_SESSION_STORAGE_KEY}", "1");
      }} catch (_err) {{}}
      window.location.replace("/");
    </script>
  </body>
</html>
"#
    )
}

fn logout_success_html() -> String {
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>CodexManager Web 已退出</title>
  </head>
  <body>
    <script>
      try {{
        window.sessionStorage.removeItem("{WEB_AUTH_TAB_SESSION_STORAGE_KEY}");
      }} catch (_err) {{}}
      window.location.replace("/__login?force=1");
    </script>
  </body>
</html>
"#
    )
}

pub(super) async fn web_auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    if path == "/__login" || path == "/__logout" {
        return next.run(request).await;
    }
    if request_is_authenticated(request.headers(), state.as_ref()) {
        return next.run(request).await;
    }
    if path.starts_with("/api/") {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "web_auth_required" })),
        )
            .into_response();
    }
    Redirect::to("/__login").into_response()
}

pub(super) async fn login_page(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LoginQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if current_web_access_password_hash().is_none() {
        return Redirect::to("/").into_response();
    }
    if request_is_authenticated(&headers, state.as_ref()) && !login_force_requested(&query) {
        return Redirect::to("/").into_response();
    }
    let show_two_factor = !login_force_requested(&query)
        && two_factor_enabled()
        && request_has_pending_second_factor(&headers, state.as_ref());
    let mut response = Html(builtin_login_html(None, show_two_factor)).into_response();
    append_no_store_headers(&mut response);
    response
}

pub(super) async fn login_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::Form(form): axum::Form<LoginForm>,
) -> impl IntoResponse {
    let Some(password_hash) = current_web_access_password_hash() else {
        return Redirect::to("/").into_response();
    };
    let waiting_for_second_factor =
        two_factor_enabled() && request_has_pending_second_factor(&headers, state.as_ref());

    if waiting_for_second_factor {
        let code = form.code.as_deref().unwrap_or("").trim();
        if code.is_empty() {
            let mut response = (
                StatusCode::BAD_REQUEST,
                Html(builtin_login_html(Some("请输入验证码或恢复码。"), true)),
            )
                .into_response();
            append_no_store_headers(&mut response);
            return response;
        }

        if let Err(err) = codexmanager_service::verify_web_access_second_factor(code) {
            let mut response = (
                StatusCode::UNAUTHORIZED,
                Html(builtin_login_html(Some(&err), true)),
            )
                .into_response();
            append_no_store_headers(&mut response);
            return response;
        }

        let token = build_web_auth_cookie_value(
            &password_hash,
            &state.rpc_token,
            &state.web_auth_session_key,
        );
        let mut response = Html(login_success_html()).into_response();
        if let Some(header_value) = set_cookie_header_value(&token) {
            response
                .headers_mut()
                .append(header::SET_COOKIE, header_value);
        }
        if let Some(header_value) = clear_pending_cookie_header_value() {
            response
                .headers_mut()
                .append(header::SET_COOKIE, header_value);
        }
        append_no_store_headers(&mut response);
        return response;
    }

    let password = form.password.as_deref().unwrap_or("");
    if !codexmanager_service::verify_web_access_password(password) {
        let mut response = (
            StatusCode::UNAUTHORIZED,
            Html(builtin_login_html(Some("密码错误，请重试。"), false)),
        )
            .into_response();
        append_no_store_headers(&mut response);
        return response;
    }

    if two_factor_enabled() {
        let pending = build_web_auth_pending_cookie_value(
            &password_hash,
            &state.rpc_token,
            &state.web_auth_session_key,
        );
        let mut response = Html(builtin_login_html(None, true)).into_response();
        if let Some(header_value) = set_pending_cookie_header_value(&pending) {
            response
                .headers_mut()
                .append(header::SET_COOKIE, header_value);
        }
        append_no_store_headers(&mut response);
        return response;
    }

    let token = build_web_auth_cookie_value(
        &password_hash,
        &state.rpc_token,
        &state.web_auth_session_key,
    );
    let mut response = Html(login_success_html()).into_response();
    if let Some(header_value) = set_cookie_header_value(&token) {
        response
            .headers_mut()
            .append(header::SET_COOKIE, header_value);
    }
    append_no_store_headers(&mut response);
    response
}

pub(super) async fn logout() -> impl IntoResponse {
    let mut response = Html(logout_success_html()).into_response();
    if let Some(header_value) = clear_cookie_header_value() {
        response
            .headers_mut()
            .append(header::SET_COOKIE, header_value);
    }
    if let Some(header_value) = clear_pending_cookie_header_value() {
        response
            .headers_mut()
            .append(header::SET_COOKIE, header_value);
    }
    append_no_store_headers(&mut response);
    response
}

pub(super) async fn auth_status() -> impl IntoResponse {
    let mut response = axum::Json(current_auth_status_value()).into_response();
    append_no_store_headers(&mut response);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};
    use totp_rs::{Algorithm, Secret, TOTP};

    static WEB_AUTH_TEST_SEQ: AtomicUsize = AtomicUsize::new(0);

    struct TestDbGuard {
        _env_lock: std::sync::MutexGuard<'static, ()>,
        original_db_path: Option<String>,
        db_path: PathBuf,
    }

    impl Drop for TestDbGuard {
        fn drop(&mut self) {
            if let Some(original) = &self.original_db_path {
                std::env::set_var("CODEXMANAGER_DB_PATH", original);
            } else {
                std::env::remove_var("CODEXMANAGER_DB_PATH");
            }
            let _ = fs::remove_file(&self.db_path);
            let _ = fs::remove_file(format!("{}-wal", self.db_path.display()));
            let _ = fs::remove_file(format!("{}-shm", self.db_path.display()));
        }
    }

    fn web_auth_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static WEB_AUTH_TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        WEB_AUTH_TEST_MUTEX
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("lock web auth test mutex")
    }

    fn setup_test_db(prefix: &str) -> TestDbGuard {
        let env_lock = web_auth_test_lock();
        let mut path = std::env::temp_dir();
        path.push(format!(
            "{prefix}-{}-{}.db",
            std::process::id(),
            WEB_AUTH_TEST_SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let original_db_path = std::env::var("CODEXMANAGER_DB_PATH").ok();
        std::env::set_var("CODEXMANAGER_DB_PATH", path.to_string_lossy().to_string());
        codexmanager_service::initialize_storage_if_needed().expect("init storage");
        TestDbGuard {
            _env_lock: env_lock,
            original_db_path,
            db_path: path,
        }
    }

    fn build_test_state() -> Arc<AppState> {
        let (shutdown_tx, _shutdown_rx) = watch::channel(false);
        Arc::new(AppState {
            client: reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("build client"),
            service_rpc_url: "http://127.0.0.1:48760/rpc".to_string(),
            service_addr: "127.0.0.1:48760".to_string(),
            rpc_token: codexmanager_service::rpc_auth_token().to_string(),
            web_auth_session_key: generate_web_auth_session_key(),
            shutdown_tx,
            spawned_service: Arc::new(tokio::sync::Mutex::new(false)),
            missing_ui_html: Arc::new(String::new()),
            web_root: Arc::new(PathBuf::from(".")),
        })
    }

    fn current_totp_code(secret: &str) -> String {
        let secret_bytes = Secret::Encoded(secret.to_string())
            .to_bytes()
            .expect("decode secret");
        TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            Some("CodexManager".to_string()),
            "Web Access".to_string(),
        )
        .expect("build totp")
        .generate_current()
        .expect("generate totp")
    }

    fn response_cookie_values(response: &Response) -> Vec<String> {
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn login_force_requested_accepts_truthy_flags() {
        for value in ["1", "true", "TRUE", "yes", "on"] {
            let query = LoginQuery {
                force: Some(value.to_string()),
            };
            assert!(login_force_requested(&query), "value={value}");
        }
        for value in ["", "0", "false", "no", "off"] {
            let query = LoginQuery {
                force: Some(value.to_string()),
            };
            assert!(!login_force_requested(&query), "value={value}");
        }
        assert!(!login_force_requested(&LoginQuery::default()));
    }

    #[test]
    fn login_success_html_marks_current_tab_session() {
        let html = login_success_html();
        assert!(html.contains("sessionStorage.setItem"));
        assert!(html.contains(WEB_AUTH_TAB_SESSION_STORAGE_KEY));
        assert!(html.contains("location.replace(\"/\")"));
    }

    #[test]
    fn two_factor_login_page_mentions_recovery_code() {
        let html = builtin_login_html(None, true);
        assert!(html.contains("恢复码"));
        assert!(html.contains("name=\"code\""));
    }

    #[tokio::test]
    async fn login_submit_requires_second_factor_after_password_verification() {
        let _db = setup_test_db("web-auth-login-step1");
        codexmanager_service::set_web_access_password(Some("P@ssw0rd!")).expect("set password");
        let setup = codexmanager_service::web_auth_two_factor_setup().expect("2fa setup");
        let secret = setup
            .get("secret")
            .and_then(|value| value.as_str())
            .expect("secret");
        let setup_token = setup
            .get("setupToken")
            .and_then(|value| value.as_str())
            .expect("setup token");
        codexmanager_service::web_auth_two_factor_verify(setup_token, &current_totp_code(secret))
            .expect("enable 2fa");

        let response = login_submit(
            State(build_test_state()),
            HeaderMap::new(),
            axum::Form(LoginForm {
                password: Some("P@ssw0rd!".to_string()),
                code: None,
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let cookies = response_cookie_values(&response);
        assert!(
            cookies
                .iter()
                .any(|value| value.contains("codexmanager_web_auth_pending=")),
            "cookies={cookies:?}"
        );
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let html = String::from_utf8(body.to_vec()).expect("html");
        assert!(html.contains("验证码或恢复码"));
    }

    #[tokio::test]
    async fn login_submit_exchanges_pending_cookie_for_authenticated_cookie() {
        let _db = setup_test_db("web-auth-login-step2");
        codexmanager_service::set_web_access_password(Some("P@ssw0rd!")).expect("set password");
        let setup = codexmanager_service::web_auth_two_factor_setup().expect("2fa setup");
        let secret = setup
            .get("secret")
            .and_then(|value| value.as_str())
            .expect("secret");
        let setup_token = setup
            .get("setupToken")
            .and_then(|value| value.as_str())
            .expect("setup token");
        codexmanager_service::web_auth_two_factor_verify(setup_token, &current_totp_code(secret))
            .expect("enable 2fa");

        let state = build_test_state();
        let password_hash =
            codexmanager_service::current_web_access_password_hash().expect("password hash");
        let pending_cookie = build_web_auth_pending_cookie_value(
            &password_hash,
            &state.rpc_token,
            &state.web_auth_session_key,
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("{WEB_AUTH_PENDING_COOKIE_NAME}={pending_cookie}"))
                .expect("pending cookie header"),
        );

        let response = login_submit(
            State(state),
            headers,
            axum::Form(LoginForm {
                password: None,
                code: Some(current_totp_code(secret)),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let cookies = response_cookie_values(&response);
        assert!(
            cookies
                .iter()
                .any(|value| value.contains("codexmanager_web_auth=")),
            "cookies={cookies:?}"
        );
        assert!(
            cookies.iter().any(|value| {
                value.contains("codexmanager_web_auth_pending=") && value.contains("Max-Age=0")
            }),
            "cookies={cookies:?}"
        );
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let html = String::from_utf8(body.to_vec()).expect("html");
        assert!(html.contains("location.replace(\"/\")"));
    }

    #[tokio::test]
    async fn login_submit_accepts_recovery_code_and_decrements_remaining_count() {
        let _db = setup_test_db("web-auth-login-recovery");
        codexmanager_service::set_web_access_password(Some("P@ssw0rd!")).expect("set password");
        let setup = codexmanager_service::web_auth_two_factor_setup().expect("2fa setup");
        let secret = setup
            .get("secret")
            .and_then(|value| value.as_str())
            .expect("secret");
        let setup_token = setup
            .get("setupToken")
            .and_then(|value| value.as_str())
            .expect("setup token");
        let recovery_code = setup
            .get("recoveryCodes")
            .and_then(|value| value.as_array())
            .and_then(|items| items.first())
            .and_then(|value| value.as_str())
            .expect("recovery code")
            .to_string();
        codexmanager_service::web_auth_two_factor_verify(setup_token, &current_totp_code(secret))
            .expect("enable 2fa");

        let state = build_test_state();
        let password_hash =
            codexmanager_service::current_web_access_password_hash().expect("password hash");
        let pending_cookie = build_web_auth_pending_cookie_value(
            &password_hash,
            &state.rpc_token,
            &state.web_auth_session_key,
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("{WEB_AUTH_PENDING_COOKIE_NAME}={pending_cookie}"))
                .expect("pending cookie header"),
        );

        let response = login_submit(
            State(state),
            headers,
            axum::Form(LoginForm {
                password: None,
                code: Some(recovery_code),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::OK);
        let cookies = response_cookie_values(&response);
        assert!(
            cookies
                .iter()
                .any(|value| value.contains("codexmanager_web_auth=")),
            "cookies={cookies:?}"
        );
        let status = codexmanager_service::web_auth_status_value().expect("status");
        assert_eq!(
            status
                .get("recoveryCodesRemaining")
                .and_then(|value| value.as_u64()),
            Some(7)
        );
    }

    #[tokio::test]
    async fn login_submit_rejects_invalid_second_factor_code_and_keeps_pending_cookie() {
        let _db = setup_test_db("web-auth-login-invalid-2fa");
        codexmanager_service::set_web_access_password(Some("P@ssw0rd!")).expect("set password");
        let setup = codexmanager_service::web_auth_two_factor_setup().expect("2fa setup");
        let secret = setup
            .get("secret")
            .and_then(|value| value.as_str())
            .expect("secret");
        let setup_token = setup
            .get("setupToken")
            .and_then(|value| value.as_str())
            .expect("setup token");
        codexmanager_service::web_auth_two_factor_verify(setup_token, &current_totp_code(secret))
            .expect("enable 2fa");

        let state = build_test_state();
        let password_hash =
            codexmanager_service::current_web_access_password_hash().expect("password hash");
        let pending_cookie = build_web_auth_pending_cookie_value(
            &password_hash,
            &state.rpc_token,
            &state.web_auth_session_key,
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_str(&format!("{WEB_AUTH_PENDING_COOKIE_NAME}={pending_cookie}"))
                .expect("pending cookie header"),
        );

        let response = login_submit(
            State(state),
            headers,
            axum::Form(LoginForm {
                password: None,
                code: Some("000000".to_string()),
            }),
        )
        .await
        .into_response();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let cookies = response_cookie_values(&response);
        assert!(
            !cookies
                .iter()
                .any(|value| value.contains("codexmanager_web_auth=")),
            "cookies={cookies:?}"
        );
        assert!(
            !cookies.iter().any(|value| {
                value.contains("codexmanager_web_auth_pending=") && value.contains("Max-Age=0")
            }),
            "cookies={cookies:?}"
        );
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        let html = String::from_utf8(body.to_vec()).expect("html");
        assert!(html.contains("验证码"));
    }
}
