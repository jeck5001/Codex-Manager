"""
邮箱服务配置 API 路由
"""

import logging
from datetime import datetime, timedelta
from typing import List, Optional, Dict, Any, Tuple

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel

from ...database import crud
from ...database.session import get_db
from ...database.models import EmailService as EmailServiceModel
from ...config.settings import get_settings, update_settings
from ...services import (
    CloudflareTempMailProvisioner,
    EmailServiceFactory,
    EmailServiceType,
)

logger = logging.getLogger(__name__)
router = APIRouter()

TEMP_MAIL_DOMAIN_FAIL_400_COOLDOWN_THRESHOLD = 2
TEMP_MAIL_DOMAIN_FAIL_400_COOLDOWN_HOURS = 6


# ============== Pydantic Models ==============

class EmailServiceCreate(BaseModel):
    """创建邮箱服务请求"""
    service_type: str
    name: str
    config: Dict[str, Any]
    enabled: bool = True
    priority: int = 0


class EmailServiceUpdate(BaseModel):
    """更新邮箱服务请求"""
    name: Optional[str] = None
    config: Optional[Dict[str, Any]] = None
    enabled: Optional[bool] = None
    priority: Optional[int] = None


class EmailServiceResponse(BaseModel):
    """邮箱服务响应"""
    id: int
    service_type: str
    name: str
    enabled: bool
    priority: int
    config: Optional[Dict[str, Any]] = None  # 过滤敏感信息后的配置
    last_used: Optional[str] = None
    created_at: Optional[str] = None
    updated_at: Optional[str] = None

    class Config:
        from_attributes = True


class EmailServiceListResponse(BaseModel):
    """邮箱服务列表响应"""
    total: int
    services: List[EmailServiceResponse]


class ServiceTestResult(BaseModel):
    """服务测试结果"""
    success: bool
    message: str
    details: Optional[Dict[str, Any]] = None


class OutlookBatchImportRequest(BaseModel):
    """Outlook 批量导入请求"""
    data: str  # 多行数据，每行格式: 邮箱----密码 或 邮箱----密码----client_id----refresh_token
    enabled: bool = True
    priority: int = 0


class OutlookBatchImportResponse(BaseModel):
    """Outlook 批量导入响应"""
    total: int
    success: int
    failed: int
    accounts: List[Dict[str, Any]]
    errors: List[str]


# ============== Helper Functions ==============

# 敏感字段列表，返回响应时需要过滤
SENSITIVE_FIELDS = {'password', 'api_key', 'refresh_token', 'access_token', 'admin_password'}

def filter_sensitive_config(config: Dict[str, Any]) -> Dict[str, Any]:
    """过滤敏感配置信息"""
    if not config:
        return {}

    filtered = {}
    for key, value in config.items():
        if key in SENSITIVE_FIELDS:
            # 敏感字段不返回，但标记是否存在
            filtered[f"has_{key}"] = bool(value)
        else:
            filtered[key] = value

    # 为 Outlook 计算是否有 OAuth
    if config.get('client_id') and config.get('refresh_token'):
        filtered['has_oauth'] = True

    return filtered


def service_to_response(service: EmailServiceModel) -> EmailServiceResponse:
    """转换服务模型为响应"""
    return EmailServiceResponse(
        id=service.id,
        service_type=service.service_type,
        name=service.name,
        enabled=service.enabled,
        priority=service.priority,
        config=filter_sensitive_config(service.config),
        last_used=service.last_used.isoformat() if service.last_used else None,
        created_at=service.created_at.isoformat() if service.created_at else None,
        updated_at=service.updated_at.isoformat() if service.updated_at else None,
    )


def _build_temp_mail_worker_defaults(config: Dict[str, Any]) -> Dict[str, Any]:
    prepared_config = dict(config or {})
    settings = get_settings()

    if not str(prepared_config.get("base_url") or "").strip():
        prepared_config["base_url"] = str(getattr(settings, "temp_mail_base_url", "") or "").strip()

    if not str(prepared_config.get("admin_password") or "").strip():
        admin_password = getattr(settings, "temp_mail_admin_password", None)
        if admin_password is not None and hasattr(admin_password, "get_secret_value"):
            prepared_config["admin_password"] = admin_password.get_secret_value()
        else:
            prepared_config["admin_password"] = str(admin_password or "").strip()

    missing_keys = [
        key for key in ("base_url", "admin_password")
        if not str(prepared_config.get(key) or "").strip()
    ]
    if missing_keys:
        raise HTTPException(
            status_code=400,
            detail=f"缺少必需配置: {missing_keys}。请在 Cloudflare Temp-Mail 设置中补全全局 Worker 配置，或在当前服务里单独填写。",
        )

    return prepared_config


def _load_temp_mail_domain_configs(settings) -> List[Dict[str, Any]]:
    raw_configs = getattr(settings, "temp_mail_domain_configs", None)
    configs: List[Dict[str, Any]] = []
    if isinstance(raw_configs, list):
        for item in raw_configs:
            if isinstance(item, dict):
                configs.append(_normalize_temp_mail_domain_config(item))

    if configs:
        return configs

    legacy_domain_base = str(getattr(settings, "temp_mail_domain_base", "") or "").strip()
    legacy_zone_id = str(getattr(settings, "cloudflare_zone_id", "") or "").strip()
    if not legacy_domain_base or not legacy_zone_id:
        return []

    return [
        _normalize_temp_mail_domain_config(
            {
                "id": "legacy-default",
                "name": "默认域名配置",
                "zone_id": legacy_zone_id,
                "domain_base": legacy_domain_base,
                "subdomain_mode": str(getattr(settings, "temp_mail_subdomain_mode", "random") or "random"),
                "subdomain_length": int(getattr(settings, "temp_mail_subdomain_length", 6) or 6),
                "subdomain_prefix": str(getattr(settings, "temp_mail_subdomain_prefix", "tm") or "tm"),
                "sync_cloudflare_enabled": bool(getattr(settings, "temp_mail_sync_cloudflare_enabled", True)),
                "require_cloudflare_sync": bool(getattr(settings, "temp_mail_require_cloudflare_sync", True)),
            }
        )
    ]


def _normalize_temp_mail_domain_config(config: Dict[str, Any]) -> Dict[str, Any]:
    item = dict(config or {})
    item["id"] = str(item.get("id") or "").strip()
    item["name"] = str(item.get("name") or "").strip()
    item["zone_id"] = str(item.get("zone_id") or "").strip()
    item["domain_base"] = str(item.get("domain_base") or "").strip()
    item["subdomain_mode"] = str(item.get("subdomain_mode") or "random").strip() or "random"
    item["subdomain_length"] = int(item.get("subdomain_length") or 6)
    item["subdomain_prefix"] = str(item.get("subdomain_prefix") or "tm").strip() or "tm"
    item["sync_cloudflare_enabled"] = bool(item.get("sync_cloudflare_enabled", True))
    item["require_cloudflare_sync"] = bool(item.get("require_cloudflare_sync", True))
    item["enabled"] = bool(item.get("enabled", True))
    item["priority"] = int(item.get("priority") or 0)
    item["register_success_count"] = max(0, int(item.get("register_success_count") or 0))
    item["register_fail_400_count"] = max(0, int(item.get("register_fail_400_count") or 0))
    item["register_consecutive_fail_400"] = max(0, int(item.get("register_consecutive_fail_400") or 0))
    item["last_register_error"] = str(item.get("last_register_error") or "").strip()
    item["cooldown_until"] = str(item.get("cooldown_until") or "").strip()
    return item


def _normalize_requested_temp_mail_domain(value: Any) -> str:
    return str(value or "").strip().strip(".").lower()


def _temp_mail_domain_matches_base(domain: str, domain_base: str) -> bool:
    normalized_domain = _normalize_requested_temp_mail_domain(domain)
    normalized_base = _normalize_requested_temp_mail_domain(domain_base)
    if not normalized_domain or not normalized_base:
        return False
    return normalized_domain == normalized_base or normalized_domain.endswith(f".{normalized_base}")


def _match_temp_mail_domain_config_for_domain(
    domain: str,
    domain_configs: List[Dict[str, Any]],
) -> Optional[Dict[str, Any]]:
    normalized_domain = _normalize_requested_temp_mail_domain(domain)
    if not normalized_domain:
        return None

    normalized_configs = [
        _normalize_temp_mail_domain_config(item)
        for item in domain_configs
        if isinstance(item, dict)
    ]
    matches = [
        item
        for item in normalized_configs
        if _temp_mail_domain_matches_base(normalized_domain, item.get("domain_base") or "")
    ]
    if not matches:
        return None

    return sorted(
        matches,
        key=lambda item: (
            -len(str(item.get("domain_base") or "")),
            int(item.get("priority") or 0),
            str(item.get("id") or ""),
        ),
    )[0]


def _parse_temp_mail_domain_cooldown(value: Any) -> Optional[datetime]:
    text = str(value or "").strip()
    if not text:
        return None
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00")).replace(tzinfo=None)
    except Exception:
        return None


def _choose_temp_mail_domain_config(
    domain_configs: List[Dict[str, Any]],
    *,
    requested_id: str = "",
) -> Optional[Dict[str, Any]]:
    normalized = [_normalize_temp_mail_domain_config(item) for item in domain_configs if isinstance(item, dict)]
    if not normalized:
        return None

    requested = str(requested_id or "").strip()
    if requested:
        return next((item for item in normalized if item["id"] == requested), None)

    enabled_configs = [item for item in normalized if item.get("enabled", True)]
    candidates = enabled_configs or normalized
    now = datetime.utcnow()
    ready_configs = []
    for item in candidates:
        cooldown_until = _parse_temp_mail_domain_cooldown(item.get("cooldown_until"))
        if cooldown_until and cooldown_until > now:
            continue
        ready_configs.append(item)
    candidates = ready_configs or candidates

    return sorted(
        candidates,
        key=lambda item: (
            int(item.get("priority") or 0),
            -int(item.get("register_success_count") or 0),
            int(item.get("register_fail_400_count") or 0),
            int(item.get("register_consecutive_fail_400") or 0),
            str(item.get("id") or ""),
        ),
    )[0]


def record_temp_mail_domain_registration_outcome(
    domain_config_id: str,
    *,
    success: bool,
    failure_http_status: Optional[int] = None,
    error_message: str = "",
) -> Optional[Dict[str, Any]]:
    normalized_id = str(domain_config_id or "").strip()
    if not normalized_id:
        return None

    settings = get_settings()
    configs = _load_temp_mail_domain_configs(settings)
    if not configs:
        return None

    updated_configs: List[Dict[str, Any]] = []
    updated_target: Optional[Dict[str, Any]] = None
    now = datetime.utcnow()

    for item in configs:
        normalized = _normalize_temp_mail_domain_config(item)
        if normalized["id"] != normalized_id:
            updated_configs.append(normalized)
            continue

        if success:
            normalized["register_success_count"] += 1
            normalized["register_consecutive_fail_400"] = 0
            normalized["cooldown_until"] = ""
            normalized["last_register_error"] = ""
        else:
            normalized["last_register_error"] = str(error_message or "").strip()
            if int(failure_http_status or 0) == 400:
                normalized["register_fail_400_count"] += 1
                normalized["register_consecutive_fail_400"] += 1
                if normalized["register_consecutive_fail_400"] >= TEMP_MAIL_DOMAIN_FAIL_400_COOLDOWN_THRESHOLD:
                    normalized["cooldown_until"] = (
                        now + timedelta(hours=TEMP_MAIL_DOMAIN_FAIL_400_COOLDOWN_HOURS)
                    ).isoformat(timespec="seconds")

        updated_target = normalized
        updated_configs.append(normalized)

    if updated_target is None:
        return None

    update_settings(temp_mail_domain_configs=updated_configs)
    return updated_target


def _apply_temp_mail_domain_config(config: Dict[str, Any], settings) -> Dict[str, Any]:
    prepared_config = dict(config or {})
    requested_domain = _normalize_requested_temp_mail_domain(prepared_config.get("domain"))
    if requested_domain:
        prepared_config["domain"] = requested_domain

    explicit_domain_base = str(prepared_config.get("temp_mail_domain_base") or "").strip()
    explicit_zone_id = str(prepared_config.get("cloudflare_zone_id") or "").strip()
    if explicit_domain_base and explicit_zone_id:
        if requested_domain and not _temp_mail_domain_matches_base(requested_domain, explicit_domain_base):
            raise HTTPException(
                status_code=400,
                detail=f"指定域名不属于当前基础域名: {requested_domain}",
            )
        return prepared_config

    domain_configs = _load_temp_mail_domain_configs(settings)
    requested_id = str(prepared_config.get("domain_config_id") or "").strip()
    selected_config = None

    if requested_id:
        selected_config = _choose_temp_mail_domain_config(
            domain_configs,
            requested_id=requested_id,
        )
        if not selected_config:
            raise HTTPException(status_code=400, detail=f"域名配置不存在: {requested_id}")
        if requested_domain and not _temp_mail_domain_matches_base(
            requested_domain,
            str(selected_config.get("domain_base") or ""),
        ):
            raise HTTPException(
                status_code=400,
                detail=f"指定域名不属于所选域名配置: {requested_domain}",
            )
    elif requested_domain:
        selected_config = _match_temp_mail_domain_config_for_domain(requested_domain, domain_configs)
        if len(domain_configs) > 0 and not selected_config:
            raise HTTPException(
                status_code=400,
                detail=f"指定域名未匹配任何 Temp-Mail 域名配置: {requested_domain}",
            )
    else:
        selected_config = _choose_temp_mail_domain_config(
            domain_configs,
            requested_id=requested_id,
        )
        if len(domain_configs) == 0:
            return prepared_config
        prepared_config.setdefault("domain_config_id", str((selected_config or {}).get("id") or "").strip())

    if not selected_config:
        return prepared_config

    prepared_config.setdefault("domain_config_id", str(selected_config.get("id") or "").strip())
    prepared_config.setdefault("domain_config_name", str(selected_config.get("name") or "").strip())
    prepared_config.setdefault("cloudflare_zone_id", str(selected_config.get("zone_id") or "").strip())
    prepared_config.setdefault("temp_mail_domain_base", str(selected_config.get("domain_base") or "").strip())
    prepared_config.setdefault(
        "temp_mail_subdomain_mode",
        str(selected_config.get("subdomain_mode") or "random").strip() or "random",
    )
    prepared_config.setdefault(
        "temp_mail_subdomain_length",
        int(selected_config.get("subdomain_length") or 6),
    )
    prepared_config.setdefault(
        "temp_mail_subdomain_prefix",
        str(selected_config.get("subdomain_prefix") or "").strip(),
    )
    prepared_config.setdefault(
        "temp_mail_sync_cloudflare_enabled",
        bool(selected_config.get("sync_cloudflare_enabled", True)),
    )
    prepared_config.setdefault(
        "temp_mail_require_cloudflare_sync",
        bool(selected_config.get("require_cloudflare_sync", True)),
    )
    return prepared_config


def _build_temp_mail_provisioner_overrides(config: Dict[str, Any]) -> Dict[str, Any]:
    overrides: Dict[str, Any] = {}
    field_names = (
        "cloudflare_zone_id",
        "temp_mail_domain_base",
        "temp_mail_subdomain_mode",
        "temp_mail_subdomain_length",
        "temp_mail_subdomain_prefix",
        "temp_mail_sync_cloudflare_enabled",
        "temp_mail_require_cloudflare_sync",
    )
    for field_name in field_names:
        if field_name in config and config.get(field_name) not in (None, ""):
            overrides[field_name] = config.get(field_name)
    return overrides


def _prepare_temp_mail_config_for_create(config: Dict[str, Any]) -> Tuple[Dict[str, Any], Dict[str, Any]]:
    """创建 temp_mail 服务前，先在 Cloudflare 预配固定域名并回填配置。"""
    settings = get_settings()
    prepared_config = _build_temp_mail_worker_defaults(config)
    prepared_config = _apply_temp_mail_domain_config(prepared_config, settings)
    rollback_only_keys = {"cloudflare_worker_previous_bindings"}
    requested_domain = _normalize_requested_temp_mail_domain(prepared_config.get("domain"))
    if requested_domain:
        prepared_config["domain"] = requested_domain

    try:
        provisioner = CloudflareTempMailProvisioner(
            settings,
            overrides=_build_temp_mail_provisioner_overrides(prepared_config),
        )
        if requested_domain:
            provision_result = provisioner.provision_domain(requested_domain=requested_domain)
        else:
            provision_result = provisioner.provision_domain()
    except Exception as exc:
        logger.error(f"Temp-Mail 域名预配失败: {exc}")
        raise HTTPException(status_code=502, detail=f"Temp-Mail 域名预配失败: {exc}") from exc

    if isinstance(provision_result, dict) and "persisted_config" in provision_result:
        persisted_config = dict(provision_result.get("persisted_config") or {})
        cleanup_payload = dict(provision_result.get("cleanup_context") or {})
    else:
        # backward-compatible shape
        persisted_config = dict(provision_result or {})
        cleanup_payload = dict(provision_result or {})

    for key in rollback_only_keys:
        persisted_config.pop(key, None)

    domain = str(persisted_config.get("domain") or "").strip()
    if not domain:
        raise HTTPException(status_code=502, detail="Temp-Mail 域名预配失败: 未返回有效 domain")

    merged_config = {
        **prepared_config,
        **persisted_config,
        "domain": domain,
    }
    cleanup_payload.setdefault("domain", domain)
    cleanup_context = {
        "provisioner": provisioner,
        "provisioned": cleanup_payload,
        "domain": domain,
    }
    return merged_config, cleanup_context


def build_temp_mail_service_for_registration(
    config: Dict[str, Any],
    *,
    owner_task_uuid: Optional[str] = None,
    owner_batch_id: Optional[str] = None,
) -> Tuple[Dict[str, Any], Dict[str, Any], str]:
    """Prepare a temp-mail service config for automatic registration use."""
    merged_config, cleanup_context = _prepare_temp_mail_config_for_create(config)
    domain = str(merged_config.get("domain") or "").strip()
    if not domain:
        raise HTTPException(status_code=502, detail="Temp-Mail 域名预配失败: 未返回有效 domain")

    merged_config["auto_created_for_registration"] = True
    merged_config["auto_cleanup"] = True
    if owner_task_uuid:
        merged_config["owner_task_uuid"] = owner_task_uuid
    if owner_batch_id:
        merged_config["owner_batch_id"] = owner_batch_id

    return merged_config, cleanup_context, domain


def _cleanup_temp_mail_provisioning(cleanup_context: Optional[Dict[str, Any]]) -> None:
    if not cleanup_context:
        return

    provisioner = cleanup_context.get("provisioner")
    provisioned = cleanup_context.get("provisioned")
    domain = cleanup_context.get("domain")
    cleanup_fn = getattr(provisioner, "cleanup_provisioned_domain", None)
    if not callable(cleanup_fn):
        return

    try:
        cleanup_fn(provisioned=provisioned, domain=domain)
    except Exception as exc:
        logger.error(f"Temp-Mail 远端回滚失败: {exc}")


# ============== API Endpoints ==============

@router.get("/stats")
async def get_email_services_stats():
    """获取邮箱服务统计信息"""
    with get_db() as db:
        from sqlalchemy import func

        # 按类型统计
        type_stats = db.query(
            EmailServiceModel.service_type,
            func.count(EmailServiceModel.id)
        ).group_by(EmailServiceModel.service_type).all()

        # 启用数量
        enabled_count = db.query(func.count(EmailServiceModel.id)).filter(
            EmailServiceModel.enabled == True
        ).scalar()

        stats = {
            'outlook_count': 0,
            'custom_count': 0,
            'temp_mail_count': 0,
            'mail_33_imap_count': 0,
            'generator_email_count': 0,
            'tempmail_available': True,  # 临时邮箱始终可用
            'enabled_count': enabled_count
        }

        for service_type, count in type_stats:
            if service_type == 'outlook':
                stats['outlook_count'] = count
            elif service_type == 'custom_domain':
                stats['custom_count'] = count
            elif service_type == 'temp_mail':
                stats['temp_mail_count'] = count
            elif service_type == 'mail_33_imap':
                stats['mail_33_imap_count'] = count
            elif service_type == 'generator_email':
                stats['generator_email_count'] = count

        return stats


@router.get("/types")
async def get_service_types():
    """获取支持的邮箱服务类型"""
    return {
        "types": [
            {
                "value": "tempmail",
                "label": "Tempmail.lol",
                "description": "临时邮箱服务，无需配置",
                "config_fields": [
                    {"name": "base_url", "label": "API 地址", "default": "https://api.tempmail.lol/v2", "required": False},
                    {"name": "timeout", "label": "超时时间", "default": 30, "required": False},
                ]
            },
            {
                "value": "outlook",
                "label": "Outlook",
                "description": "Outlook 邮箱，需要配置账户信息",
                "config_fields": [
                    {"name": "email", "label": "邮箱地址", "required": True},
                    {"name": "password", "label": "密码", "required": True},
                    {"name": "client_id", "label": "OAuth Client ID", "required": False},
                    {"name": "refresh_token", "label": "OAuth Refresh Token", "required": False},
                ]
            },
            {
                "value": "custom_domain",
                "label": "自定义域名",
                "description": "自定义域名邮箱服务",
                "config_fields": [
                    {"name": "base_url", "label": "API 地址", "required": True},
                    {"name": "api_key", "label": "API Key", "required": True},
                    {"name": "default_domain", "label": "默认域名", "required": False},
                ]
            },
            {
                "value": "temp_mail",
                "label": "Temp-Mail（自部署）",
                "description": "自部署 Cloudflare Worker 临时邮箱，admin 模式管理",
                "config_fields": [
                    {
                        "name": "base_url",
                        "label": "Worker 地址",
                        "required": False,
                        "placeholder": "https://mail.example.com",
                        "description": "留空时使用上方 Cloudflare Temp-Mail 设置里的全局 Worker 地址",
                    },
                    {
                        "name": "admin_password",
                        "label": "Admin 密码",
                        "required": False,
                        "secret": True,
                        "description": "留空时使用上方 Cloudflare Temp-Mail 设置里的全局 Admin 密码",
                    },
                    {
                        "name": "domain",
                        "label": "邮箱域名（可选）",
                        "required": False,
                        "description": "可直接填写完整域名；留空时由服务端自动生成固定子域名",
                    },
                    {"name": "enable_prefix", "label": "启用前缀", "required": False, "default": True},
                ]
            },
            {
                "value": "mail_33_imap",
                "label": "33mail + IMAP",
                "description": "使用 33mail 生成别名，通过真实邮箱 IMAP 自动收取 OpenAI 验证码",
                "config_fields": [
                    {"name": "alias_domain", "label": "33mail 域名后缀", "required": True, "placeholder": "demo.33mail.com", "description": "不要带 @，直接填写 33mail 分配的域名后缀"},
                    {"name": "real_inbox_email", "label": "真实收件邮箱", "required": True, "placeholder": "name@example.com", "description": "33mail 转发的真实目标邮箱"},
                    {"name": "imap_host", "label": "IMAP Host", "required": True, "placeholder": "imap.qq.com"},
                    {"name": "imap_port", "label": "IMAP Port", "required": True, "default": 993},
                    {"name": "imap_username", "label": "IMAP 用户名", "required": True, "placeholder": "name@example.com"},
                    {"name": "imap_password", "label": "IMAP 密码/授权码", "required": True, "secret": True},
                    {"name": "imap_mailbox", "label": "邮箱目录", "required": False, "default": "INBOX"},
                    {"name": "imap_ssl", "label": "启用 SSL", "required": False, "default": True},
                    {
                        "name": "from_filter",
                        "label": "发件人过滤",
                        "required": False,
                        "default": "openai.com",
                        "placeholder": "openai.com, sender@mailer1.33mail.com",
                        "description": "支持多个值，逗号或 、 分隔；留空表示不过滤发件人",
                    },
                    {
                        "name": "subject_keyword",
                        "label": "主题关键字",
                        "required": False,
                        "default": "Your ChatGPT code is",
                        "placeholder": "Your ChatGPT code is",
                        "description": "支持多个值，逗号或 、 分隔；留空表示不过滤主题/正文关键字",
                    },
                    {"name": "otp_pattern", "label": "验证码正则", "required": False, "default": "(?<!\\\\d)(\\\\d{6})(?!\\\\d)"},
                    {"name": "poll_interval", "label": "轮询间隔(秒)", "required": False, "default": 3},
                    {"name": "timeout", "label": "超时时间(秒)", "required": False, "default": 120},
                    {"name": "alias_length", "label": "别名前缀长度", "required": False, "default": 12},
                ]
            },
            {
                "value": "generator_email",
                "label": "Generator.email",
                "description": "Generator.email 临时邮箱服务",
                "config_fields": [
                    {"name": "base_url", "label": "Base URL", "required": False, "default": "https://generator.email"},
                    {"name": "timeout", "label": "请求超时(秒)", "required": False, "default": 30},
                    {"name": "poll_interval", "label": "轮询间隔(秒)", "required": False, "default": 3},
                ],
            }
        ]
    }


@router.get("", response_model=EmailServiceListResponse)
async def list_email_services(
    service_type: Optional[str] = Query(None, description="服务类型筛选"),
    enabled_only: bool = Query(False, description="只显示启用的服务"),
):
    """获取邮箱服务列表"""
    with get_db() as db:
        query = db.query(EmailServiceModel)

        if service_type:
            query = query.filter(EmailServiceModel.service_type == service_type)

        if enabled_only:
            query = query.filter(EmailServiceModel.enabled == True)

        services = query.order_by(EmailServiceModel.priority.asc(), EmailServiceModel.id.asc()).all()

        return EmailServiceListResponse(
            total=len(services),
            services=[service_to_response(s) for s in services]
        )


@router.get("/{service_id}", response_model=EmailServiceResponse)
async def get_email_service(service_id: int):
    """获取单个邮箱服务详情"""
    with get_db() as db:
        service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
        if not service:
            raise HTTPException(status_code=404, detail="服务不存在")
        return service_to_response(service)


@router.get("/{service_id}/full")
async def get_email_service_full(service_id: int):
    """获取单个邮箱服务完整详情（包含敏感字段，用于编辑）"""
    with get_db() as db:
        service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
        if not service:
            raise HTTPException(status_code=404, detail="服务不存在")

        return {
            "id": service.id,
            "service_type": service.service_type,
            "name": service.name,
            "enabled": service.enabled,
            "priority": service.priority,
            "config": service.config or {},  # 返回完整配置
            "last_used": service.last_used.isoformat() if service.last_used else None,
            "created_at": service.created_at.isoformat() if service.created_at else None,
            "updated_at": service.updated_at.isoformat() if service.updated_at else None,
        }


@router.post("", response_model=EmailServiceResponse)
async def create_email_service(request: EmailServiceCreate):
    """创建邮箱服务配置"""
    # 验证服务类型
    try:
        EmailServiceType(request.service_type)
    except ValueError:
        raise HTTPException(status_code=400, detail=f"无效的服务类型: {request.service_type}")

    with get_db() as db:
        service_name = str(request.name or "").strip()
        if service_name:
            existing = db.query(EmailServiceModel).filter(EmailServiceModel.name == service_name).first()
            if existing:
                raise HTTPException(status_code=400, detail="服务名称已存在")

        config = dict(request.config or {})
        cleanup_context = None
        if request.service_type == EmailServiceType.TEMP_MAIL.value:
            config, cleanup_context = _prepare_temp_mail_config_for_create(config)
            if not service_name:
                service_name = str(config.get("domain") or "").strip()
                existing = db.query(EmailServiceModel).filter(EmailServiceModel.name == service_name).first()
                if existing:
                    _cleanup_temp_mail_provisioning(cleanup_context)
                    raise HTTPException(status_code=400, detail="服务名称已存在")

        service = EmailServiceModel(
            service_type=request.service_type,
            name=service_name,
            config=config,
            enabled=request.enabled,
            priority=request.priority
        )

        try:
            db.add(service)
            db.commit()
        except Exception as exc:
            rollback_fn = getattr(db, "rollback", None)
            if callable(rollback_fn):
                rollback_fn()
            if request.service_type == EmailServiceType.TEMP_MAIL.value:
                _cleanup_temp_mail_provisioning(cleanup_context)
            logger.error(f"创建邮箱服务失败: {exc}")
            raise HTTPException(status_code=500, detail=f"创建邮箱服务失败: {exc}") from exc

        try:
            db.refresh(service)
        except Exception as exc:
            logger.error(f"创建邮箱服务后刷新失败: {exc}")
            raise HTTPException(status_code=500, detail=f"创建邮箱服务后刷新失败: {exc}") from exc

        return service_to_response(service)


@router.patch("/{service_id}", response_model=EmailServiceResponse)
async def update_email_service(service_id: int, request: EmailServiceUpdate):
    """更新邮箱服务配置"""
    with get_db() as db:
        service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
        if not service:
            raise HTTPException(status_code=404, detail="服务不存在")

        update_data = {}
        if request.name is not None:
            update_data["name"] = request.name
        if request.config is not None:
            # 合并配置而不是替换
            current_config = service.config or {}
            incoming_config = dict(request.config)

            if service.service_type == EmailServiceType.TEMP_MAIL.value and "domain" in incoming_config:
                raise HTTPException(
                    status_code=400,
                    detail="temp_mail 服务不支持通过更新接口提交 config.domain",
                )

            merged_config = {**current_config, **incoming_config}
            # 仅移除显式 None，保留空字符串/false/0，允许清空过滤条件和关闭布尔配置。
            merged_config = {k: v for k, v in merged_config.items() if v is not None}
            update_data["config"] = merged_config
        if request.enabled is not None:
            update_data["enabled"] = request.enabled
        if request.priority is not None:
            update_data["priority"] = request.priority

        for key, value in update_data.items():
            setattr(service, key, value)

        db.commit()
        db.refresh(service)

        return service_to_response(service)


@router.delete("/{service_id}")
async def delete_email_service(service_id: int):
    """删除邮箱服务配置"""
    with get_db() as db:
        service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
        if not service:
            raise HTTPException(status_code=404, detail="服务不存在")

        db.delete(service)
        db.commit()

        return {"success": True, "message": f"服务 {service.name} 已删除"}


@router.post("/{service_id}/test", response_model=ServiceTestResult)
async def test_email_service(service_id: int):
    """测试邮箱服务是否可用"""
    with get_db() as db:
        service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
        if not service:
            raise HTTPException(status_code=404, detail="服务不存在")

        try:
            service_type = EmailServiceType(service.service_type)
            email_service = EmailServiceFactory.create(service_type, service.config, name=service.name)

            health = email_service.check_health()

            if health:
                return ServiceTestResult(
                    success=True,
                    message="服务连接正常",
                    details=email_service.get_service_info() if hasattr(email_service, 'get_service_info') else None
                )
            else:
                return ServiceTestResult(
                    success=False,
                    message="服务连接失败"
                )

        except Exception as e:
            logger.error(f"测试邮箱服务失败: {e}")
            return ServiceTestResult(
                success=False,
                message=f"测试失败: {str(e)}"
            )


@router.post("/{service_id}/enable")
async def enable_email_service(service_id: int):
    """启用邮箱服务"""
    with get_db() as db:
        service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
        if not service:
            raise HTTPException(status_code=404, detail="服务不存在")

        service.enabled = True
        db.commit()

        return {"success": True, "message": f"服务 {service.name} 已启用"}


@router.post("/{service_id}/disable")
async def disable_email_service(service_id: int):
    """禁用邮箱服务"""
    with get_db() as db:
        service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
        if not service:
            raise HTTPException(status_code=404, detail="服务不存在")

        service.enabled = False
        db.commit()

        return {"success": True, "message": f"服务 {service.name} 已禁用"}


@router.post("/reorder")
async def reorder_services(service_ids: List[int]):
    """重新排序邮箱服务优先级"""
    with get_db() as db:
        for index, service_id in enumerate(service_ids):
            service = db.query(EmailServiceModel).filter(EmailServiceModel.id == service_id).first()
            if service:
                service.priority = index

        db.commit()

        return {"success": True, "message": "优先级已更新"}


@router.post("/outlook/batch-import", response_model=OutlookBatchImportResponse)
async def batch_import_outlook(request: OutlookBatchImportRequest):
    """
    批量导入 Outlook 邮箱账户

    支持两种格式：
    - 格式一（密码认证）：邮箱----密码
    - 格式二（XOAUTH2 认证）：邮箱----密码----client_id----refresh_token

    每行一个账户，使用四个连字符（----）分隔字段
    """
    lines = request.data.strip().split("\n")
    total = len(lines)
    success = 0
    failed = 0
    accounts = []
    errors = []

    with get_db() as db:
        for i, line in enumerate(lines):
            line = line.strip()

            # 跳过空行和注释
            if not line or line.startswith("#"):
                continue

            parts = line.split("----")

            # 验证格式
            if len(parts) < 2:
                failed += 1
                errors.append(f"行 {i+1}: 格式错误，至少需要邮箱和密码")
                continue

            email = parts[0].strip()
            password = parts[1].strip()

            # 验证邮箱格式
            if "@" not in email:
                failed += 1
                errors.append(f"行 {i+1}: 无效的邮箱地址: {email}")
                continue

            # 检查是否已存在
            existing = db.query(EmailServiceModel).filter(
                EmailServiceModel.service_type == "outlook",
                EmailServiceModel.name == email
            ).first()

            if existing:
                failed += 1
                errors.append(f"行 {i+1}: 邮箱已存在: {email}")
                continue

            # 构建配置
            config = {
                "email": email,
                "password": password
            }

            # 检查是否有 OAuth 信息（格式二）
            if len(parts) >= 4:
                client_id = parts[2].strip()
                refresh_token = parts[3].strip()
                if client_id and refresh_token:
                    config["client_id"] = client_id
                    config["refresh_token"] = refresh_token

            # 创建服务记录
            try:
                service = EmailServiceModel(
                    service_type="outlook",
                    name=email,
                    config=config,
                    enabled=request.enabled,
                    priority=request.priority
                )
                db.add(service)
                db.commit()
                db.refresh(service)

                accounts.append({
                    "id": service.id,
                    "email": email,
                    "has_oauth": bool(config.get("client_id")),
                    "name": email
                })
                success += 1

            except Exception as e:
                failed += 1
                errors.append(f"行 {i+1}: 创建失败: {str(e)}")
                db.rollback()

    return OutlookBatchImportResponse(
        total=total,
        success=success,
        failed=failed,
        accounts=accounts,
        errors=errors
    )


@router.delete("/outlook/batch")
async def batch_delete_outlook(service_ids: List[int]):
    """批量删除 Outlook 邮箱服务"""
    deleted = 0
    with get_db() as db:
        for service_id in service_ids:
            service = db.query(EmailServiceModel).filter(
                EmailServiceModel.id == service_id,
                EmailServiceModel.service_type == "outlook"
            ).first()
            if service:
                db.delete(service)
                deleted += 1
        db.commit()

    return {"success": True, "deleted": deleted, "message": f"已删除 {deleted} 个服务"}


# ============== 临时邮箱测试 ==============

class TempmailTestRequest(BaseModel):
    """临时邮箱测试请求"""
    api_url: Optional[str] = None


@router.post("/test-tempmail")
async def test_tempmail_service(request: TempmailTestRequest):
    """测试临时邮箱服务是否可用"""
    try:
        from ...services import EmailServiceFactory, EmailServiceType
        from ...config.settings import get_settings

        settings = get_settings()
        base_url = request.api_url or settings.tempmail_base_url

        config = {"base_url": base_url}
        tempmail = EmailServiceFactory.create(EmailServiceType.TEMPMAIL, config)

        # 检查服务健康状态
        health = tempmail.check_health()

        if health:
            return {"success": True, "message": "临时邮箱连接正常"}
        else:
            return {"success": False, "message": "临时邮箱连接失败"}

    except Exception as e:
        logger.error(f"测试临时邮箱失败: {e}")
        return {"success": False, "message": f"测试失败: {str(e)}"}
