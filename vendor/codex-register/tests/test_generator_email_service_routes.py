import asyncio
import importlib.util
import sys
import types
import unittest
from pathlib import Path


def load_email_services_module():
    module_name = "src.web.routes.email_services"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "web"
        / "routes"
        / "email_services.py"
    )

    for module_name_item in list(sys.modules):
        if (
            module_name_item == "src"
            or module_name_item.startswith("src.")
            or module_name_item.startswith("curl_cffi")
            or module_name_item == "fastapi"
            or module_name_item.startswith("fastapi.")
            or module_name_item == "pydantic"
            or module_name_item.startswith("pydantic.")
        ):
            sys.modules.pop(module_name_item, None)

    src_dir = Path(__file__).resolve().parents[1] / "src"
    web_dir = src_dir / "web"
    routes_dir = web_dir / "routes"
    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = [str(src_dir)]
    web_pkg = types.ModuleType("src.web")
    web_pkg.__path__ = [str(web_dir)]
    routes_pkg = types.ModuleType("src.web.routes")
    routes_pkg.__path__ = [str(routes_dir)]
    sys.modules["src"] = src_pkg
    sys.modules["src.web"] = web_pkg
    sys.modules["src.web.routes"] = routes_pkg

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


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
    crud_module.get_enabled_proxies = lambda *_args, **_kwargs: []
    crud_module.update_proxy_last_used = lambda *_args, **_kwargs: None
    crud_module.update_email_service_last_used = lambda *_args, **_kwargs: None
    sys.modules["src.database.crud"] = crud_module

    class _PriorityField:
        @staticmethod
        def asc():
            return None

    class _EnabledField:
        def __eq__(self, _other):
            return None

    class _ServiceTypeField:
        def __eq__(self, _other):
            return None

    models_module = types.ModuleType("src.database.models")
    models_module.RegistrationTask = type("RegistrationTask", (), {})
    models_module.Proxy = type("Proxy", (), {})
    models_module.Account = type("Account", (), {})
    models_module.BrowserbaseConfig = type("BrowserbaseConfig", (), {})
    models_module.EmailService = type(
        "EmailService",
        (),
        {
            "service_type": _ServiceTypeField(),
            "enabled": _EnabledField(),
            "priority": _PriorityField(),
            "id": None,
        },
    )
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
    round_robin_module.build_round_robin_schedule = lambda items, count: list(items)[:count]
    round_robin_module.pick_round_robin_item = lambda items: items[0] if items else None
    sys.modules["src.core.round_robin"] = round_robin_module

    services_module = types.ModuleType("src.services")

    class EmailServiceType:
        TEMPMAIL = types.SimpleNamespace(value="tempmail")
        OUTLOOK = types.SimpleNamespace(value="outlook")
        CUSTOM_DOMAIN = types.SimpleNamespace(value="custom_domain")
        TEMP_MAIL = types.SimpleNamespace(value="temp_mail")
        MAIL_33_IMAP = types.SimpleNamespace(value="mail_33_imap")
        GENERATOR_EMAIL = types.SimpleNamespace(value="generator_email")

        def __init__(self, value):
            self.value = value

    services_module.EmailServiceFactory = type("EmailServiceFactory", (), {"create": staticmethod(lambda *_args, **_kwargs: None)})
    services_module.EmailServiceType = EmailServiceType
    sys.modules["src.services"] = services_module

    settings_module = types.ModuleType("src.config.settings")
    settings_module.get_settings = lambda: types.SimpleNamespace(
        custom_domain_base_url="",
        custom_domain_api_key="",
    )
    sys.modules["src.config.settings"] = settings_module

    session_module = types.ModuleType("src.database.session")

    class _Query:
        def __init__(self, service_type):
            self._service_type = service_type

        def filter(self, *args, **kwargs):
            return self

        def order_by(self, *args, **kwargs):
            return self

        def all(self):
            if self._service_type == "generator_email":
                return [
                    types.SimpleNamespace(
                        id=7,
                        name="Generator Email Pool",
                        priority=2,
                        config={},
                    )
                ]
            return []

    class _DB:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return False

        def query(self, model):
            return _Query(service_type="generator_email")

    session_module.get_db = lambda: _DB()
    sys.modules["src.database.session"] = session_module

    task_manager_module = types.ModuleType("src.web.task_manager")
    task_manager_module.task_manager = types.SimpleNamespace()
    sys.modules["src.web.task_manager"] = task_manager_module

    email_services_helper_module = types.ModuleType("src.web.routes.email_services")
    email_services_helper_module._load_temp_mail_domain_configs = lambda _settings: []
    sys.modules["src.web.routes.email_services"] = email_services_helper_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


class GeneratorEmailRouteExposureTests(unittest.TestCase):
    def test_email_service_types_expose_generator_email(self):
        module = load_email_services_module()

        payload = asyncio.run(module.get_service_types())
        values = {item["value"] for item in payload["types"]}

        self.assertIn("generator_email", values)

    def test_registration_available_services_include_generator_email_group(self):
        module = load_registration_module()

        payload = asyncio.run(module.get_available_email_services())

        self.assertIn("generator_email", payload)
        self.assertTrue(payload["generator_email"]["available"])
        self.assertEqual(payload["generator_email"]["count"], 1)
        self.assertEqual(payload["generator_email"]["services"][0]["type"], "generator_email")

    def test_email_service_stats_include_generator_email_count(self):
        module = load_email_services_module()

        class _StatsQuery:
            def __init__(self, values):
                self._values = values

            def group_by(self, *_args, **_kwargs):
                return self

            def filter(self, *_args, **_kwargs):
                return self

            def all(self):
                return list(self._values)

            def scalar(self):
                return 4

        class _StatsDB:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def query(self, *args):
                if len(args) == 2:
                    return _StatsQuery([
                        ("outlook", 1),
                        ("generator_email", 3),
                    ])
                return _StatsQuery([])

        module.get_db = lambda: _StatsDB()

        payload = asyncio.run(module.get_email_services_stats())

        self.assertEqual(payload["generator_email_count"], 3)
        self.assertEqual(payload["enabled_count"], 4)


if __name__ == "__main__":
    unittest.main()
