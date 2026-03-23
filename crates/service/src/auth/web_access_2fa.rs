use crate::app_settings::{
    get_persisted_app_setting, save_persisted_app_setting,
    APP_SETTING_WEB_ACCESS_2FA_RECOVERY_CODES_KEY,
    APP_SETTING_WEB_ACCESS_2FA_SECRET_ENCRYPTED_KEY,
};
use codexmanager_core::storage::now_ts;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use totp_rs::{Algorithm, Secret, TOTP};

const TOTP_ISSUER: &str = "CodexManager";
const TOTP_ACCOUNT_NAME: &str = "Web Access";
const SETUP_TOKEN_TTL_SECS: i64 = 10 * 60;
const RECOVERY_CODE_COUNT: usize = 8;
const RECOVERY_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Debug)]
pub struct WebAuthSecondFactorOutcome {
    pub method: &'static str,
    pub recovery_codes_remaining: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetupPayload {
    issued_at: i64,
    secret: String,
    recovery_codes: Vec<String>,
}

fn current_secret_encrypted() -> Option<String> {
    get_persisted_app_setting(APP_SETTING_WEB_ACCESS_2FA_SECRET_ENCRYPTED_KEY)
}

fn current_recovery_code_hashes() -> Vec<String> {
    get_persisted_app_setting(APP_SETTING_WEB_ACCESS_2FA_RECOVERY_CODES_KEY)
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .unwrap_or_default()
}

pub fn web_auth_two_factor_enabled() -> bool {
    current_secret_encrypted()
        .and_then(|secret| decrypt_secret(&secret).ok())
        .is_some()
}

pub fn web_auth_two_factor_setup() -> Result<Value, String> {
    if !crate::web_access_password_configured() {
        return Err("请先设置访问密码，再启用二步验证".to_string());
    }

    let mut secret_raw = [0u8; 20];
    rand::rngs::OsRng.fill_bytes(&mut secret_raw);
    let totp = build_totp_from_raw_secret(secret_raw.to_vec())?;
    let secret = totp.get_secret_base32();
    let recovery_codes = generate_recovery_codes();
    let setup_token = encode_setup_token(&SetupPayload {
        issued_at: now_ts(),
        secret: secret.clone(),
        recovery_codes: recovery_codes.clone(),
    })?;

    Ok(json!({
        "enabled": web_auth_two_factor_enabled(),
        "secret": secret,
        "otpAuthUrl": totp.get_url(),
        "qrCodeDataUrl": format!("data:image/png;base64,{}", totp.get_qr_base64().map_err(|err| err.to_string())?),
        "recoveryCodes": recovery_codes,
        "setupToken": setup_token,
    }))
}

pub fn web_auth_two_factor_verify(setup_token: &str, code: &str) -> Result<Value, String> {
    if !crate::web_access_password_configured() {
        return Err("请先设置访问密码，再启用二步验证".to_string());
    }

    let payload = decode_setup_token(setup_token)?;
    if !verify_totp_code(&payload.secret, code)? {
        return Err("验证码无效，请重试".to_string());
    }

    save_persisted_app_setting(
        APP_SETTING_WEB_ACCESS_2FA_SECRET_ENCRYPTED_KEY,
        Some(&encrypt_secret(&payload.secret)),
    )?;
    save_recovery_code_hashes(&payload.recovery_codes)?;

    Ok(two_factor_status_value("totp"))
}

pub fn web_auth_two_factor_verify_current(code: &str) -> Result<Value, String> {
    let outcome = verify_web_access_second_factor(code)?;
    Ok(two_factor_status_value(outcome.method))
}

pub fn web_auth_two_factor_disable(code: &str) -> Result<Value, String> {
    let _ = verify_web_access_second_factor(code)?;
    clear_web_access_two_factor()?;
    Ok(two_factor_status_value("disabled"))
}

pub fn verify_web_access_second_factor(code: &str) -> Result<WebAuthSecondFactorOutcome, String> {
    let secret = current_secret_encrypted()
        .ok_or_else(|| "尚未启用二步验证".to_string())
        .and_then(|raw| decrypt_secret(&raw))?;

    if verify_totp_code(&secret, code)? {
        return Ok(WebAuthSecondFactorOutcome {
            method: "totp",
            recovery_codes_remaining: current_recovery_code_hashes().len(),
        });
    }

    let normalized_recovery = normalize_recovery_code(code);
    if normalized_recovery.is_empty() {
        return Err("验证码或恢复码无效".to_string());
    }
    let target_hash = hash_recovery_code(&normalized_recovery);
    let mut remaining = current_recovery_code_hashes();
    if let Some(index) = remaining.iter().position(|item| item == &target_hash) {
        remaining.remove(index);
        save_recovery_code_hashes_hashed(&remaining)?;
        return Ok(WebAuthSecondFactorOutcome {
            method: "recovery_code",
            recovery_codes_remaining: remaining.len(),
        });
    }

    Err("验证码或恢复码无效".to_string())
}

pub fn clear_web_access_two_factor() -> Result<(), String> {
    save_persisted_app_setting(APP_SETTING_WEB_ACCESS_2FA_SECRET_ENCRYPTED_KEY, Some(""))?;
    save_persisted_app_setting(APP_SETTING_WEB_ACCESS_2FA_RECOVERY_CODES_KEY, Some(""))?;
    Ok(())
}

pub(crate) fn web_auth_two_factor_recovery_codes_remaining() -> usize {
    current_recovery_code_hashes().len()
}

fn build_totp_from_raw_secret(secret: Vec<u8>) -> Result<TOTP, String> {
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret,
        Some(TOTP_ISSUER.to_string()),
        TOTP_ACCOUNT_NAME.to_string(),
    )
    .map_err(|err| err.to_string())
}

fn build_totp_from_encoded_secret(secret: &str) -> Result<TOTP, String> {
    let secret_bytes = Secret::Encoded(secret.to_string())
        .to_bytes()
        .map_err(|err| err.to_string())?;
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
        Some(TOTP_ISSUER.to_string()),
        TOTP_ACCOUNT_NAME.to_string(),
    )
    .map_err(|err| err.to_string())
}

fn verify_totp_code(secret: &str, code: &str) -> Result<bool, String> {
    let normalized = normalize_totp_code(code);
    if normalized.is_empty() {
        return Ok(false);
    }
    build_totp_from_encoded_secret(secret)?
        .check_current(&normalized)
        .map_err(|err| err.to_string())
}

fn encode_setup_token(payload: &SetupPayload) -> Result<String, String> {
    let encoded = hex_encode(
        &serde_json::to_vec(payload).map_err(|err| format!("serialize 2fa setup token failed: {err}"))?,
    );
    let signature = hex_sha256(
        format!("codexmanager:web-auth-2fa-setup:{encoded}:{}", crate::rpc_auth_token())
            .as_bytes(),
    );
    Ok(format!("{encoded}.{signature}"))
}

fn decode_setup_token(token: &str) -> Result<SetupPayload, String> {
    let (encoded, signature) = token
        .trim()
        .split_once('.')
        .ok_or_else(|| "setup token 格式无效".to_string())?;
    let expected = hex_sha256(
        format!("codexmanager:web-auth-2fa-setup:{encoded}:{}", crate::rpc_auth_token())
            .as_bytes(),
    );
    if !crate::auth::rpc::constant_time_eq(expected.as_bytes(), signature.as_bytes()) {
        return Err("setup token 校验失败".to_string());
    }
    let bytes = hex_decode(encoded)?;
    let payload = serde_json::from_slice::<SetupPayload>(&bytes)
        .map_err(|err| format!("parse 2fa setup token failed: {err}"))?;
    if now_ts() - payload.issued_at > SETUP_TOKEN_TTL_SECS {
        return Err("setup token 已过期，请重新生成二维码".to_string());
    }
    Ok(payload)
}

fn encrypt_secret(secret: &str) -> String {
    xor_encrypt_hex("web-auth-2fa-secret", secret.as_bytes())
}

fn decrypt_secret(value: &str) -> Result<String, String> {
    let bytes = xor_decrypt_hex("web-auth-2fa-secret", value)?;
    String::from_utf8(bytes).map_err(|err| format!("decode 2fa secret failed: {err}"))
}

fn save_recovery_code_hashes(codes: &[String]) -> Result<(), String> {
    let hashes = codes
        .iter()
        .map(|code| hash_recovery_code(&normalize_recovery_code(code)))
        .collect::<Vec<_>>();
    save_recovery_code_hashes_hashed(&hashes)
}

fn save_recovery_code_hashes_hashed(hashes: &[String]) -> Result<(), String> {
    let raw = serde_json::to_string(hashes)
        .map_err(|err| format!("serialize recovery codes failed: {err}"))?;
    save_persisted_app_setting(APP_SETTING_WEB_ACCESS_2FA_RECOVERY_CODES_KEY, Some(&raw))
        .map(|_| ())
}

fn hash_recovery_code(code: &str) -> String {
    hex_sha256(
        format!(
            "codexmanager:web-auth-recovery-code:{}:{}",
            crate::rpc_auth_token(),
            code
        )
        .as_bytes(),
    )
}

fn two_factor_status_value(method: &str) -> Value {
    json!({
        "enabled": web_auth_two_factor_enabled(),
        "recoveryCodesRemaining": web_auth_two_factor_recovery_codes_remaining(),
        "method": method,
    })
}

fn normalize_totp_code(code: &str) -> String {
    let normalized = code
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if normalized.len() == 6 {
        normalized
    } else {
        String::new()
    }
}

fn normalize_recovery_code(code: &str) -> String {
    code.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .collect::<String>()
}

fn generate_recovery_codes() -> Vec<String> {
    (0..RECOVERY_CODE_COUNT)
        .map(|_| {
            let raw = random_code_chars(8);
            format!("{}-{}", &raw[..4], &raw[4..])
        })
        .collect()
}

fn random_code_chars(len: usize) -> String {
    let mut out = String::with_capacity(len);
    let mut rng = rand::rngs::OsRng;
    for _ in 0..len {
        let index = (rng.next_u32() as usize) % RECOVERY_CODE_ALPHABET.len();
        out.push(RECOVERY_CODE_ALPHABET[index] as char);
    }
    out
}

fn xor_encrypt_hex(scope: &str, bytes: &[u8]) -> String {
    let key = derive_key(scope);
    let mut out = Vec::with_capacity(bytes.len());
    for (index, byte) in bytes.iter().enumerate() {
        out.push(byte ^ key[index % key.len()]);
    }
    hex_encode(&out)
}

fn xor_decrypt_hex(scope: &str, value: &str) -> Result<Vec<u8>, String> {
    let bytes = hex_decode(value)?;
    let key = derive_key(scope);
    Ok(bytes
        .into_iter()
        .enumerate()
        .map(|(index, byte)| byte ^ key[index % key.len()])
        .collect())
}

fn derive_key(scope: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(format!("codexmanager:{scope}:{}", crate::rpc_auth_token()).as_bytes());
    hasher.finalize().to_vec()
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    hex_encode(digest.as_slice())
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    if normalized.len() % 2 != 0 {
        return Err("hex value 长度无效".to_string());
    }
    let mut out = Vec::with_capacity(normalized.len() / 2);
    let bytes = normalized.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let chunk = std::str::from_utf8(&bytes[index..index + 2])
            .map_err(|err| format!("hex decode failed: {err}"))?;
        let value = u8::from_str_radix(chunk, 16)
            .map_err(|err| format!("hex decode failed: {err}"))?;
        out.push(value);
        index += 2;
    }
    Ok(out)
}
