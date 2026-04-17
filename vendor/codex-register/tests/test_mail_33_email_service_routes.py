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
        if module_name_item == "src" or module_name_item.startswith("src."):
            sys.modules.pop(module_name_item, None)
    for module_name_item in ("fastapi", "pydantic"):
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

    session_module = types.ModuleType("src.database.session")
    session_module.get_db = lambda: None
    sys.modules["src.database.session"] = session_module

    models_module = types.ModuleType("src.database.models")
    models_module.RegistrationTask = type("RegistrationTask", (), {})
    models_module.Proxy = type("Proxy", (), {})
    models_module.Account = type("Account", (), {})
    models_module.BrowserbaseConfig = type("BrowserbaseConfig", (), {})
    models_module.EmailService = type(
        "EmailService",
        (),
        {
            "service_type": None,
            "enabled": None,
            "priority": types.SimpleNamespace(asc=lambda: None),
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


class Mail33RouteExposureTests(unittest.TestCase):
    def test_email_service_types_expose_mail33_imap(self):
        module = load_email_services_module()

        payload = asyncio.run(module.get_service_types())
        values = {item["value"] for item in payload["types"]}
        mail33 = next(item for item in payload["types"] if item["value"] == "mail_33_imap")
        subject_keyword_field = next(
            field for field in mail33["config_fields"] if field["name"] == "subject_keyword"
        )

        self.assertIn("mail_33_imap", values)
        self.assertEqual(subject_keyword_field["default"], "Your ChatGPT code is")

    def test_registration_available_services_include_mail33_group(self):
        module = load_registration_module()

        class FakeService:
            def __init__(self, service_id, name, service_type, priority):
                self.id = service_id
                self.name = name
                self.service_type = service_type
                self.priority = priority
                self.enabled = True
                self.config = {"alias_domain": "demo.33mail.com"}

        services = [FakeService(9, "33mail Main", "mail_33_imap", 1)]

        class FakeQuery:
            def __init__(self, values):
                self.values = values

            def filter(self, *_args, **_kwargs):
                return self

            def order_by(self, *_args, **_kwargs):
                return self

            def all(self):
                return self.values

        class FakeDb:
            def query(self, _model):
                return FakeQuery(services)

        class DbContext:
            def __enter__(self):
                return FakeDb()

            def __exit__(self, exc_type, exc, tb):
                return False

        module.get_db = lambda: DbContext()
        module.get_settings = lambda: types.SimpleNamespace(
            custom_domain_base_url="",
            custom_domain_api_key="",
        )

        payload = asyncio.run(module.get_available_email_services())

        self.assertTrue(payload["mail_33_imap"]["available"])
        self.assertEqual(payload["mail_33_imap"]["count"], 1)
        self.assertEqual(payload["mail_33_imap"]["services"][0]["id"], 9)

    def test_update_mail33_service_allows_clearing_optional_filters(self):
        module = load_email_services_module()

        class FakeService:
            def __init__(self):
                self.id = 7
                self.service_type = "mail_33_imap"
                self.name = "33mail Main"
                self.enabled = True
                self.priority = 1
                self.config = {
                    "alias_domain": "295542345.33mail.com",
                    "real_inbox_email": "295542345@qq.com",
                    "imap_host": "imap.qq.com",
                    "imap_username": "295542345@qq.com",
                    "imap_password": "secret",
                    "from_filter": "openai.com",
                    "subject_keyword": "OpenAI",
                }
                self.last_used = None
                self.created_at = None
                self.updated_at = None

        service = FakeService()

        class FakeQuery:
            def filter(self, *_args, **_kwargs):
                return self

            def first(self):
                return service

        class FakeDb:
            def query(self, _model):
                return FakeQuery()

            def commit(self):
                return None

            def refresh(self, _service):
                return None

        class DbContext:
            def __enter__(self):
                return FakeDb()

            def __exit__(self, exc_type, exc, tb):
                return False

        module.get_db = lambda: DbContext()

        response = asyncio.run(
            module.update_email_service(
                7,
                types.SimpleNamespace(
                    name=None,
                    config={"from_filter": "", "subject_keyword": ""},
                    enabled=None,
                    priority=None,
                ),
            )
        )

        self.assertEqual(service.config["from_filter"], "")
        self.assertEqual(service.config["subject_keyword"], "")
        self.assertEqual(response.config["from_filter"], "")
        self.assertEqual(response.config["subject_keyword"], "")


if __name__ == "__main__":
    unittest.main()
