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
        def __init__(self):
            self.calls = []

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

    crud_module = types.ModuleType("src.database.crud")
    crud_module.create_registration_task = lambda *_args, **_kwargs: None
    crud_module.get_enabled_proxies = lambda *_args, **_kwargs: []
    crud_module.update_proxy_last_used = lambda *_args, **_kwargs: None
    crud_module.update_email_service_last_used = lambda *_args, **_kwargs: None
    crud_module.get_registration_tasks_by_statuses = lambda *_args, **_kwargs: []
    crud_module.append_task_log = lambda *_args, **_kwargs: None
    crud_module.update_registration_task = lambda *_args, **_kwargs: None
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
    models_module.BrowserbaseConfig = type("BrowserbaseConfig", (), {})
    sys.modules["src.database.models"] = models_module

    register_module = types.ModuleType("src.core.register")
    register_module.RegistrationEngine = type("RegistrationEngine", (), {})
    register_module.RegistrationResult = type("RegistrationResult", (), {})
    sys.modules["src.core.register"] = register_module

    browserbase_module = types.ModuleType("src.core.browserbase_ddg")
    browserbase_module.BROWSERBASE_DDG_REGISTER_MODE = "browserbase_ddg"
    browserbase_module.DEFAULT_REGISTER_MODE = "standard"
    browserbase_module.BrowserbaseDDGRegistrationRunner = type(
        "BrowserbaseDDGRegistrationRunner",
        (),
        {},
    )
    sys.modules["src.core.browserbase_ddg"] = browserbase_module

    any_auto_module = types.ModuleType("src.core.any_auto_register")
    any_auto_module.ANY_AUTO_REGISTER_MODE = "any_auto"
    any_auto_module.AnyAutoRegistrationRunner = type(
        "AnyAutoRegistrationRunner",
        (),
        {},
    )
    sys.modules["src.core.any_auto_register"] = any_auto_module

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

        def __init__(self, value):
            self.value = value

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


class RegisterAnyAutoModeTests(unittest.TestCase):
    def test_normalize_register_mode_accepts_any_auto(self):
        self.assertEqual(REGISTRATION_MODULE._normalize_register_mode("any_auto"), "any_auto")
        self.assertEqual(REGISTRATION_MODULE._normalize_register_mode("ANY_AUTO"), "any_auto")

    def test_create_single_task_persists_any_auto_mode(self):
        background_tasks = REGISTRATION_MODULE.BackgroundTasks()
        created_calls = []

        class Task:
            def __init__(self):
                self.id = 11
                self.task_uuid = "task-any-auto-1"
                self.status = "pending"
                self.email_service_id = None
                self.browserbase_config_id = None
                self.register_mode = "any_auto"
                self.proxy = None
                self.logs = ""
                self.result = None
                self.error_message = None
                self.created_at = None
                self.started_at = None
                self.completed_at = None

        REGISTRATION_MODULE.crud.create_registration_task = (
            lambda _db, **kwargs: created_calls.append(kwargs) or Task()
        )

        task = REGISTRATION_MODULE._create_single_registration_task(
            db=object(),
            background_tasks=background_tasks,
            email_service_type="tempmail",
            proxy=None,
            email_service_config=None,
            email_service_id=None,
            register_mode="any_auto",
            browserbase_config_id=None,
        )

        self.assertEqual(task.register_mode, "any_auto")
        self.assertEqual(created_calls[0]["register_mode"], "any_auto")
        self.assertEqual(background_tasks.calls[0][0][6], "any_auto")


if __name__ == "__main__":
    unittest.main()
