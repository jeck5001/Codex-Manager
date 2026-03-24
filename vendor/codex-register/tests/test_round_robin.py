from datetime import datetime, timedelta, timezone
import importlib.util
from pathlib import Path
from types import SimpleNamespace
import unittest

MODULE_PATH = Path(__file__).resolve().parents[1] / "src" / "core" / "round_robin.py"
MODULE_SPEC = importlib.util.spec_from_file_location("round_robin_helper", MODULE_PATH)
assert MODULE_SPEC and MODULE_SPEC.loader
ROUND_ROBIN_MODULE = importlib.util.module_from_spec(MODULE_SPEC)
MODULE_SPEC.loader.exec_module(ROUND_ROBIN_MODULE)
build_round_robin_schedule = ROUND_ROBIN_MODULE.build_round_robin_schedule
pick_round_robin_item = ROUND_ROBIN_MODULE.pick_round_robin_item


class RoundRobinHelperTests(unittest.TestCase):
    def test_prefers_never_used_item_with_same_priority(self):
        now = datetime.now(timezone.utc)
        items = [
            SimpleNamespace(id=2, priority=0, last_used=now - timedelta(minutes=5)),
            SimpleNamespace(id=1, priority=0, last_used=None),
        ]

        picked = pick_round_robin_item(items)

        self.assertIsNotNone(picked)
        self.assertEqual(picked.id, 1)

    def test_prefers_oldest_last_used_item_with_same_priority(self):
        now = datetime.now(timezone.utc)
        items = [
            SimpleNamespace(id=1, priority=0, last_used=now - timedelta(minutes=1)),
            SimpleNamespace(id=2, priority=0, last_used=now - timedelta(minutes=10)),
        ]

        picked = pick_round_robin_item(items)

        self.assertIsNotNone(picked)
        self.assertEqual(picked.id, 2)

    def test_priority_beats_last_used(self):
        now = datetime.now(timezone.utc)
        items = [
            SimpleNamespace(id=1, priority=1, last_used=None),
            SimpleNamespace(id=2, priority=0, last_used=now),
        ]

        picked = pick_round_robin_item(items)

        self.assertIsNotNone(picked)
        self.assertEqual(picked.id, 2)

    def test_schedule_cycles_through_sorted_items(self):
        now = datetime.now(timezone.utc)
        items = [
            SimpleNamespace(id=3, priority=0, last_used=now - timedelta(minutes=1)),
            SimpleNamespace(id=1, priority=0, last_used=None),
            SimpleNamespace(id=2, priority=0, last_used=now - timedelta(minutes=10)),
        ]

        schedule = build_round_robin_schedule(items, 5)

        self.assertEqual([item.id for item in schedule], [1, 2, 3, 1, 2])

    def test_schedule_returns_empty_for_empty_pool(self):
        schedule = build_round_robin_schedule([], 3)

        self.assertEqual(schedule, [])


if __name__ == "__main__":
    unittest.main()
