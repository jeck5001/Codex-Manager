use reqwest::blocking::Client;
use std::sync::OnceLock;
use std::time::Duration;

static UPSTREAM_CLIENT: OnceLock<Client> = OnceLock::new();

pub(crate) const DEFAULT_MODELS_CLIENT_VERSION: &str = "0.98.0";
pub(crate) const DEFAULT_GATEWAY_DEBUG: bool = false;
const DEFAULT_UPSTREAM_CONNECT_TIMEOUT_SECS: u64 = 15;
const DEFAULT_ACCOUNT_MAX_INFLIGHT: usize = 0;

pub(crate) fn upstream_client() -> &'static Client {
    UPSTREAM_CLIENT.get_or_init(|| {
        Client::builder()
            // 中文注释：显式关闭总超时，避免长时流式响应在客户端层被误判超时中断。
            .timeout(None::<Duration>)
            // 中文注释：连接阶段设置超时，避免网络异常时线程长期卡死占满并发槽位。
            .connect_timeout(upstream_connect_timeout())
            .pool_max_idle_per_host(32)
            .pool_idle_timeout(Some(Duration::from_secs(90)))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

fn upstream_connect_timeout() -> Duration {
    Duration::from_secs(DEFAULT_UPSTREAM_CONNECT_TIMEOUT_SECS)
}

pub(crate) fn account_max_inflight_limit() -> usize {
    DEFAULT_ACCOUNT_MAX_INFLIGHT
}
