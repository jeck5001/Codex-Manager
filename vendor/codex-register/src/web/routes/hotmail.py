import uuid
from typing import Dict

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel


router = APIRouter()
hotmail_batches: Dict[str, dict] = {}


class HotmailBatchCreateRequest(BaseModel):
    count: int
    concurrency: int = 1
    interval_min: int = 1
    interval_max: int = 2


@router.post("/batches")
async def create_hotmail_batch(request: HotmailBatchCreateRequest):
    batch_id = str(uuid.uuid4())
    hotmail_batches[batch_id] = {
        "batch_id": batch_id,
        "total": request.count,
        "completed": 0,
        "success": 0,
        "failed": 0,
        "finished": False,
        "logs": [],
        "artifacts": [],
    }
    return hotmail_batches[batch_id]


@router.get("/batches/{batch_id}")
async def get_hotmail_batch(batch_id: str):
    batch = hotmail_batches.get(batch_id)
    if not batch:
        raise HTTPException(status_code=404, detail="Hotmail batch not found")
    return batch
