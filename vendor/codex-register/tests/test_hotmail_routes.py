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


class HotmailRoutesTests(unittest.TestCase):
    def setUp(self):
        self.module = _load_hotmail_routes_module()
        app = FastAPI()
        app.include_router(self.module.router, prefix="/api/hotmail")
        self.client = TestClient(app)

    def test_create_hotmail_batch_returns_batch_metadata(self):
        response = self.client.post(
            "/api/hotmail/batches",
            json={"count": 2, "concurrency": 1, "interval_min": 1, "interval_max": 2},
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
            json={"count": 2, "concurrency": 1, "interval_min": 0, "interval_max": 0},
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


if __name__ == "__main__":
    unittest.main()
