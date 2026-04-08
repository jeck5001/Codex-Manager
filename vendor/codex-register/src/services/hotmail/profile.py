import random
import re
import string
from typing import List

from .types import HotmailRegistrationProfile


HOTMAIL_DOMAIN_POLICY = ("hotmail.com", "outlook.com")
DEFAULT_FIRST_NAMES = ("Alice", "Evelyn", "Nora", "Maya", "Liam", "Ethan", "Lucas", "Owen")
DEFAULT_LAST_NAMES = ("Johnson", "Taylor", "Miller", "Davis", "Clark", "Walker", "Moore", "Hall")


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


def build_password(seed: str = "") -> str:
    normalized_seed = _slug(seed)[:6]
    suffix = normalized_seed or "".join(random.choice(string.ascii_lowercase + string.digits) for _ in range(6))
    required = random.choice("!@#$%")
    body = "".join(random.choice(string.ascii_letters + string.digits) for _ in range(8))
    return f"{body}{required}{suffix[:4]}A1"


def build_registration_profile(seed: str = "") -> HotmailRegistrationProfile:
    first_name = random.choice(DEFAULT_FIRST_NAMES)
    last_name = random.choice(DEFAULT_LAST_NAMES)
    random_seed = seed or "".join(random.choice(string.ascii_lowercase + string.digits) for _ in range(4))
    return HotmailRegistrationProfile(
        first_name=first_name,
        last_name=last_name,
        birth_day=str(random.randint(1, 28)),
        birth_month=str(random.randint(1, 12)),
        birth_year=str(random.randint(1988, 2002)),
        password=build_password(random_seed),
        username_candidates=build_username_candidates(first_name, last_name, random_seed),
    )
