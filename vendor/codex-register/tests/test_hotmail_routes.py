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


if __name__ == "__main__":
    unittest.main()
