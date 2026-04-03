"""
Browserbase/DDG 注册配置 API 路由
"""

from typing import Any, Dict, List, Optional

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel

from ...database import crud
from ...database.session import get_db
from ...database.models import BrowserbaseConfig as BrowserbaseConfigModel

router = APIRouter()

SENSITIVE_FIELDS = {
    "ddg_token",
    "ddgToken",
    "browserbase_api_key",
    "browserbaseApiKey",
    "mail_inbox_url",
    "mailInboxUrl",
}


class BrowserbaseConfigCreate(BaseModel):
    name: str
    enabled: bool = True
    priority: int = 0
    config: Dict[str, Any]


class BrowserbaseConfigUpdate(BaseModel):
    name: Optional[str] = None
    enabled: Optional[bool] = None
    priority: Optional[int] = None
    config: Optional[Dict[str, Any]] = None


class BrowserbaseConfigResponse(BaseModel):
    id: int
    name: str
    enabled: bool
    priority: int
    config: Dict[str, Any]
    last_used: Optional[str] = None
    created_at: Optional[str] = None
    updated_at: Optional[str] = None

    class Config:
        from_attributes = True


class BrowserbaseConfigListResponse(BaseModel):
    total: int
    configs: List[BrowserbaseConfigResponse]


def _filter_sensitive_config(config: Optional[Dict[str, Any]]) -> Dict[str, Any]:
    filtered: Dict[str, Any] = {}
    for key, value in (config or {}).items():
        if key in SENSITIVE_FIELDS:
            filtered[f"has_{key}"] = bool(value)
        else:
            filtered[key] = value
    return filtered


def _config_to_response(config: BrowserbaseConfigModel, include_sensitive: bool = False) -> BrowserbaseConfigResponse:
    return BrowserbaseConfigResponse(
        id=config.id,
        name=config.name,
        enabled=config.enabled,
        priority=config.priority,
        config=(config.config or {}) if include_sensitive else _filter_sensitive_config(config.config),
        last_used=config.last_used.isoformat() if config.last_used else None,
        created_at=config.created_at.isoformat() if config.created_at else None,
        updated_at=config.updated_at.isoformat() if config.updated_at else None,
    )


@router.get("", response_model=BrowserbaseConfigListResponse)
async def list_browserbase_configs(
    enabled_only: bool = Query(False, description="只显示启用的配置"),
):
    with get_db() as db:
        configs = crud.get_browserbase_configs(db, enabled=True if enabled_only else None)
        return BrowserbaseConfigListResponse(
            total=len(configs),
            configs=[_config_to_response(item) for item in configs],
        )


@router.get("/{config_id}", response_model=BrowserbaseConfigResponse)
async def get_browserbase_config(config_id: int):
    with get_db() as db:
        config = crud.get_browserbase_config_by_id(db, config_id)
        if not config:
            raise HTTPException(status_code=404, detail="配置不存在")
        return _config_to_response(config)


@router.get("/{config_id}/full")
async def get_browserbase_config_full(config_id: int):
    with get_db() as db:
        config = crud.get_browserbase_config_by_id(db, config_id)
        if not config:
            raise HTTPException(status_code=404, detail="配置不存在")
        response = _config_to_response(config, include_sensitive=True)
        return {
            "id": response.id,
            "name": response.name,
            "enabled": response.enabled,
            "priority": response.priority,
            "config": response.config,
            "last_used": response.last_used,
            "created_at": response.created_at,
            "updated_at": response.updated_at,
        }


@router.post("", response_model=BrowserbaseConfigResponse)
async def create_browserbase_config(request: BrowserbaseConfigCreate):
    with get_db() as db:
        existing = db.query(BrowserbaseConfigModel).filter(BrowserbaseConfigModel.name == request.name).first()
        if existing:
            raise HTTPException(status_code=400, detail="配置名称已存在")
        config = crud.create_browserbase_config(
            db,
            name=request.name,
            enabled=request.enabled,
            priority=request.priority,
            config=request.config or {},
        )
        return _config_to_response(config)


@router.post("/{config_id}", response_model=BrowserbaseConfigResponse)
async def update_browserbase_config(config_id: int, request: BrowserbaseConfigUpdate):
    with get_db() as db:
        existing = crud.get_browserbase_config_by_id(db, config_id)
        if not existing:
            raise HTTPException(status_code=404, detail="配置不存在")
        if request.name:
            duplicate = (
                db.query(BrowserbaseConfigModel)
                .filter(
                    BrowserbaseConfigModel.name == request.name,
                    BrowserbaseConfigModel.id != config_id,
                )
                .first()
            )
            if duplicate:
                raise HTTPException(status_code=400, detail="配置名称已存在")
        updated = crud.update_browserbase_config(
            db,
            config_id,
            name=request.name,
            enabled=request.enabled,
            priority=request.priority,
            config=request.config if request.config is not None else None,
        )
        assert updated is not None
        return _config_to_response(updated)


@router.delete("/{config_id}")
async def delete_browserbase_config(config_id: int):
    with get_db() as db:
        if not crud.delete_browserbase_config(db, config_id):
            raise HTTPException(status_code=404, detail="配置不存在")
        return {"success": True, "message": "配置已删除"}
