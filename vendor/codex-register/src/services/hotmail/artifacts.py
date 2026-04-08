import json
from typing import Iterable


def build_accounts_txt(records: Iterable[dict]) -> str:
    return "\n".join(f"{item['email']}----{item['password']}" for item in records)


def build_accounts_json(records: Iterable[dict]) -> str:
    return json.dumps(list(records), ensure_ascii=False, indent=2)
