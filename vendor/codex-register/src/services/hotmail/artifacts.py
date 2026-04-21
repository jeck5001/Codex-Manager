import json
import os
from pathlib import Path
from tempfile import gettempdir
from typing import Iterable


def build_accounts_txt(records: Iterable[dict]) -> str:
    return "\n".join(f"{item['email']}----{item['password']}" for item in records)


def build_accounts_json(records: Iterable[dict]) -> str:
    return json.dumps(list(records), ensure_ascii=False, indent=2)


def _resolve_artifacts_root() -> Path:
    override = (os.environ.get("HOTMAIL_ARTIFACTS_ROOT") or "").strip()
    if override:
        return Path(override)
    persistent = Path("/app/data/hotmail-artifacts")
    try:
        persistent.mkdir(parents=True, exist_ok=True)
        return persistent
    except (OSError, PermissionError):
        return Path(gettempdir()) / "codex-register" / "hotmail"


def write_artifacts(batch_id: str, records: Iterable[dict]) -> list[dict]:
    data = list(records)
    root = _resolve_artifacts_root() / batch_id
    root.mkdir(parents=True, exist_ok=True)

    json_path = root / "accounts.json"
    txt_path = root / "accounts.txt"

    json_path.write_text(build_accounts_json(data), encoding="utf-8")
    txt_path.write_text(build_accounts_txt(data), encoding="utf-8")

    return [
        {"name": json_path.name, "path": str(json_path)},
        {"name": txt_path.name, "path": str(txt_path)},
    ]
