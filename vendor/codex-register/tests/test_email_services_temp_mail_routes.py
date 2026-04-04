import importlib.util
import sys
import types
import unittest
from datetime import datetime
from pathlib import Path
from typing import Any, Callable, List, Optional

from fastapi import FastAPI
from fastapi.testclient import TestClient
from pydantic import SecretStr


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
        self.rollbacks = 0
        self.refreshed: List[Any] = []
        self.on_add: Optional[Callable[[], None]] = None
        self.on_commit: Optional[Callable[[], None]] = None

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
        if self.on_commit:
            self.on_commit()
        self.commits += 1

    def rollback(self):
        self.rollbacks += 1

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


class _DummyResponse:
    def __init__(self, status_code: int, payload: Any):
        self.status_code = status_code
        self._payload = payload

    def json(self):
        return self._payload


class _RecordingHttpClient:
    def __init__(self, responses: List[_DummyResponse]):
        self._responses = list(responses)
        self.calls: List[dict] = []

    def request(self, method: str, url: str, **kwargs):
        self.calls.append({"method": method, "url": url, "kwargs": kwargs})
        if not self._responses:
            raise AssertionError("No more stub responses configured")
        return self._responses.pop(0)


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

    def test_temp_mail_update_rejects_domain_when_only_spacing_case_changes(self):
        service = self.module.EmailServiceModel(
            service_type="temp_mail",
            name="Temp Mail D",
            config={
                "base_url": "https://worker.example.com",
                "admin_password": "secret",
                "domain": "tm-old.mail.example.com",
            },
            enabled=True,
            priority=0,
        )
        service.id = 89
        service.created_at = datetime.utcnow()
        service.updated_at = datetime.utcnow()

        fake_db = _FakeDB(first_results=[service])
        self.module.get_db = lambda: _DBContext(fake_db)

        response = self.client.patch(
            "/api/email-services/89",
            json={"config": {"domain": "  TM-OLD.MAIL.EXAMPLE.COM  "}},
        )

        self.assertEqual(response.status_code, 400)
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

    def test_temp_mail_create_rolls_back_remote_state_when_db_commit_fails(self):
        cleanup_calls: List[dict] = []
        fake_db = _FakeDB(first_results=[None])
        fake_db.on_commit = lambda: (_ for _ in ()).throw(RuntimeError("db commit failed"))
        self.module.get_db = lambda: _DBContext(fake_db)

        class FakeProvisioner:
            def __init__(self, *_args, **_kwargs):
                pass

            def provision_domain(self):
                return {"domain": "tm-fixed.mail.example.com", "cloudflare_subdomain": {"id": "sub-123"}}

            def cleanup_provisioned_domain(self, provisioned, domain=None):
                cleanup_calls.append({"provisioned": provisioned, "domain": domain})

        self.module.CloudflareTempMailProvisioner = FakeProvisioner
        self.module.get_settings = lambda: object()

        response = self.client.post(
            "/api/email-services",
            json={
                "service_type": "temp_mail",
                "name": "Temp Mail E",
                "config": {
                    "base_url": "https://worker.example.com",
                    "admin_password": "secret",
                },
                "enabled": True,
                "priority": 0,
            },
        )

        self.assertEqual(response.status_code, 500)
        self.assertEqual(len(cleanup_calls), 1)
        self.assertEqual(cleanup_calls[0]["domain"], "tm-fixed.mail.example.com")
        self.assertEqual(fake_db.rollbacks, 1)

    def test_provisioner_patch_failure_triggers_subdomain_cleanup(self):
        from src.config.settings import Settings

        settings = Settings(
            cloudflare_api_token=SecretStr("token"),
            cloudflare_account_id="account",
            cloudflare_zone_id="zone",
            cloudflare_worker_name="worker",
            temp_mail_domain_base="mail.example.com",
            temp_mail_subdomain_mode="sequence",
            temp_mail_subdomain_length=6,
            temp_mail_subdomain_prefix="tm",
        )
        http_client = _RecordingHttpClient(
            responses=[
                _DummyResponse(200, {"result": {"id": "sub-1", "name": "tm-abc123.mail.example.com"}}),
                _DummyResponse(200, {"result": {"settings": {"bindings": []}}}),
                _DummyResponse(500, {"errors": ["patch failed"]}),
                _DummyResponse(200, {"result": {"id": "sub-1"}}),
            ]
        )
        provisioner = self.module.CloudflareTempMailProvisioner(settings, http_client=http_client)

        with self.assertRaises(Exception):
            provisioner.provision_domain()

        methods = [entry["method"] for entry in http_client.calls]
        self.assertEqual(methods, ["POST", "GET", "PATCH", "DELETE"])
