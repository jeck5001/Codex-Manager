import importlib.util
import sys
import types
import unittest
from pathlib import Path

from fastapi import FastAPI
from fastapi.testclient import TestClient


def _load_hotmail_routes_module():
    src_dir = Path(__file__).resolve().parents[1] / "src"
    web_dir = src_dir / "web"
    routes_dir = web_dir / "routes"

    for module_name in list(sys.modules):
        if module_name == "src" or module_name.startswith("src."):
            sys.modules.pop(module_name, None)

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = [str(src_dir)]
    sys.modules["src"] = src_pkg

    web_pkg = types.ModuleType("src.web")
    web_pkg.__path__ = [str(web_dir)]
    sys.modules["src.web"] = web_pkg

    routes_pkg = types.ModuleType("src.web.routes")
    routes_pkg.__path__ = [str(routes_dir)]
    sys.modules["src.web.routes"] = routes_pkg

    spec = importlib.util.spec_from_file_location(
        "src.web.routes.hotmail",
        routes_dir / "hotmail.py",
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules["src.web.routes.hotmail"] = module
    spec.loader.exec_module(module)
    return module


class HotmailLocalFirstRouteTests(unittest.TestCase):
    def setUp(self):
        self.module = _load_hotmail_routes_module()
        app = FastAPI()
        app.include_router(self.module.router, prefix="/api/hotmail")
        self.client = TestClient(app)

        mailbox_service = types.SimpleNamespace(
            create_email=lambda: {
                "email": "verify@temp.example.com",
                "service_id": "mailbox-1",
            },
            get_verification_code=lambda **_kwargs: "654321",
        )
        mailbox = types.SimpleNamespace(
            name="temp-mail-1",
            service_type="temp_mail",
            service=mailbox_service,
        )
        self.module.build_default_hotmail_verification_provider = lambda: types.SimpleNamespace(
            acquire_mailbox=lambda: mailbox,
        )

    def test_create_batch_returns_local_first_task_payload(self):
        response = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 1,
                "concurrency": 1,
                "interval_min": 0,
                "interval_max": 0,
                "execution_mode": "local_first",
            },
        )

        self.assertEqual(response.status_code, 200)
        payload = response.json()
        self.assertEqual(payload["status"], "pending_local_start")
        self.assertEqual(payload["execution_mode"], "local_first")
        self.assertTrue(payload["current_task"]["task_id"])
        self.assertEqual(
            payload["current_task_payload"]["verification_mailbox"]["email"],
            "verify@temp.example.com",
        )

    def test_helper_progress_updates_batch_state(self):
        batch = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 1,
                "concurrency": 1,
                "interval_min": 0,
                "interval_max": 0,
                "execution_mode": "local_first",
            },
        ).json()
        task_id = batch["current_task"]["task_id"]

        response = self.client.post(
            f"/api/hotmail/batches/{batch['batch_id']}/tasks/{task_id}/progress",
            json={
                "status": "running",
                "current_step": "submitting_profile",
                "log_line": "profile submitted",
            },
        )

        self.assertEqual(response.status_code, 200)
        payload = self.client.get(f"/api/hotmail/batches/{batch['batch_id']}").json()
        self.assertEqual(payload["status"], "running")
        self.assertEqual(payload["current_task"]["current_step"], "submitting_profile")
        self.assertIn("profile submitted", payload["logs"][-1])

    def test_verification_code_endpoint_uses_reserved_mailbox(self):
        batch = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 1,
                "concurrency": 1,
                "interval_min": 0,
                "interval_max": 0,
                "execution_mode": "local_first",
            },
        ).json()
        task_id = batch["current_task"]["task_id"]

        response = self.client.post(
            f"/api/hotmail/batches/{batch['batch_id']}/tasks/{task_id}/verification-code",
            json={"timeout": 1, "poll_interval": 1},
        )

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.json()["code"], "654321")
