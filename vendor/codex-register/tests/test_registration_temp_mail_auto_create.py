import asyncio
import importlib.util
import sys
import types
import unittest
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

_MISSING = object()
_STUBBED_MODULE_NAMES = [
    "src",
    "src.web",
    "src.web.routes",
    "src.web.routes.registration",
    "src.web.routes.email_services",
    "src.database",
    "src.database.crud",
    "src.database.session",
    "src.database.models",
    "src.core",
    "src.core.register",
    "src.core.browserbase_ddg",
    "src.core.any_auto_register",
    "src.core.round_robin",
    "src.services",
    "src.config",
    "src.config.settings",
    "src.web.task_manager",
    "fastapi",
    "pydantic",
]


def load_registration_module():
    module_name = "src.web.routes.registration"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "web"
        / "routes"
        / "registration.py"
    )

    for name in list(sys.modules):
        if name == "src" or name.startswith("src."):
            sys.modules.pop(name, None)

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    web_pkg = types.ModuleType("src.web")
    web_pkg.__path__ = []
    routes_pkg = types.ModuleType("src.web.routes")
    routes_pkg.__path__ = []
    database_pkg = types.ModuleType("src.database")
    database_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    services_pkg = types.ModuleType("src.services")
    services_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []

    sys.modules["src"] = src_pkg
    sys.modules["src.web"] = web_pkg
    sys.modules["src.web.routes"] = routes_pkg
    sys.modules["src.database"] = database_pkg
    sys.modules["src.core"] = core_pkg
    sys.modules["src.services"] = services_pkg
    sys.modules["src.config"] = config_pkg

    fastapi_module = types.ModuleType("fastapi")

    class HTTPException(Exception):
        def __init__(self, status_code, detail):
            super().__init__(detail)
            self.status_code = status_code
            self.detail = detail

    class APIRouter:
        def get(self, *_args, **_kwargs):
            return lambda fn: fn

        def post(self, *_args, **_kwargs):
            return lambda fn: fn

        def delete(self, *_args, **_kwargs):
            return lambda fn: fn

        def patch(self, *_args, **_kwargs):
            return lambda fn: fn

    class BackgroundTasks:
        def __init__(self):
            self.calls: List[Tuple[tuple, dict]] = []

        def add_task(self, *args, **kwargs):
            self.calls.append((args, kwargs))

    fastapi_module.APIRouter = APIRouter
    fastapi_module.HTTPException = HTTPException
    fastapi_module.Query = lambda default=None, **_kwargs: default
    fastapi_module.BackgroundTasks = BackgroundTasks
    sys.modules["fastapi"] = fastapi_module

    pydantic_module = types.ModuleType("pydantic")

    class BaseModel:
        def __init__(self, **kwargs):
            for key, value in kwargs.items():
                setattr(self, key, value)

    pydantic_module.BaseModel = BaseModel
    pydantic_module.Field = lambda default=None, **_kwargs: default
    sys.modules["pydantic"] = pydantic_module

    # ---- stubs: email_services helpers (what we are integrating) ----
    email_services_module = types.ModuleType("src.web.routes.email_services")
    email_services_module.build_calls: List[dict] = []
    email_services_module.cleanup_calls: List[Optional[Dict[str, Any]]] = []
    email_services_module.record_calls: List[dict] = []

    def build_temp_mail_service_for_registration(config: Dict[str, Any], *, owner_task_uuid=None, owner_batch_id=None):
        email_services_module.build_calls.append(
            {"config": dict(config or {}), "owner_task_uuid": owner_task_uuid, "owner_batch_id": owner_batch_id}
        )
        cleanup_context = {"cleanup": True, "owner_task_uuid": owner_task_uuid, "owner_batch_id": owner_batch_id}
        merged = dict(config or {})
        merged["domain"] = "tm-fixed.mail.example.com"
        return merged, cleanup_context, "tm-fixed.mail.example.com"

    def _cleanup_temp_mail_provisioning(cleanup_context):
        email_services_module.cleanup_calls.append(cleanup_context)

    def record_temp_mail_domain_registration_outcome(
        domain_config_id: str,
        *,
        success: bool,
        failure_http_status=None,
        error_message: str = "",
    ):
        email_services_module.record_calls.append(
            {
                "domain_config_id": domain_config_id,
                "success": success,
                "failure_http_status": failure_http_status,
                "error_message": error_message,
            }
        )
        return None

    email_services_module.build_temp_mail_service_for_registration = build_temp_mail_service_for_registration
    email_services_module._cleanup_temp_mail_provisioning = _cleanup_temp_mail_provisioning
    email_services_module.record_temp_mail_domain_registration_outcome = record_temp_mail_domain_registration_outcome
    sys.modules["src.web.routes.email_services"] = email_services_module

    # ---- stubs: crud/db ----
    crud_module = types.ModuleType("src.database.crud")
    crud_module.created_email_services: List[dict] = []
    crud_module.deleted_email_services: List[int] = []
    crud_module.created_tasks: Dict[str, Any] = {}
    crud_module.updated_tasks: List[Tuple[str, dict]] = []
    crud_module.deleted_tasks: List[str] = []

    def create_email_service(_db, service_type: str, name: str, config: dict, enabled: bool = True, priority: int = 0):
        crud_module.created_email_services.append(
            {"service_type": service_type, "name": name, "config": dict(config or {}), "enabled": enabled, "priority": priority}
        )
        return types.SimpleNamespace(id=123, service_type=service_type, name=name, config=dict(config or {}), enabled=enabled)

    def delete_email_service(_db, service_id: int) -> bool:
        crud_module.deleted_email_services.append(service_id)
        return True

    def create_registration_task(
        _db,
        task_uuid: str,
        email_service_id: Optional[int] = None,
        proxy: Optional[str] = None,
        register_mode: str = "standard",
        browserbase_config_id: Optional[int] = None,
    ):
        task = types.SimpleNamespace(
            id=len(crud_module.created_tasks) + 1,
            task_uuid=task_uuid,
            status="pending",
            register_mode=register_mode,
            email_service_id=email_service_id,
            email_service=None,
            browserbase_config_id=browserbase_config_id,
            proxy=proxy,
            logs=None,
            result=None,
            error_message=None,
            created_at=None,
            started_at=None,
            completed_at=None,
        )
        crud_module.created_tasks[task_uuid] = task
        return task

    def get_registration_task(_db, task_uuid: str):
        return crud_module.created_tasks.get(task_uuid)

    def update_registration_task(_db, task_uuid: str, **kwargs):
        crud_module.updated_tasks.append((task_uuid, dict(kwargs)))
        task = crud_module.created_tasks.get(task_uuid)
        if task:
            for key, value in kwargs.items():
                setattr(task, key, value)
        return task

    def delete_registration_task(_db, task_uuid: str) -> bool:
        crud_module.deleted_tasks.append(task_uuid)
        crud_module.created_tasks.pop(task_uuid, None)
        return True

    crud_module.create_email_service = create_email_service
    crud_module.delete_email_service = delete_email_service
    crud_module.create_registration_task = create_registration_task
    crud_module.get_registration_task = get_registration_task
    crud_module.update_registration_task = update_registration_task
    crud_module.delete_registration_task = delete_registration_task
    crud_module.get_enabled_proxies = lambda *_args, **_kwargs: []
    crud_module.update_proxy_last_used = lambda *_args, **_kwargs: None
    crud_module.update_email_service_last_used = lambda *_args, **_kwargs: None
    sys.modules["src.database.crud"] = crud_module

    session_module = types.ModuleType("src.database.session")

    class DummyDbContext:
        def __init__(self):
            self.db = object()

        def __enter__(self):
            return self.db

        def __exit__(self, exc_type, exc, tb):
            return False

    session_module.get_db = lambda: DummyDbContext()
    sys.modules["src.database.session"] = session_module

    models_module = types.ModuleType("src.database.models")
    models_module.RegistrationTask = type("RegistrationTask", (), {})
    models_module.Proxy = type("Proxy", (), {})
    models_module.EmailService = type("EmailService", (), {})
    models_module.Account = type("Account", (), {})
    models_module.BrowserbaseConfig = type("BrowserbaseConfig", (), {})
    sys.modules["src.database.models"] = models_module

    register_module = types.ModuleType("src.core.register")
    register_module.RegistrationEngine = type("RegistrationEngine", (), {})
    register_module.RegistrationResult = type("RegistrationResult", (), {})
    sys.modules["src.core.register"] = register_module

    browserbase_module = types.ModuleType("src.core.browserbase_ddg")
    browserbase_module.BROWSERBASE_DDG_REGISTER_MODE = "browserbase_ddg"
    browserbase_module.DEFAULT_REGISTER_MODE = "standard"
    browserbase_module.BrowserbaseDDGRegistrationRunner = type("BrowserbaseDDGRegistrationRunner", (), {})
    sys.modules["src.core.browserbase_ddg"] = browserbase_module

    any_auto_module = types.ModuleType("src.core.any_auto_register")
    any_auto_module.ANY_AUTO_REGISTER_MODE = "any_auto"
    any_auto_module.AnyAutoRegistrationRunner = type("AnyAutoRegistrationRunner", (), {})
    sys.modules["src.core.any_auto_register"] = any_auto_module

    round_robin_module = types.ModuleType("src.core.round_robin")
    round_robin_module.build_round_robin_schedule = lambda items, count: []
    round_robin_module.pick_round_robin_item = lambda items: items[0] if items else None
    sys.modules["src.core.round_robin"] = round_robin_module

    services_module = types.ModuleType("src.services")
    from enum import Enum

    class EmailServiceFactory:
        @staticmethod
        def create(*_args, **_kwargs):
            return object()

    class EmailServiceType(str, Enum):
        TEMPMAIL = "tempmail"
        OUTLOOK = "outlook"
        CUSTOM_DOMAIN = "custom_domain"
        TEMP_MAIL = "temp_mail"

    services_module.EmailServiceFactory = EmailServiceFactory
    services_module.EmailServiceType = EmailServiceType
    sys.modules["src.services"] = services_module

    settings_module = types.ModuleType("src.config.settings")
    settings_module.get_settings = lambda: types.SimpleNamespace(proxy_url=None)
    sys.modules["src.config.settings"] = settings_module

    task_manager_module = types.ModuleType("src.web.task_manager")
    task_manager_module.task_manager = types.SimpleNamespace(
        create_log_callback=lambda *_args, **_kwargs: (lambda *_a, **_k: None),
        update_status=lambda *_args, **_kwargs: None,
        get_loop=lambda: None,
        set_loop=lambda *_args, **_kwargs: None,
        add_log=lambda *_args, **_kwargs: None,
        init_batch=lambda *_args, **_kwargs: None,
        add_batch_log=lambda *_args, **_kwargs: None,
        update_batch_status=lambda *_args, **_kwargs: None,
        is_batch_cancelled=lambda *_args, **_kwargs: False,
        is_cancelled=lambda *_args, **_kwargs: False,
        executor=None,
        cancel_task=lambda *_args, **_kwargs: None,
        cancel_batch=lambda *_args, **_kwargs: None,
    )
    sys.modules["src.web.task_manager"] = task_manager_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


REGISTRATION_MODULE = None


class RegistrationTempMailAutoCreateTests(unittest.TestCase):
    def setUp(self):
        global REGISTRATION_MODULE
        self._saved_modules = {
            name: sys.modules.get(name, _MISSING)
            for name in _STUBBED_MODULE_NAMES
        }
        REGISTRATION_MODULE = load_registration_module()
        # reset stub call history between tests
        email_services = sys.modules["src.web.routes.email_services"]
        email_services.build_calls[:] = []
        email_services.cleanup_calls[:] = []
        email_services.record_calls[:] = []
        crud = sys.modules["src.database.crud"]
        crud.created_email_services[:] = []
        crud.deleted_email_services[:] = []
        crud.created_tasks.clear()
        crud.updated_tasks[:] = []
        crud.deleted_tasks[:] = []

    def tearDown(self):
        for name, original in self._saved_modules.items():
            if original is _MISSING:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = original

    def test_single_register_flag_creates_and_binds_temp_mail_service(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.RegistrationTaskCreate(
            email_service_type="temp_mail",
            email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
            auto_create_temp_mail_service=True,
        )

        response = asyncio.run(REGISTRATION_MODULE.start_registration(request, background))

        crud = sys.modules["src.database.crud"]
        email_services = sys.modules["src.web.routes.email_services"]

        self.assertEqual(len(email_services.build_calls), 1)
        created_task_uuid = next(iter(crud.created_tasks))
        self.assertEqual(email_services.build_calls[0]["owner_task_uuid"], created_task_uuid)

        self.assertEqual(len(crud.created_email_services), 1)
        self.assertEqual(crud.created_email_services[0]["service_type"], "temp_mail")
        self.assertEqual(crud.created_email_services[0]["name"], "tm-fixed.mail.example.com")

        self.assertEqual(crud.created_tasks[created_task_uuid].email_service_id, 123)
        self.assertEqual(response.email_service_id, 123)

        self.assertEqual(len(background.calls), 1)
        args, kwargs = background.calls[0]
        self.assertIs(args[0], REGISTRATION_MODULE.run_registration_task)
        self.assertEqual(args[1], created_task_uuid)
        self.assertEqual(args[5], 123)  # email_service_id passed to background task
        self.assertEqual(kwargs, {})

    def test_batch_register_flag_creates_one_service_and_reuses_across_tasks(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.BatchRegistrationRequest(
            count=3,
            email_service_type="temp_mail",
            email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
            auto_create_temp_mail_service=True,
            interval_min=0,
            interval_max=0,
            concurrency=1,
            mode="parallel",
        )

        response = asyncio.run(REGISTRATION_MODULE.start_batch_registration(request, background))

        crud = sys.modules["src.database.crud"]
        email_services = sys.modules["src.web.routes.email_services"]

        self.assertEqual(len(email_services.build_calls), 1)
        self.assertEqual(email_services.build_calls[0]["owner_batch_id"], response.batch_id)
        self.assertEqual(len(crud.created_email_services), 1)
        self.assertEqual(len(crud.created_tasks), 3)
        self.assertTrue(all(task.email_service_id == 123 for task in crud.created_tasks.values()))

        self.assertEqual(len(background.calls), 1)
        args, _kwargs = background.calls[0]
        self.assertIs(args[0], REGISTRATION_MODULE.run_batch_registration)
        task_email_service_ids = args[10]
        self.assertEqual(task_email_service_ids, [123, 123, 123])

    def test_cleanup_path_deletes_service_and_runs_remote_cleanup(self):
        task_uuid = "task-1"
        # seed cleanup metadata as if created during registration setup
        REGISTRATION_MODULE._AUTO_CREATED_TEMP_MAIL_TASKS = {
            task_uuid: {"service_id": 123, "cleanup_context": {"cleanup": True}, "task_uuid": task_uuid}
        }

        REGISTRATION_MODULE._cleanup_auto_created_temp_mail_task(task_uuid)

        crud = sys.modules["src.database.crud"]
        email_services = sys.modules["src.web.routes.email_services"]

        self.assertEqual(crud.deleted_email_services, [123])
        self.assertEqual(email_services.cleanup_calls, [{"cleanup": True}])

    def test_flag_off_preserves_old_behavior(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.RegistrationTaskCreate(
            email_service_type="temp_mail",
            email_service_id=55,
            auto_create_temp_mail_service=False,
        )

        response = asyncio.run(REGISTRATION_MODULE.start_registration(request, background))

        crud = sys.modules["src.database.crud"]
        email_services = sys.modules["src.web.routes.email_services"]

        self.assertEqual(len(email_services.build_calls), 0)
        self.assertEqual(len(crud.created_email_services), 0)
        created_task_uuid = next(iter(crud.created_tasks))
        self.assertEqual(crud.created_tasks[created_task_uuid].email_service_id, 55)
        self.assertEqual(response.email_service_id, 55)

    def test_setup_failure_after_service_creation_triggers_rollback_cleanup(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        crud = sys.modules["src.database.crud"]
        email_services = sys.modules["src.web.routes.email_services"]

        original_create_task = crud.create_registration_task

        def boom(*_args, **_kwargs):
            raise RuntimeError("db down")

        crud.create_registration_task = boom
        try:
            request = REGISTRATION_MODULE.RegistrationTaskCreate(
                email_service_type="temp_mail",
                email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
                auto_create_temp_mail_service=True,
            )
            with self.assertRaises(RuntimeError):
                asyncio.run(REGISTRATION_MODULE.start_registration(request, background))
        finally:
            crud.create_registration_task = original_create_task

        # service was created then rolled back
        self.assertEqual(len(crud.created_email_services), 1)
        self.assertEqual(crud.deleted_email_services, [123])
        self.assertEqual(len(email_services.cleanup_calls), 1)
        self.assertTrue(email_services.cleanup_calls[0].get("cleanup"))

    def test_cancel_single_task_defers_cleanup_to_finalizer(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.RegistrationTaskCreate(
            email_service_type="temp_mail",
            email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
            auto_create_temp_mail_service=True,
        )

        response = asyncio.run(REGISTRATION_MODULE.start_registration(request, background))
        task_uuid = response.task_uuid

        asyncio.run(REGISTRATION_MODULE.cancel_task(task_uuid))

        crud = sys.modules["src.database.crud"]
        email_services = sys.modules["src.web.routes.email_services"]

        self.assertEqual(crud.deleted_email_services, [])
        self.assertEqual(email_services.cleanup_calls, [])
        self.assertIn(task_uuid, getattr(REGISTRATION_MODULE, "_AUTO_CREATED_TEMP_MAIL_TASKS", {}))

        REGISTRATION_MODULE._cleanup_auto_created_temp_mail_task(task_uuid)

        self.assertEqual(crud.deleted_email_services, [123])
        self.assertEqual(len(email_services.cleanup_calls), 1)
        self.assertNotIn(task_uuid, getattr(REGISTRATION_MODULE, "_AUTO_CREATED_TEMP_MAIL_TASKS", {}))

    def test_cancel_batch_defers_cleanup_to_finalizer(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.BatchRegistrationRequest(
            count=2,
            email_service_type="temp_mail",
            email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
            auto_create_temp_mail_service=True,
            interval_min=0,
            interval_max=0,
            concurrency=1,
            mode="parallel",
        )

        response = asyncio.run(REGISTRATION_MODULE.start_batch_registration(request, background))

        # endpoint requires batch state to exist; emulate started background batch bookkeeping
        crud = sys.modules["src.database.crud"]
        batch_task_uuids = list(crud.created_tasks.keys())
        REGISTRATION_MODULE.batch_tasks[response.batch_id] = {
            "finished": False,
            "cancelled": False,
            "task_uuids": batch_task_uuids,
        }

        asyncio.run(REGISTRATION_MODULE.cancel_batch(response.batch_id))

        email_services = sys.modules["src.web.routes.email_services"]

        self.assertEqual(crud.deleted_email_services, [])
        self.assertEqual(email_services.cleanup_calls, [])
        self.assertIn(response.batch_id, getattr(REGISTRATION_MODULE, "_AUTO_CREATED_TEMP_MAIL_BATCHES", {}))

        REGISTRATION_MODULE._cleanup_auto_created_temp_mail_batch(response.batch_id)

        self.assertEqual(crud.deleted_email_services, [123])
        self.assertEqual(len(email_services.cleanup_calls), 1)
        self.assertNotIn(response.batch_id, getattr(REGISTRATION_MODULE, "_AUTO_CREATED_TEMP_MAIL_BATCHES", {}))
        self.assertTrue(all(task.email_service_id is None for task in crud.created_tasks.values()))

    def test_cancel_running_task_defers_cleanup(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.RegistrationTaskCreate(
            email_service_type="temp_mail",
            email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
            auto_create_temp_mail_service=True,
        )

        response = asyncio.run(REGISTRATION_MODULE.start_registration(request, background))
        task_uuid = response.task_uuid

        crud = sys.modules["src.database.crud"]
        crud.created_tasks[task_uuid].status = "running"

        asyncio.run(REGISTRATION_MODULE.cancel_task(task_uuid))

        email_services = sys.modules["src.web.routes.email_services"]
        self.assertEqual(crud.deleted_email_services, [])
        self.assertEqual(email_services.cleanup_calls, [])
        self.assertIn(task_uuid, getattr(REGISTRATION_MODULE, "_AUTO_CREATED_TEMP_MAIL_TASKS", {}))

    def test_cancel_batch_with_running_task_defers_cleanup(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.BatchRegistrationRequest(
            count=2,
            email_service_type="temp_mail",
            email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
            auto_create_temp_mail_service=True,
            interval_min=0,
            interval_max=0,
            concurrency=1,
            mode="parallel",
        )

        response = asyncio.run(REGISTRATION_MODULE.start_batch_registration(request, background))
        crud = sys.modules["src.database.crud"]
        task_uuids = list(crud.created_tasks.keys())
        crud.created_tasks[task_uuids[0]].status = "running"
        REGISTRATION_MODULE.batch_tasks[response.batch_id] = {
            "finished": False,
            "cancelled": False,
            "task_uuids": task_uuids,
        }

        asyncio.run(REGISTRATION_MODULE.cancel_batch(response.batch_id))

        email_services = sys.modules["src.web.routes.email_services"]
        self.assertEqual(crud.deleted_email_services, [])
        self.assertEqual(email_services.cleanup_calls, [])
        self.assertIn(response.batch_id, getattr(REGISTRATION_MODULE, "_AUTO_CREATED_TEMP_MAIL_BATCHES", {}))

        # Later batch finalization should still be able to clean up (idempotent).
        REGISTRATION_MODULE._cleanup_auto_created_temp_mail_batch(response.batch_id)
        self.assertEqual(crud.deleted_email_services, [123])
        self.assertEqual(len(email_services.cleanup_calls), 1)

    def test_retry_failed_auto_created_temp_mail_reprovisions_fresh_service(self):
        background = sys.modules["fastapi"].BackgroundTasks()
        request = REGISTRATION_MODULE.RegistrationTaskCreate(
            email_service_type="temp_mail",
            email_service_config={"base_url": "https://worker.example.com", "admin_password": "secret"},
            auto_create_temp_mail_service=True,
        )

        response = asyncio.run(REGISTRATION_MODULE.start_registration(request, background))
        task_uuid = response.task_uuid

        crud = sys.modules["src.database.crud"]
        email_services = sys.modules["src.web.routes.email_services"]

        # Simulate task finished as failed and its temp-mail resource cleaned up.
        crud.created_tasks[task_uuid].status = "failed"
        REGISTRATION_MODULE._cleanup_auto_created_temp_mail_task(task_uuid)

        retry_bg = sys.modules["fastapi"].BackgroundTasks()
        retry_response = asyncio.run(REGISTRATION_MODULE.retry_task(task_uuid, retry_bg, None))

        self.assertEqual(len(crud.created_email_services), 2)
        self.assertEqual(len(email_services.build_calls), 2)
        self.assertNotEqual(retry_response.task_uuid, task_uuid)
        self.assertEqual(retry_response.email_service_id, 123)

    def test_record_temp_mail_domain_result_marks_password_400_as_domain_penalty(self):
        result = types.SimpleNamespace(
            success=False,
            error_message="注册密码失败",
            logs=[
                "[10:13:03] 密码注册会话摘要: count=14 cf_clearance=no",
                "[10:13:04] 提交密码状态: 400",
                "[10:13:04] 密码注册失败: {\"error\":{\"message\":\"Failed to create account.\"}}",
            ],
        )

        REGISTRATION_MODULE._record_temp_mail_domain_result("cfg-1", result)

        email_services = sys.modules["src.web.routes.email_services"]
        self.assertEqual(
            email_services.record_calls,
            [
                {
                    "domain_config_id": "cfg-1",
                    "success": False,
                    "failure_http_status": 400,
                    "error_message": "注册密码失败",
                }
            ],
        )


if __name__ == "__main__":
    unittest.main()
