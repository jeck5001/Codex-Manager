import asyncio
import uuid
from typing import Dict, Optional

from fastapi import APIRouter, HTTPException, BackgroundTasks
from pydantic import BaseModel

from ...services.hotmail import (
    HotmailAccountArtifact,
    HotmailRegistrationEngine,
    HotmailRegistrationResult,
    build_default_hotmail_verification_provider,
)
from ...services.hotmail.artifacts import write_artifacts


router = APIRouter()
hotmail_batches: Dict[str, dict] = {}


def _format_hotmail_batch_error(result: HotmailRegistrationResult) -> str:
    message = str(result.error_message or result.reason_code or "unknown failure").strip()
    normalized = message.lower()
    if (
        str(result.reason_code or "").strip().lower() == "unsupported_challenge"
        or "unsupported_challenge" in normalized
        or "let's prove you're human" in normalized
        or "press and hold the button" in normalized
    ):
        return f"微软要求人工验证（Press and hold the button） | {message}"
    return message


def create_hotmail_engine(*, proxy_url: Optional[str] = None) -> HotmailRegistrationEngine:
    return HotmailRegistrationEngine(
        proxy_url=proxy_url,
        verification_provider=build_default_hotmail_verification_provider(),
    )


class HotmailBatchCreateRequest(BaseModel):
    count: int
    concurrency: int = 1
    interval_min: int = 1
    interval_max: int = 2
    proxy: Optional[str] = None


async def _run_hotmail_batch(batch_id: str, request: HotmailBatchCreateRequest):
    batch = hotmail_batches[batch_id]
    successful_records = []

    for _index in range(request.count):
        if batch.get("cancelled"):
            break

        engine = create_hotmail_engine(proxy_url=request.proxy)
        result = await asyncio.to_thread(engine.run)
        batch["completed"] += 1

        if result.success and result.artifact:
            artifact = result.artifact
            successful_records.append(
                {
                    "email": artifact.email,
                    "password": artifact.password,
                    "target_domain": artifact.target_domain,
                    "verification_email": artifact.verification_email,
                }
            )
            batch["success"] += 1
        else:
            batch["failed"] += 1
            batch["logs"].append(_format_hotmail_batch_error(result))
            if str(result.reason_code or "").strip().lower() == "unsupported_challenge":
                batch["status"] = "action_required"
                batch["action_required_reason"] = "unsupported_challenge"
                break

        if request.interval_max > 0:
            await asyncio.sleep(request.interval_min)

    batch["artifacts"] = write_artifacts(batch_id, successful_records)
    if batch.get("status") != "action_required":
        batch["finished"] = True


@router.post("/batches")
async def create_hotmail_batch(request: HotmailBatchCreateRequest, background_tasks: BackgroundTasks):
    batch_id = str(uuid.uuid4())
    hotmail_batches[batch_id] = {
        "batch_id": batch_id,
        "total": request.count,
        "completed": 0,
        "success": 0,
        "failed": 0,
        "status": "running",
        "action_required_reason": "",
        "finished": False,
        "cancelled": False,
        "logs": [],
        "artifacts": [],
    }
    background_tasks.add_task(_run_hotmail_batch, batch_id, request)
    return hotmail_batches[batch_id]


@router.get("/batches/{batch_id}")
async def get_hotmail_batch(batch_id: str):
    batch = hotmail_batches.get(batch_id)
    if not batch:
        raise HTTPException(status_code=404, detail="Hotmail batch not found")
    return batch


@router.get("/batches/{batch_id}/artifacts")
async def get_hotmail_batch_artifacts(batch_id: str):
    batch = hotmail_batches.get(batch_id)
    if not batch:
        raise HTTPException(status_code=404, detail="Hotmail batch not found")
    return {"batch_id": batch_id, "artifacts": batch.get("artifacts", [])}


@router.post("/batches/{batch_id}/cancel")
async def cancel_hotmail_batch(batch_id: str):
    batch = hotmail_batches.get(batch_id)
    if not batch:
        raise HTTPException(status_code=404, detail="Hotmail batch not found")
    batch["cancelled"] = True
    batch["status"] = "cancelled"
    return {"success": True, "batch_id": batch_id}
