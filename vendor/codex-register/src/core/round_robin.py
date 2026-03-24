"""
轮询选择辅助函数
"""

from datetime import datetime
from typing import Callable, Iterable, Optional, TypeVar

T = TypeVar("T")


def pick_round_robin_item(
    items: Iterable[T],
    *,
    priority_getter: Callable[[T], object] = lambda item: getattr(item, "priority", 0),
    last_used_getter: Callable[[T], object] = lambda item: getattr(item, "last_used", None),
    id_getter: Callable[[T], object] = lambda item: getattr(item, "id", 0),
) -> Optional[T]:
    """
    选择最适合用于下一次轮询的对象。

    规则：
    1. `priority` 越小越优先
    2. `last_used` 越早越优先，`None` 视为从未使用，优先级最高
    3. 最后按 `id` 稳定排序，避免结果抖动
    """

    candidates = list(items)
    if not candidates:
        return None

    def _normalize_priority(value: object) -> int:
        return value if isinstance(value, int) else 0

    def _normalize_last_used(value: object) -> tuple[int, datetime]:
        if isinstance(value, datetime):
            return (1, value)
        return (0, datetime.min)

    def _normalize_id(value: object) -> int:
        return value if isinstance(value, int) else 0

    candidates.sort(
        key=lambda item: (
            _normalize_priority(priority_getter(item)),
            _normalize_last_used(last_used_getter(item)),
            _normalize_id(id_getter(item)),
        )
    )
    return candidates[0]
