import importlib.util
import sys
import types
import unittest
from pathlib import Path


def load_registration_module():
    module_name = "src.web.routes.registration"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "web"
        / "routes"
        / "registration.py"
    )

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

    class BackgroundTasks:
        def add_task(self, *_args, **_kwargs):
            return None

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

    crud_module = types.ModuleType("src.database.crud")
    crud_module.get_registration_tasks_by_statuses = lambda *_args, **_kwargs: []
    crud_module.update_registration_task = lambda *_args, **_kwargs: None
    crud_module.append_task_log = lambda *_args, **_kwargs: True
    crud_module.get_enabled_proxies = lambda *_args, **_kwargs: []
    crud_module.update_proxy_last_used = lambda *_args, **_kwargs: None
    crud_module.update_email_service_last_used = lambda *_args, **_kwargs: None
    sys.modules["src.database.crud"] = crud_module

    session_module = types.ModuleType("src.database.session")

    class DummyDbContext:
        def __enter__(self):
            return object()

        def __exit__(self, exc_type, exc, tb):
            return False

    session_module.get_db = lambda: DummyDbContext()
    sys.modules["src.database.session"] = session_module

    models_module = types.ModuleType("src.database.models")
    models_module.RegistrationTask = type("RegistrationTask", (), {})
    models_module.Proxy = type("Proxy", (), {})
    models_module.EmailService = type("EmailService", (), {})
    models_module.Account = type("Account", (), {})
    sys.modules["src.database.models"] = models_module

    register_module = types.ModuleType("src.core.register")
    register_module.RegistrationEngine = type("RegistrationEngine", (), {})
    register_module.RegistrationResult = type("RegistrationResult", (), {})
    sys.modules["src.core.register"] = register_module

    round_robin_module = types.ModuleType("src.core.round_robin")
    round_robin_module.build_round_robin_schedule = lambda items, count: []
    round_robin_module.pick_round_robin_item = lambda items: items[0] if items else None
    sys.modules["src.core.round_robin"] = round_robin_module

    services_module = types.ModuleType("src.services")

    class EmailServiceFactory:
        @staticmethod
        def create(*_args, **_kwargs):
            return object()

    class EmailServiceType:
        OUTLOOK = types.SimpleNamespace(value="outlook")
        TEMPMAIL = types.SimpleNamespace(value="tempmail")
        TEMP_MAIL = types.SimpleNamespace(value="temp_mail")
        CUSTOM_DOMAIN = types.SimpleNamespace(value="custom_domain")

    services_module.EmailServiceFactory = EmailServiceFactory
    services_module.EmailServiceType = EmailServiceType
    sys.modules["src.services"] = services_module

    settings_module = types.ModuleType("src.config.settings")
    settings_module.get_settings = lambda: types.SimpleNamespace(proxy_url=None)
    sys.modules["src.config.settings"] = settings_module

    task_manager_module = types.ModuleType("src.web.task_manager")
    task_manager_module.task_manager = types.SimpleNamespace(
        update_status=lambda *_args, **_kwargs: None,
        add_log=lambda *_args, **_kwargs: None,
        create_log_callback=lambda *_args, **_kwargs: (lambda *_a, **_k: None),
        get_loop=lambda: None,
        set_loop=lambda *_args, **_kwargs: None,
        init_batch=lambda *_args, **_kwargs: None,
        add_batch_log=lambda *_args, **_kwargs: None,
        update_batch_status=lambda *_args, **_kwargs: None,
        is_batch_cancelled=lambda *_args, **_kwargs: False,
        is_cancelled=lambda *_args, **_kwargs: False,
        executor=None,
    )
    sys.modules["src.web.task_manager"] = task_manager_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


REGISTRATION_MODULE = load_registration_module()


class RegistrationRecoveryTests(unittest.TestCase):
    def test_recover_interrupted_tasks_requeues_pending_and_fails_running(self):
        class Task:
            def __init__(self, task_uuid, status, service_type="temp_mail", proxy=None, email_service_id=None):
                self.task_uuid = task_uuid
                self.status = status
                self.proxy = proxy
                self.email_service_id = email_service_id
                self.logs = ""
                self.email_service = types.SimpleNamespace(service_type=service_type)

        pending_task = Task("pending-1", "pending", proxy="http://proxy")
        running_task = Task("running-1", "running")
        scheduled = []
        updates = []
        status_updates = []
        appended_logs = []

        REGISTRATION_MODULE.crud.get_registration_tasks_by_statuses = (
            lambda _db, statuses: [pending_task, running_task]
        )
        REGISTRATION_MODULE.crud.update_registration_task = (
            lambda _db, task_uuid, **kwargs: updates.append((task_uuid, kwargs))
        )
        REGISTRATION_MODULE.crud.append_task_log = (
            lambda _db, task_uuid, message: appended_logs.append((task_uuid, message)) or True
        )
        REGISTRATION_MODULE.task_manager.update_status = (
            lambda task_uuid, status, **kwargs: status_updates.append((task_uuid, status, kwargs))
        )

        summary = REGISTRATION_MODULE._recover_interrupted_registration_tasks(
            lambda task_uuid, email_service_type, proxy, email_service_id: scheduled.append(
                (task_uuid, email_service_type, proxy, email_service_id)
            )
        )

        self.assertEqual(summary["resumed_pending"], 1)
        self.assertEqual(summary["failed_running"], 1)
        self.assertEqual(
            scheduled,
            [("pending-1", "temp_mail", "http://proxy", None)],
        )
        self.assertTrue(
            any(task_uuid == "running-1" and kwargs.get("status") == "failed" for task_uuid, kwargs in updates)
        )
        self.assertTrue(
            any(task_uuid == "pending-1" and "自动恢复排队" in message for task_uuid, message in appended_logs)
        )
        self.assertTrue(
            any(task_uuid == "running-1" and status == "failed" for task_uuid, status, _kwargs in status_updates)
        )

    def test_resume_recovered_registration_tasks_limits_concurrency(self):
        task_specs = [
            ("task-1", "temp_mail", None, None),
            ("task-2", "temp_mail", None, None),
            ("task-3", "temp_mail", None, None),
            ("task-4", "temp_mail", None, None),
        ]
        active = 0
        max_active = 0

        async def runner(task_uuid, email_service_type, proxy, email_service_id):
            nonlocal active, max_active
            self.assertEqual(email_service_type, "temp_mail")
            active += 1
            max_active = max(max_active, active)
            await __import__("asyncio").sleep(0.01)
            active -= 1

        __import__("asyncio").run(
            REGISTRATION_MODULE.resume_recovered_registration_tasks(
                task_specs,
                runner,
                concurrency=2,
            )
        )

        self.assertEqual(max_active, 2)


if __name__ == "__main__":
    unittest.main()
