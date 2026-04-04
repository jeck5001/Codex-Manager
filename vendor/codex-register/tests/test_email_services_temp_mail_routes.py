import importlib.util
import sys
import types
import unittest
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, List, Optional

from fastapi import FastAPI
from fastapi.testclient import TestClient


def _load_email_services_module():
    src_dir = Path(__file__).resolve().parents[1] / "src"
    if "src" not in sys.modules:
        src_pkg = types.ModuleType("src")
        src_pkg.__path__ = [str(src_dir)]
        sys.modules["src"] = src_pkg

    web_dir = src_dir / "web"
    if "src.web" not in sys.modules:
        web_pkg = types.ModuleType("src.web")
        web_pkg.__path__ = [str(web_dir)]
        sys.modules["src.web"] = web_pkg

    routes_dir = Path(__file__).resolve().parents[1] / "src" / "web" / "routes"
    pkg_name = "src.web.routes"
    if pkg_name not in sys.modules:
        routes_pkg = types.ModuleType(pkg_name)
        routes_pkg.__path__ = [str(routes_dir)]
        sys.modules[pkg_name] = routes_pkg

    spec = importlib.util.spec_from_file_location(
        "src.web.routes.email_services",
        routes_dir / "email_services.py",
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules["src.web.routes.email_services"] = module
    spec.loader.exec_module(module)
    return module


class _FakeQuery:
    def __init__(self, db: "_FakeDB"):
        self._db = db

    def filter(self, *_args, **_kwargs):
        return self

    def first(self):
        return self._db.pop_first_result()


class _FakeDB:
    def __init__(self, first_results: Optional[List[Any]] = None):
        self._first_results = list(first_results or [])
        self.added: List[Any] = []
        self.commits = 0
        self.refreshed: List[Any] = []
        self.on_add: Optional[Callable[[], None]] = None

    def pop_first_result(self):
        if self._first_results:
            return self._first_results.pop(0)
        return None

    def query(self, _model):
        return _FakeQuery(self)

    def add(self, obj):
        if self.on_add:
            self.on_add()
        self.added.append(obj)

    def commit(self):
        self.commits += 1

    def refresh(self, obj):
        if getattr(obj, "id", None) is None:
            obj.id = len(self.added)
        now = datetime.utcnow()
        if getattr(obj, "created_at", None) is None:
            obj.created_at = now
        obj.updated_at = now
        self.refreshed.append(obj)


class _DBContext:
    def __init__(self, db: _FakeDB):
        self._db = db

    def __enter__(self):
        return self._db

    def __exit__(self, exc_type, exc_val, exc_tb):
        return False


class EmailServicesTempMailRoutesTests(unittest.TestCase):
    def setUp(self):
        self.module = _load_email_services_module()
        app = FastAPI()
        app.include_router(self.module.router, prefix="/api/email-services")
        self.client = TestClient(app)

    def test_temp_mail_create_provisions_before_insert_and_persists_generated_domain(self):
        events: List[str] = []
        fake_db = _FakeDB(first_results=[None])
        fake_db.on_add = lambda: events.append("add")
        self.module.get_db = lambda: _DBContext(fake_db)

        class FakeProvisioner:
            def __init__(self, *_args, **_kwargs):
                pass

            def provision_domain(self):
                events.append("provision")
                return {
                    "domain": "tm-fixed.mail.example.com",
                    "cloudflare_subdomain_id": "subdomain-123",
                }

        self.module.CloudflareTempMailProvisioner = FakeProvisioner
        self.module.get_settings = lambda: object()

        response = self.client.post(
            "/api/email-services",
            json={
                "service_type": "temp_mail",
                "name": "Temp Mail A",
                "config": {
                    "base_url": "https://worker.example.com",
                    "admin_password": "secret",
                    "domain": "manual.invalid.example",
                },
                "enabled": True,
                "priority": 0,
            },
        )

        self.assertEqual(response.status_code, 200)
        self.assertEqual(len(fake_db.added), 1)
        self.assertEqual(fake_db.added[0].config["domain"], "tm-fixed.mail.example.com")
        self.assertEqual(fake_db.added[0].config["cloudflare_subdomain_id"], "subdomain-123")
        self.assertEqual(events, ["provision", "add"])

    def test_temp_mail_create_provisioning_failure_returns_http_error_and_skips_insert(self):
        fake_db = _FakeDB(first_results=[None])
        self.module.get_db = lambda: _DBContext(fake_db)

        class FailingProvisioner:
            def __init__(self, *_args, **_kwargs):
                pass

            def provision_domain(self):
                raise RuntimeError("cloudflare provisioning failed")

        self.module.CloudflareTempMailProvisioner = FailingProvisioner
        self.module.get_settings = lambda: object()

        response = self.client.post(
            "/api/email-services",
            json={
                "service_type": "temp_mail",
                "name": "Temp Mail B",
                "config": {
                    "base_url": "https://worker.example.com",
                    "admin_password": "secret",
                },
                "enabled": True,
                "priority": 0,
            },
        )

        self.assertEqual(response.status_code, 502)
        self.assertEqual(len(fake_db.added), 0)
        self.assertEqual(fake_db.commits, 0)

    def test_temp_mail_update_rejects_domain_override(self):
        service = self.module.EmailServiceModel(
            service_type="temp_mail",
            name="Temp Mail C",
            config={
                "base_url": "https://worker.example.com",
                "admin_password": "secret",
                "domain": "tm-old.mail.example.com",
            },
            enabled=True,
            priority=0,
        )
        service.id = 88
        service.created_at = datetime.utcnow()
        service.updated_at = datetime.utcnow()

        fake_db = _FakeDB(first_results=[service])
        self.module.get_db = lambda: _DBContext(fake_db)

        response = self.client.patch(
            "/api/email-services/88",
            json={"config": {"domain": "tm-new.mail.example.com"}},
        )

        self.assertEqual(response.status_code, 400)
        self.assertIn("domain", response.json()["detail"])
        self.assertEqual(service.config["domain"], "tm-old.mail.example.com")
        self.assertEqual(fake_db.commits, 0)

    def test_non_temp_mail_create_path_is_unchanged(self):
        fake_db = _FakeDB(first_results=[None])
        self.module.get_db = lambda: _DBContext(fake_db)

        class GuardProvisioner:
            def __init__(self, *_args, **_kwargs):
                raise AssertionError("Temp mail provisioner should not be used for outlook")

        self.module.CloudflareTempMailProvisioner = GuardProvisioner
        self.module.get_settings = lambda: object()

        payload = {
            "service_type": "outlook",
            "name": "Outlook A",
            "config": {"email": "a@example.com", "password": "x"},
            "enabled": True,
            "priority": 1,
        }
        response = self.client.post("/api/email-services", json=payload)

        self.assertEqual(response.status_code, 200)
        self.assertEqual(len(fake_db.added), 1)
        self.assertEqual(fake_db.added[0].config, payload["config"])
