import importlib.util
import asyncio
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


class HotmailRoutesTests(unittest.TestCase):
    def setUp(self):
        self.module = _load_hotmail_routes_module()
        app = FastAPI()
        app.include_router(self.module.router, prefix="/api/hotmail")
        self.client = TestClient(app)

    def test_create_hotmail_batch_returns_batch_metadata(self):
        self.module.create_hotmail_engine = lambda **_kwargs: types.SimpleNamespace(
            run=lambda: self.module.HotmailRegistrationResult(
                success=False,
                reason_code="phone_verification_required",
                error_message="phone required",
            )
        )
        response = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 2,
                "concurrency": 1,
                "interval_min": 1,
                "interval_max": 2,
                "execution_mode": "remote",
            },
        )

        self.assertEqual(response.status_code, 200)
        payload = response.json()
        self.assertIn("batch_id", payload)
        self.assertEqual(payload["total"], 2)

    def test_get_unknown_batch_returns_404(self):
        response = self.client.get("/api/hotmail/batches/missing")

        self.assertEqual(response.status_code, 404)

    def test_create_hotmail_engine_wires_default_verification_provider(self):
        captured = {}
        provider = object()

        def fake_provider_builder():
            return provider

        class FakeEngine:
            def __init__(self, **kwargs):
                captured.update(kwargs)

        self.module.build_default_hotmail_verification_provider = fake_provider_builder
        self.module.HotmailRegistrationEngine = FakeEngine

        engine = self.module.create_hotmail_engine(proxy_url="http://proxy.example:8080")

        self.assertIsInstance(engine, FakeEngine)
        self.assertEqual(captured["proxy_url"], "http://proxy.example:8080")
        self.assertIs(captured["verification_provider"], provider)

    def test_batch_completion_tracks_success_failed_and_artifacts(self):
        module = self.module

        class FakeEngine:
            def __init__(self):
                self._calls = 0

            def run(self):
                self._calls += 1
                if self._calls == 1:
                    return module.HotmailRegistrationResult(
                        success=True,
                        artifact=module.HotmailAccountArtifact(
                            email="ok@hotmail.com",
                            password="pw-1",
                            target_domain="hotmail.com",
                            verification_email="v@temp.example.com",
                        ),
                    )
                return module.HotmailRegistrationResult(
                    success=False,
                    reason_code="phone_verification_required",
                    error_message="phone required",
                )

        engine = FakeEngine()
        self.module.create_hotmail_engine = lambda **_kwargs: engine

        response = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 2,
                "concurrency": 1,
                "interval_min": 0,
                "interval_max": 0,
                "execution_mode": "remote",
            },
        )

        self.assertEqual(response.status_code, 200)
        batch_id = response.json()["batch_id"]
        batch = self.client.get(f"/api/hotmail/batches/{batch_id}").json()

        self.assertTrue(batch["finished"])
        self.assertEqual(batch["completed"], 2)
        self.assertEqual(batch["success"], 1)
        self.assertEqual(batch["failed"], 1)
        self.assertEqual(len(batch["artifacts"]), 2)

        artifacts = self.client.get(f"/api/hotmail/batches/{batch_id}/artifacts")
        self.assertEqual(artifacts.status_code, 200)
        self.assertEqual(len(artifacts.json()["artifacts"]), 2)

    def test_run_hotmail_batch_executes_engine_via_asyncio_to_thread(self):
        module = self.module
        observed = {"to_thread_calls": 0, "run_calls": 0}

        class FakeEngine:
            def run(self):
                observed["run_calls"] += 1
                return module.HotmailRegistrationResult(
                    success=False,
                    reason_code="unsupported_challenge",
                    error_message="challenge",
                )

        async def fake_to_thread(callback, *args, **kwargs):
            observed["to_thread_calls"] += 1
            return callback(*args, **kwargs)

        module.create_hotmail_engine = lambda **_kwargs: FakeEngine()
        module.asyncio.to_thread = fake_to_thread
        module.hotmail_batches["batch-thread"] = {
            "batch_id": "batch-thread",
            "total": 1,
            "completed": 0,
            "success": 0,
            "failed": 0,
            "finished": False,
            "cancelled": False,
            "logs": [],
            "artifacts": [],
        }

        asyncio.run(
            module._run_hotmail_batch(
                "batch-thread",
                module.HotmailBatchCreateRequest(count=1, concurrency=1, interval_min=0, interval_max=0),
            )
        )

        self.assertEqual(observed["to_thread_calls"], 1)
        self.assertEqual(observed["run_calls"], 1)

    def test_run_hotmail_batch_normalizes_unsupported_challenge_logs(self):
        module = self.module

        class FakeEngine:
            def run(self):
                return module.HotmailRegistrationResult(
                    success=False,
                    reason_code="unsupported_challenge",
                    error_message=(
                        "Hotmail signup failed: unsupported_challenge | "
                        "title=Let's prove you're human | "
                        "text=Press and hold the button."
                    ),
                    handoff_context={
                        "handoff_id": "handoff-challenge",
                        "url": "https://signup.live.com/challenge",
                        "title": "Let's prove you're human",
                        "instructions": "人工处理后继续。",
                    },
                )

        module.create_hotmail_engine = lambda **_kwargs: FakeEngine()
        module.hotmail_batches["batch-challenge"] = {
            "batch_id": "batch-challenge",
            "total": 1,
            "completed": 0,
            "success": 0,
            "failed": 0,
            "finished": False,
            "cancelled": False,
            "logs": [],
            "artifacts": [],
        }

        asyncio.run(
            module._run_hotmail_batch(
                "batch-challenge",
                module.HotmailBatchCreateRequest(count=1, concurrency=1, interval_min=0, interval_max=0),
            )
        )

        batch = module.hotmail_batches["batch-challenge"]
        self.assertEqual(batch["failed"], 0)
        self.assertEqual(batch["completed"], 0)
        self.assertEqual(len(batch["logs"]), 1)
        self.assertIn("微软要求人工验证", batch["logs"][0])
        self.assertIn("Press and hold the button", batch["logs"][0])
        self.assertEqual(batch["status"], "action_required")
        self.assertEqual(batch["action_required_reason"], "unsupported_challenge")
        self.assertFalse(batch["finished"])

    def test_create_batch_exposes_handoff_metadata_for_manual_challenge(self):
        module = self.module

        class FakeEngine:
            def run(self):
                return types.SimpleNamespace(
                    success=False,
                    artifact=None,
                    reason_code="unsupported_challenge",
                    error_message="Hotmail signup failed: unsupported_challenge",
                    handoff_context={
                        "handoff_id": "handoff-1",
                        "url": "https://signup.live.com/challenge",
                        "title": "Let's prove you're human",
                        "instructions": "请在运行注册服务的主机上完成人工验证，然后回来点击继续。",
                        "local_handoff": {
                            "handoff_id": "handoff-1",
                            "url": "https://signup.live.com/challenge",
                            "title": "Let's prove you're human",
                            "user_agent": "Mozilla/5.0 test",
                            "proxy_url": "http://127.0.0.1:7890",
                            "state": "unsupported_challenge",
                            "cookies": [{"name": "MSPRequ", "value": "cookie-value"}],
                            "origins": [],
                        },
                    },
                )

        module.create_hotmail_engine = lambda **_kwargs: FakeEngine()

        response = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 1,
                "concurrency": 1,
                "interval_min": 0,
                "interval_max": 0,
                "execution_mode": "remote",
            },
        )

        self.assertEqual(response.status_code, 200)
        payload = self.client.get(f"/api/hotmail/batches/{response.json()['batch_id']}").json()
        self.assertEqual(payload["status"], "action_required")
        self.assertEqual(payload["action_required_reason"], "unsupported_challenge")
        self.assertEqual(payload["handoff_id"], "handoff-1")
        self.assertEqual(payload["handoff_url"], "https://signup.live.com/challenge")
        self.assertEqual(payload["handoff_title"], "Let's prove you're human")
        self.assertTrue(payload["handoff_instructions"])
        self.assertEqual(payload["local_handoff"]["handoff_id"], "handoff-1")
        self.assertEqual(payload["local_handoff"]["state"], "unsupported_challenge")
        self.assertEqual(payload["failed"], 0)
        self.assertEqual(payload["completed"], 0)
        self.assertFalse(payload["finished"])

    def test_continue_endpoint_resumes_handoff_and_finishes_batch(self):
        module = self.module
        observed = {"resume_calls": 0}

        class FakeEngine:
            def run(self):
                return types.SimpleNamespace(
                    success=False,
                    artifact=None,
                    reason_code="unsupported_challenge",
                    error_message="Hotmail signup failed: unsupported_challenge",
                    handoff_context={
                        "handoff_id": "handoff-continue",
                        "url": "https://signup.live.com/challenge",
                        "title": "Let's prove you're human",
                        "instructions": "人工处理后继续。",
                    },
                )

            def resume_handoff(self, handoff_context):
                observed["resume_calls"] += 1
                assert handoff_context["handoff_id"] == "handoff-continue"
                return module.HotmailRegistrationResult(
                    success=True,
                    artifact=module.HotmailAccountArtifact(
                        email="ok@hotmail.com",
                        password="pw-1",
                        target_domain="hotmail.com",
                        verification_email="verify@temp.example.com",
                    ),
                )

        engine = FakeEngine()
        module.create_hotmail_engine = lambda **_kwargs: engine

        create_response = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 1,
                "concurrency": 1,
                "interval_min": 0,
                "interval_max": 0,
                "execution_mode": "remote",
            },
        )
        batch_id = create_response.json()["batch_id"]

        response = self.client.post(f"/api/hotmail/batches/{batch_id}/continue")

        self.assertEqual(response.status_code, 200)
        payload = response.json()
        self.assertEqual(observed["resume_calls"], 1)
        self.assertEqual(payload["success"], 1)
        self.assertEqual(payload["failed"], 0)
        self.assertEqual(payload["completed"], 1)
        self.assertTrue(payload["finished"])
        self.assertEqual(payload["handoff_id"], "")

    def test_abandon_endpoint_marks_pending_attempt_failed_and_finishes_batch(self):
        module = self.module
        observed = {"abandon_calls": 0}

        class FakeEngine:
            def run(self):
                return types.SimpleNamespace(
                    success=False,
                    artifact=None,
                    reason_code="unsupported_challenge",
                    error_message="Hotmail signup failed: unsupported_challenge",
                    handoff_context={
                        "handoff_id": "handoff-abandon",
                        "url": "https://signup.live.com/challenge",
                        "title": "Let's prove you're human",
                        "instructions": "人工处理后继续。",
                    },
                )

            def abandon_handoff(self, handoff_context):
                observed["abandon_calls"] += 1
                assert handoff_context["handoff_id"] == "handoff-abandon"

        engine = FakeEngine()
        module.create_hotmail_engine = lambda **_kwargs: engine

        create_response = self.client.post(
            "/api/hotmail/batches",
            json={
                "count": 1,
                "concurrency": 1,
                "interval_min": 0,
                "interval_max": 0,
                "execution_mode": "remote",
            },
        )
        batch_id = create_response.json()["batch_id"]

        response = self.client.post(f"/api/hotmail/batches/{batch_id}/abandon")

        self.assertEqual(response.status_code, 200)
        payload = response.json()
        self.assertEqual(observed["abandon_calls"], 1)
        self.assertEqual(payload["success"], 0)
        self.assertEqual(payload["failed"], 1)
        self.assertEqual(payload["completed"], 1)
        self.assertTrue(payload["finished"])
        self.assertEqual(payload["handoff_id"], "")


if __name__ == "__main__":
    unittest.main()
