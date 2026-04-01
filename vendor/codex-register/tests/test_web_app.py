import asyncio
import importlib.util
import sys
import types
import unittest
from pathlib import Path


def load_web_app_module():
    module_name = "src.web.app"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "web"
        / "app.py"
    )

    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    web_pkg = types.ModuleType("src.web")
    web_pkg.__path__ = []
    routes_pkg = types.ModuleType("src.web.routes")
    routes_pkg.__path__ = []
    config_pkg = types.ModuleType("src.config")
    config_pkg.__path__ = []

    sys.modules["src"] = src_pkg
    sys.modules["src.web"] = web_pkg
    sys.modules["src.web.routes"] = routes_pkg
    sys.modules["src.config"] = config_pkg

    fastapi_module = types.ModuleType("fastapi")

    class FastAPI:
        def __init__(self, *args, **kwargs):
            self.routes = {}

        def add_middleware(self, *_args, **_kwargs):
            return None

        def mount(self, *_args, **_kwargs):
            return None

        def include_router(self, *_args, **_kwargs):
            return None

        def get(self, path, **_kwargs):
            def decorator(fn):
                self.routes[("GET", path)] = fn
                return fn

            return decorator

        def post(self, path, **_kwargs):
            def decorator(fn):
                self.routes[("POST", path)] = fn
                return fn

            return decorator

        def on_event(self, *_args, **_kwargs):
            return lambda fn: fn

    fastapi_module.FastAPI = FastAPI
    fastapi_module.Request = object
    fastapi_module.Form = lambda default=None, **_kwargs: default
    sys.modules["fastapi"] = fastapi_module

    staticfiles_module = types.ModuleType("fastapi.staticfiles")
    staticfiles_module.StaticFiles = type("StaticFiles", (), {"__init__": lambda self, *a, **k: None})
    sys.modules["fastapi.staticfiles"] = staticfiles_module

    templating_module = types.ModuleType("fastapi.templating")

    class Jinja2Templates:
        def __init__(self, directory):
            self.directory = directory

        def TemplateResponse(self, request, name, context=None, status_code=200):
            return {
                "request": request,
                "name": name,
                "context": context or {},
                "status_code": status_code,
            }

    templating_module.Jinja2Templates = Jinja2Templates
    sys.modules["fastapi.templating"] = templating_module

    cors_module = types.ModuleType("fastapi.middleware.cors")
    cors_module.CORSMiddleware = type("CORSMiddleware", (), {})
    sys.modules["fastapi.middleware.cors"] = cors_module

    responses_module = types.ModuleType("fastapi.responses")

    class HTMLResponse:
        pass

    class RedirectResponse:
        def __init__(self, url, status_code=302):
            self.url = url
            self.status_code = status_code
            self.cookies = {}
            self.deleted = []

        def set_cookie(self, key, value, **_kwargs):
            self.cookies[key] = value

        def delete_cookie(self, key):
            self.deleted.append(key)

    responses_module.HTMLResponse = HTMLResponse
    responses_module.RedirectResponse = RedirectResponse
    sys.modules["fastapi.responses"] = responses_module

    settings_module = types.ModuleType("src.config.settings")

    class SecretValue:
        def __init__(self, value):
            self.value = value

        def get_secret_value(self):
            return self.value

    settings_module.get_settings = lambda: types.SimpleNamespace(
        app_name="test-app",
        app_version="0.0.0",
        debug=False,
        database_url="sqlite:///test.db",
        webui_secret_key=SecretValue("secret"),
        webui_access_password=SecretValue("password"),
    )
    sys.modules["src.config.settings"] = settings_module

    routes_pkg.api_router = object()
    websocket_module = types.ModuleType("src.web.routes.websocket")
    websocket_module.router = object()
    sys.modules["src.web.routes.websocket"] = websocket_module

    task_manager_module = types.ModuleType("src.web.task_manager")
    task_manager_module.task_manager = types.SimpleNamespace(set_loop=lambda *_args, **_kwargs: None)
    sys.modules["src.web.task_manager"] = task_manager_module

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


WEB_APP_MODULE = load_web_app_module()


class WebAppTemplateResponseTests(unittest.TestCase):
    def test_login_page_passes_request_as_first_template_argument(self):
        app = WEB_APP_MODULE.create_app()
        login_page = app.routes[("GET", "/login")]
        request = types.SimpleNamespace(cookies={}, url=types.SimpleNamespace(path="/login"))

        response = asyncio.run(login_page(request, "/"))

        self.assertIs(response["request"], request)
        self.assertEqual(response["name"], "login.html")
        self.assertEqual(response["context"]["next"], "/")

    def test_login_submit_invalid_password_renders_login_template(self):
        app = WEB_APP_MODULE.create_app()
        login_submit = app.routes[("POST", "/login")]
        request = types.SimpleNamespace(cookies={}, url=types.SimpleNamespace(path="/login"))

        response = asyncio.run(login_submit(request, password="wrong", next="/"))

        self.assertIs(response["request"], request)
        self.assertEqual(response["name"], "login.html")
        self.assertEqual(response["status_code"], 401)
        self.assertEqual(response["context"]["error"], "密码错误")


if __name__ == "__main__":
    unittest.main()
