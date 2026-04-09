import asyncio
import uuid
from typing import Any, Dict, Optional

from fastapi import APIRouter, BackgroundTasks, HTTPException
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
hotmail_handoffs: Dict[str, dict] = {}


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


def _public_hotmail_batch(batch: dict) -> dict:
    return {
        "batch_id": batch["batch_id"],
        "total": batch["total"],
        "completed": batch["completed"],
        "success": batch["success"],
        "failed": batch["failed"],
        "status": batch.get("status", ""),
        "action_required_reason": batch.get("action_required_reason", ""),
        "handoff_id": batch.get("handoff_id", ""),
        "handoff_url": batch.get("handoff_url", ""),
        "handoff_title": batch.get("handoff_title", ""),
        "handoff_instructions": batch.get("handoff_instructions", ""),
        "finished": batch.get("finished", False),
        "cancelled": batch.get("cancelled", False),
        "logs": list(batch.get("logs", [])),
        "artifacts": list(batch.get("artifacts", [])),
    }


def _default_batch(batch_id: str, request: HotmailBatchCreateRequest) -> dict:
    return {
        "batch_id": batch_id,
        "total": request.count,
        "completed": 0,
        "success": 0,
        "failed": 0,
        "status": "running",
        "action_required_reason": "",
        "handoff_id": "",
        "handoff_url": "",
        "handoff_title": "",
        "handoff_instructions": "",
        "finished": False,
        "cancelled": False,
        "logs": [],
        "artifacts": [],
        "_request": request.model_dump(),
        "_successful_records": [],
    }


def _get_batch_or_404(batch_id: str) -> dict:
    batch = hotmail_batches.get(batch_id)
    if not batch:
        raise HTTPException(status_code=404, detail="Hotmail batch not found")
    return batch


def _handoff_payload(engine: Any, handoff_context: Any) -> dict[str, str]:
    payload_builder = getattr(engine, "build_handoff_payload", None)
    if callable(payload_builder):
        payload = payload_builder(handoff_context)
        if isinstance(payload, dict):
            return payload

    if isinstance(handoff_context, dict):
        return {
            "handoff_id": str(handoff_context.get("handoff_id") or "").strip(),
            "url": str(handoff_context.get("url") or "").strip(),
            "title": str(handoff_context.get("title") or "").strip(),
            "instructions": str(handoff_context.get("instructions") or "").strip(),
        }

    return {
        "handoff_id": str(getattr(handoff_context, "handoff_id", "") or "").strip(),
        "url": "",
        "title": "",
        "instructions": "",
    }


def _store_batch_handoff(batch: dict, engine: Any, handoff_context: Any) -> None:
    payload = _handoff_payload(engine, handoff_context)
    handoff_id = payload.get("handoff_id") or str(uuid.uuid4())
    if isinstance(handoff_context, dict):
        handoff_context.setdefault("handoff_id", handoff_id)

    previous_handoff_id = str(batch.get("handoff_id") or "").strip()
    if previous_handoff_id and previous_handoff_id != handoff_id:
        hotmail_handoffs.pop(previous_handoff_id, None)

    hotmail_handoffs[handoff_id] = {
        "engine": engine,
        "handoff_context": handoff_context,
    }
    batch["handoff_id"] = handoff_id
    batch["handoff_url"] = payload.get("url", "")
    batch["handoff_title"] = payload.get("title", "")
    batch["handoff_instructions"] = payload.get("instructions", "")


def _clear_batch_handoff(batch: dict) -> None:
    handoff_id = str(batch.get("handoff_id") or "").strip()
    if handoff_id:
        hotmail_handoffs.pop(handoff_id, None)
    batch["handoff_id"] = ""
    batch["handoff_url"] = ""
    batch["handoff_title"] = ""
    batch["handoff_instructions"] = ""
    batch["action_required_reason"] = ""


def _record_result(batch: dict, result: HotmailRegistrationResult) -> None:
    batch["completed"] += 1
    if result.success and result.artifact:
        artifact = result.artifact
        successful_records = batch.setdefault("_successful_records", [])
        successful_records.append(
            {
                "email": artifact.email,
                "password": artifact.password,
                "target_domain": artifact.target_domain,
                "verification_email": artifact.verification_email,
            }
        )
        batch["success"] += 1
        batch["artifacts"] = write_artifacts(batch["batch_id"], successful_records)
        return

    batch["failed"] += 1
    batch["logs"].append(_format_hotmail_batch_error(result))
    batch["artifacts"] = write_artifacts(batch["batch_id"], batch.setdefault("_successful_records", []))


async def _resume_remaining_batch(batch_id: str) -> None:
    batch = hotmail_batches.get(batch_id)
    if not batch or batch.get("cancelled"):
        return
    request = HotmailBatchCreateRequest(**batch["_request"])
    await _run_hotmail_batch(batch_id, request)


async def _run_hotmail_batch(batch_id: str, request: HotmailBatchCreateRequest):
    batch = hotmail_batches[batch_id]

    while batch["completed"] < batch["total"]:
        if batch.get("cancelled") or batch.get("status") == "action_required":
            break

        engine = create_hotmail_engine(proxy_url=request.proxy)
        result = await asyncio.to_thread(engine.run)
        handoff_context = getattr(result, "handoff_context", None)

        if handoff_context is not None:
            batch["logs"].append(_format_hotmail_batch_error(result))
            batch["status"] = "action_required"
            batch["action_required_reason"] = "unsupported_challenge"
            _store_batch_handoff(batch, engine, handoff_context)
            break

        _record_result(batch, result)

        if batch["completed"] < batch["total"] and request.interval_max > 0:
            await asyncio.sleep(request.interval_min)

    if batch.get("status") != "action_required":
        batch["finished"] = batch.get("cancelled", False) or batch["completed"] >= batch["total"]
        batch["status"] = "cancelled" if batch.get("cancelled") else ("finished" if batch["finished"] else "running")


@router.post("/batches")
async def create_hotmail_batch(request: HotmailBatchCreateRequest, background_tasks: BackgroundTasks):
    batch_id = str(uuid.uuid4())
    hotmail_batches[batch_id] = _default_batch(batch_id, request)
    background_tasks.add_task(_run_hotmail_batch, batch_id, request)
    return _public_hotmail_batch(hotmail_batches[batch_id])


@router.get("/batches/{batch_id}")
async def get_hotmail_batch(batch_id: str):
    return _public_hotmail_batch(_get_batch_or_404(batch_id))


@router.get("/batches/{batch_id}/artifacts")
async def get_hotmail_batch_artifacts(batch_id: str):
    batch = _get_batch_or_404(batch_id)
    return {"batch_id": batch_id, "artifacts": batch.get("artifacts", [])}


@router.post("/batches/{batch_id}/continue")
async def continue_hotmail_batch(batch_id: str):
    batch = _get_batch_or_404(batch_id)
    handoff_id = str(batch.get("handoff_id") or "").strip()
    if not handoff_id:
        raise HTTPException(status_code=409, detail="Hotmail batch has no pending handoff")

    handoff_entry = hotmail_handoffs.get(handoff_id)
    if not handoff_entry:
        raise HTTPException(status_code=410, detail="Hotmail handoff session expired")

    batch["status"] = "running"
    batch["action_required_reason"] = ""
    result = await asyncio.to_thread(
        handoff_entry["engine"].resume_handoff,
        handoff_entry["handoff_context"],
    )

    next_handoff = getattr(result, "handoff_context", None)
    if next_handoff is not None:
        batch["logs"].append(_format_hotmail_batch_error(result))
        batch["status"] = "action_required"
        batch["action_required_reason"] = "unsupported_challenge"
        _store_batch_handoff(batch, handoff_entry["engine"], next_handoff)
        return _public_hotmail_batch(batch)

    _clear_batch_handoff(batch)
    _record_result(batch, result)

    if not batch.get("cancelled") and batch["completed"] < batch["total"]:
        batch["status"] = "running"
        asyncio.create_task(_resume_remaining_batch(batch_id))
    else:
        batch["finished"] = True
        batch["status"] = "cancelled" if batch.get("cancelled") else "finished"

    return _public_hotmail_batch(batch)


@router.post("/batches/{batch_id}/abandon")
async def abandon_hotmail_batch(batch_id: str):
    batch = _get_batch_or_404(batch_id)
    handoff_id = str(batch.get("handoff_id") or "").strip()
    if not handoff_id:
        raise HTTPException(status_code=409, detail="Hotmail batch has no pending handoff")

    handoff_entry = hotmail_handoffs.get(handoff_id)
    if not handoff_entry:
        raise HTTPException(status_code=410, detail="Hotmail handoff session expired")

    abandon_fn = getattr(handoff_entry["engine"], "abandon_handoff", None)
    if callable(abandon_fn):
        await asyncio.to_thread(abandon_fn, handoff_entry["handoff_context"])

    _clear_batch_handoff(batch)
    _record_result(
        batch,
        HotmailRegistrationResult(
            success=False,
            reason_code="unsupported_challenge",
            error_message="Hotmail signup abandoned after manual handoff",
        ),
    )

    if not batch.get("cancelled") and batch["completed"] < batch["total"]:
        batch["status"] = "running"
        asyncio.create_task(_resume_remaining_batch(batch_id))
    else:
        batch["finished"] = True
        batch["status"] = "cancelled" if batch.get("cancelled") else "finished"

    return _public_hotmail_batch(batch)


@router.post("/batches/{batch_id}/cancel")
async def cancel_hotmail_batch(batch_id: str):
    batch = _get_batch_or_404(batch_id)
    handoff_id = str(batch.get("handoff_id") or "").strip()
    if handoff_id:
        handoff_entry = hotmail_handoffs.get(handoff_id)
        if handoff_entry:
            abandon_fn = getattr(handoff_entry["engine"], "abandon_handoff", None)
            if callable(abandon_fn):
                await asyncio.to_thread(abandon_fn, handoff_entry["handoff_context"])
        _clear_batch_handoff(batch)
    batch["cancelled"] = True
    batch["status"] = "cancelled"
    batch["finished"] = True
    return {"success": True, "batch_id": batch_id}
