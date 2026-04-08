import random
import re
import string
from typing import List


HOTMAIL_DOMAIN_POLICY = ("hotmail.com", "outlook.com")


def choose_target_domains() -> List[str]:
    return list(HOTMAIL_DOMAIN_POLICY)


def _slug(value: str) -> str:
    lowered = str(value or "").strip().lower()
    return re.sub(r"[^a-z0-9]+", "", lowered)


def build_username_candidates(first_name: str, last_name: str, seed: str) -> List[str]:
    base = f"{_slug(first_name)}{_slug(last_name)}{_slug(seed)}"
    trimmed = base[:28]
    if not trimmed:
        trimmed = "".join(random.choice(string.ascii_lowercase) for _ in range(10))
    return [trimmed, f"{trimmed}1", f"{trimmed}01"]
