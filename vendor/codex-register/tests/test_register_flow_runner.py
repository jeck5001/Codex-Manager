import importlib.util
import sys
import types
import unittest
from pathlib import Path


BASE_DIR = Path(__file__).resolve().parents[1] / "src" / "core"


def load_module(module_name: str, file_name: str):
    spec = importlib.util.spec_from_file_location(module_name, BASE_DIR / file_name)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def load_flow_runner_module():
    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = []
    core_pkg = types.ModuleType("src.core")
    core_pkg.__path__ = []
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg

    load_module("src.core.register_flow_state", "register_flow_state.py")
    load_module("src.core.register_token_resolver", "register_token_resolver.py")
    return load_module("src.core.register_flow_runner", "register_flow_runner.py")


FLOW_RUNNER_MODULE = load_flow_runner_module()


class EngineStub:
    def __init__(self):
        self.calls = []
        self._last_login_recovery_page_type = ""
        self._post_create_continue_url = ""
        self._post_create_page_type = ""
        self._is_existing_account = False

    def _build_callback_url_from_page(self, page):
        self.calls.append(("build_callback", page))
        return "http://localhost:1455/auth/callback?code=page&state=state"

    def _get_workspace_id(self):
        self.calls.append(("get_workspace_id",))
        return "ws_123"

    def _select_workspace(self, workspace_id):
        self.calls.append(("select_workspace", workspace_id))
        return "https://auth.openai.com/workspace/continue"

    def _follow_redirects(self, url):
        self.calls.append(("follow_redirects", url))
        return "http://localhost:1455/auth/callback?code=redir&state=state"

    def _advance_workspace_authorization(self, url):
        self.calls.append(("advance_workspace", url))
        return "http://localhost:1455/auth/callback?code=phone&state=state"

    def _build_authenticated_oauth_url(self):
        return (
            "https://auth.openai.com/oauth/authorize"
            "?client_id=test&response_type=code&state=state"
        )

    def _log(self, *_args, **_kwargs):
        return None


class RegisterFlowRunnerTests(unittest.TestCase):
    def test_runner_returns_auth_resolution_result_shape(self):
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=EngineStub())
        result = runner.resolve_auth_result({"page": {"type": "add_phone"}})
        self.assertEqual(result.page_type, "add_phone")
        self.assertEqual(result.continue_url, "")
        self.assertIsNone(result.callback_url)

    def test_resolve_callback_from_auth_page_uses_token_exchange_builder(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_auth_page(
            {"type": "token_exchange", "continue_url": "https://auth.openai.com/token"},
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=page&state=state",
        )
        self.assertEqual(engine.calls[0][0], "build_callback")

    def test_resolve_callback_from_auth_page_selects_workspace(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_auth_page(
            {"type": "workspace"},
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(
            engine.calls,
            [
                ("get_workspace_id",),
                ("select_workspace", "ws_123"),
                ("follow_redirects", "https://auth.openai.com/workspace/continue"),
            ],
        )

    def test_resolve_callback_from_auth_page_follows_external_url(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_auth_page(
            {"type": "external_url", "payload": {"url": "https://chatgpt.com/auth/callback"}},
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(
            engine.calls,
            [("follow_redirects", "https://chatgpt.com/auth/callback")],
        )

    def test_resolve_callback_from_continue_url_advances_add_phone(self):
        engine = EngineStub()
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)
        callback = runner.resolve_callback_from_continue_url(
            "https://auth.openai.com/add-phone",
            "测试阶段",
        )
        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=phone&state=state",
        )
        self.assertEqual(
            engine.calls,
            [("advance_workspace", "https://auth.openai.com/add-phone")],
        )

    def test_advance_workspace_authorization_uses_consent_response(self):
        class FakeResponse:
            url = "https://auth.openai.com/sign-in-with-chatgpt/codex/consent"
            text = '<script>window.__NEXT_DATA__={"activeWorkspaceId":"ws-consent"}</script>'
            history = []

            def json(self):
                raise ValueError("not json")

        class FakeSession:
            def get(self, url, **_kwargs):
                return FakeResponse()

        engine = EngineStub()
        engine.session = FakeSession()
        engine._cached_workspace_id = ""
        engine._extract_workspace_id_from_response = lambda **_kwargs: "ws-consent"
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        callback = runner.advance_workspace_authorization("https://auth.openai.com/add-phone")

        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(engine._cached_workspace_id, "ws-consent")
        self.assertEqual(
            engine.calls,
            [
                ("select_workspace", "ws-consent"),
                ("follow_redirects", "https://auth.openai.com/workspace/continue"),
            ],
        )

    def test_advance_workspace_authorization_follows_discovered_consent_link(self):
        class AddPhoneResponse:
            url = "https://auth.openai.com/add-phone"
            text = '<a href="/sign-in-with-chatgpt/codex/consent?step=1">continue</a>'
            history = []

            def json(self):
                raise ValueError("not json")

        class ConsentResponse:
            url = "https://auth.openai.com/sign-in-with-chatgpt/codex/consent?step=1"
            text = '<script>window.__NEXT_DATA__={"activeWorkspaceId":"ws-linked"}</script>'
            history = []

            def json(self):
                raise ValueError("not json")

        class FakeSession:
            def __init__(self):
                self.urls = []

            def get(self, url, **_kwargs):
                self.urls.append(url)
                if "sign-in-with-chatgpt/codex/consent" in url:
                    return ConsentResponse()
                return AddPhoneResponse()

        engine = EngineStub()
        engine.session = FakeSession()
        engine._cached_workspace_id = ""
        engine._extract_workspace_id_from_response = lambda **_kwargs: (
            "ws-linked" if _kwargs.get("url", "").endswith("step=1") else None
        )
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        callback = runner.advance_workspace_authorization("https://auth.openai.com/add-phone")

        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(engine._cached_workspace_id, "ws-linked")
        self.assertEqual(
            engine.session.urls,
            [
                "https://auth.openai.com/add-phone",
                "https://auth.openai.com/sign-in-with-chatgpt/codex/consent?step=1",
            ],
        )

    def test_attempt_add_phone_login_bypass_tries_workspace_authorization_before_oauth_retry(self):
        engine = EngineStub()
        engine.email = "user@example.com"
        engine.password = "secret"
        engine.oauth_start = types.SimpleNamespace(
            auth_url="https://auth.openai.com/oauth/authorize?client_id=test",
        )
        engine._submit_login_identifier = lambda did, sen: {"type": "login_password"}
        engine._verify_login_password = lambda password: {"type": "email_otp_verification"}
        engine._complete_login_email_otp_verification = lambda: types.SimpleNamespace(
            callback_url=None,
            page_type="add_phone",
            continue_url="https://auth.openai.com/add-phone",
        )
        engine._restart_oauth_session_for_login = lambda: (_ for _ in ()).throw(AssertionError("should not restart"))
        advanced_targets = []
        followed_urls = []
        engine._advance_workspace_authorization = lambda auth_target: (
            advanced_targets.append(auth_target)
            or "http://localhost:1455/auth/callback?code=phone&state=state"
        )
        engine._follow_redirects = lambda url: followed_urls.append(url) or None
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        callback = runner.attempt_add_phone_login_bypass("did", "sentinel")

        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=phone&state=state",
        )
        self.assertEqual(advanced_targets, ["https://auth.openai.com/add-phone"])
        self.assertEqual(followed_urls, [])

    def test_attempt_add_phone_login_bypass_reuses_authenticated_oauth_url_without_prompt_login(self):
        engine = EngineStub()
        engine.email = "user@example.com"
        engine.password = "secret"
        engine.oauth_start = types.SimpleNamespace(
            auth_url=(
                "https://auth.openai.com/oauth/authorize"
                "?client_id=test&response_type=code&prompt=login&state=state"
            ),
        )
        engine._submit_login_identifier = lambda did, sen: {"type": "login_password"}
        engine._verify_login_password = lambda password: {"type": "email_otp_verification"}
        engine._complete_login_email_otp_verification = lambda: types.SimpleNamespace(
            callback_url=None,
            page_type="add_phone",
            continue_url="https://auth.openai.com/add-phone",
        )
        engine._restart_oauth_session_for_login = lambda: (_ for _ in ()).throw(AssertionError("should not restart"))
        engine._advance_workspace_authorization = lambda auth_target: None
        followed_urls = []
        engine._follow_redirects = lambda url: followed_urls.append(url) or "http://localhost:1455/auth/callback?code=redir&state=state"
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        callback = runner.attempt_add_phone_login_bypass("did", "sentinel")

        self.assertEqual(
            callback,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(
            followed_urls,
            [
                "https://auth.openai.com/oauth/authorize"
                "?client_id=test&response_type=code&state=state"
            ],
        )

    def test_complete_login_email_otp_verification_continues_oauth_after_add_phone_resolution(self):
        engine = EngineStub()
        engine._otp_sent_at = None
        engine.oauth_start = types.SimpleNamespace(auth_url="https://auth.openai.com/oauth/authorize?client_id=test")
        engine._get_verification_code = lambda: "123456"
        engine._validate_verification_code_with_payload = lambda code: {"page": {"type": "add_phone"}}
        engine._resend_email_verification_code = lambda: (_ for _ in ()).throw(AssertionError("should not resend"))
        engine._is_wrong_email_otp_code_error = lambda: False
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        result = runner.complete_login_email_otp_verification()

        self.assertEqual(result.page_type, "add_phone")
        self.assertEqual(result.continue_url, "")
        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(engine._last_login_recovery_page_type, "add_phone")

    def test_complete_login_email_otp_verification_falls_back_to_oauth_retry(self):
        engine = EngineStub()
        engine._otp_sent_at = None
        engine.oauth_start = types.SimpleNamespace(auth_url="https://auth.openai.com/oauth/authorize?client_id=test")
        engine._get_verification_code = lambda: "123456"
        engine._validate_verification_code_with_payload = lambda code: {"page": {"type": ""}}
        engine._resend_email_verification_code = lambda: (_ for _ in ()).throw(AssertionError("should not resend"))
        engine._is_wrong_email_otp_code_error = lambda: False
        followed_urls = []
        engine._follow_redirects = lambda url: followed_urls.append(url) or "http://localhost:1455/auth/callback?code=oauth&state=state"
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        result = runner.complete_login_email_otp_verification()

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=oauth&state=state",
        )
        self.assertEqual(followed_urls, [engine._build_authenticated_oauth_url()])
        self.assertEqual(engine._last_login_recovery_page_type, "")

    def test_complete_login_email_otp_verification_only_resends_once(self):
        engine = EngineStub()
        engine._otp_sent_at = None
        engine.oauth_start = types.SimpleNamespace(auth_url="https://auth.openai.com/oauth/authorize?client_id=test")
        get_code_calls = []
        resend_calls = []
        engine._get_verification_code = lambda: get_code_calls.append(True) or None
        engine._validate_verification_code_with_payload = lambda code: None
        engine._resend_email_verification_code = lambda: resend_calls.append(True) or True
        engine._is_wrong_email_otp_code_error = lambda: False
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        result = runner.complete_login_email_otp_verification()

        self.assertEqual(result.callback_url, None)
        self.assertEqual(result.page_type, "")
        self.assertEqual(result.continue_url, "")
        self.assertEqual(len(resend_calls), 1)
        self.assertEqual(len(get_code_calls), 2)

    def test_resolve_post_registration_callback_ignores_add_phone_bypass_and_prefers_workspace_flow(self):
        engine = EngineStub()
        engine._post_create_page_type = "add_phone"
        engine._post_create_continue_url = "https://auth.openai.com/add-phone"
        engine._attempt_add_phone_login_bypass = (
            lambda did, sen: (_ for _ in ()).throw(AssertionError("should not use bypass"))
        )
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        result = runner.resolve_post_registration_callback("did", "sentinel")

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=redir&state=state",
        )
        self.assertEqual(result.workspace_id, "ws_123")
        self.assertEqual(result.metadata, {})
        self.assertEqual(
            engine.calls,
            [
                ("get_workspace_id",),
                ("select_workspace", "ws_123"),
                ("follow_redirects", "https://auth.openai.com/workspace/continue"),
            ],
        )

    def test_resolve_post_registration_callback_falls_back_to_authenticated_oauth_url(self):
        engine = EngineStub()
        engine._get_workspace_id = lambda: None
        followed_urls = []
        engine._follow_redirects = lambda url: followed_urls.append(url) or "http://localhost:1455/auth/callback?code=oauth&state=state"
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        result = runner.resolve_post_registration_callback("did", "sentinel")

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=oauth&state=state",
        )
        self.assertEqual(result.workspace_id, "")
        self.assertEqual(followed_urls, [engine._build_authenticated_oauth_url()])

    def test_resolve_post_registration_callback_ignores_prior_add_phone_bypass_attempt_and_uses_oauth_fallback(self):
        engine = EngineStub()
        engine._post_create_page_type = "add_phone"
        engine._post_create_continue_url = "https://auth.openai.com/add-phone"
        engine._add_phone_login_bypass_attempted = True
        engine._attempt_add_phone_login_bypass = (
            lambda did, sen: (_ for _ in ()).throw(AssertionError("should not retry bypass"))
        )
        engine._get_workspace_id = lambda: None
        followed_urls = []
        engine._follow_redirects = lambda url: followed_urls.append(url) or "http://localhost:1455/auth/callback?code=oauth&state=state"
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        result = runner.resolve_post_registration_callback("did", "sentinel")

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=oauth&state=state",
        )
        self.assertEqual(followed_urls, [engine._build_authenticated_oauth_url()])

    def test_resolve_post_registration_callback_returns_generic_error_when_oauth_still_blocked(self):
        engine = EngineStub()
        engine._post_create_page_type = "add_phone"
        engine._post_create_continue_url = "https://auth.openai.com/add-phone"
        engine._get_workspace_id = lambda: None
        engine._follow_redirects = lambda url: None
        runner = FLOW_RUNNER_MODULE.RegisterFlowRunner(engine=engine)

        result = runner.resolve_post_registration_callback("did", "sentinel")

        self.assertIsNone(result.callback_url)
        self.assertEqual(result.workspace_id, "")
        self.assertEqual(result.metadata, {})
        self.assertEqual(
            result.error_message,
            "跟随重定向链失败",
        )


if __name__ == "__main__":
    unittest.main()
