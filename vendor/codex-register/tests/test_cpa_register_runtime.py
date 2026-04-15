import importlib.util
import sys
import unittest
from pathlib import Path


def load_cpa_page_driver_module():
    module_name = "src.core.cpa_page_driver"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "core"
        / "cpa_page_driver.py"
    )

    for name in list(sys.modules):
        if name == "src" or name.startswith("src."):
            sys.modules.pop(name, None)

    src_dir = Path(__file__).resolve().parents[1] / "src"
    core_dir = src_dir / "core"
    src_pkg = type(sys)("src")
    src_pkg.__path__ = [str(src_dir)]
    core_pkg = type(sys)("src.core")
    core_pkg.__path__ = [str(core_dir)]
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def load_cpa_register_runtime_module():
    module_name = "src.core.cpa_register_runtime"
    module_path = (
        Path(__file__).resolve().parents[1]
        / "src"
        / "core"
        / "cpa_register_runtime.py"
    )

    for name in list(sys.modules):
        if name == "src" or name.startswith("src."):
            sys.modules.pop(name, None)

    src_dir = Path(__file__).resolve().parents[1] / "src"
    core_dir = src_dir / "core"
    src_pkg = type(sys)("src")
    src_pkg.__path__ = [str(src_dir)]
    core_pkg = type(sys)("src.core")
    core_pkg.__path__ = [str(core_dir)]
    sys.modules["src"] = src_pkg
    sys.modules["src.core"] = core_pkg

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


class CPAPageDriverTests(unittest.TestCase):
    def test_classify_signup_state_prefers_existing_account_signal(self):
        module = load_cpa_page_driver_module()

        state = module.classify_signup_state(
            {
                "is_signup_password_page": True,
                "page_text": "Account associated with this email address already exists",
                "has_retry_button": False,
            }
        )

        self.assertEqual(state["kind"], "email_exists")
        self.assertFalse(state["retryable"])

    def test_classify_signup_state_marks_password_retry_as_retryable(self):
        module = load_cpa_page_driver_module()

        state = module.classify_signup_state(
            {
                "is_signup_password_page": True,
                "page_text": "Something went wrong. Operation timed out.",
                "has_retry_button": True,
            }
        )

        self.assertEqual(state["kind"], "password_retry")
        self.assertTrue(state["retryable"])


class CPARegisterRuntimeTests(unittest.TestCase):
    def test_resolve_callback_payload_accepts_query_fragment_mix(self):
        module = load_cpa_register_runtime_module()

        payload = module.resolve_callback_payload(
            "http://localhost:1455/auth/callback?code=abc123#state=xyz456"
        )

        self.assertEqual(payload["code"], "abc123")
        self.assertEqual(payload["state"], "xyz456")

    def test_normalize_callback_url_rebuilds_redirect_uri_from_payload(self):
        module = load_cpa_register_runtime_module()

        class OAuthStart:
            redirect_uri = "http://localhost:1455/auth/callback"

        class Engine:
            oauth_start = OAuthStart()

        runtime = module.CPARegisterRuntime(Engine())
        normalized = runtime.normalize_callback_url(
            "code=abc123&state=xyz456"
        )

        self.assertEqual(
            normalized,
            "http://localhost:1455/auth/callback?code=abc123&state=xyz456",
        )

    def test_resolve_post_registration_callback_prefers_post_create_continue_url(self):
        module = load_cpa_register_runtime_module()

        class FlowRunner:
            def __init__(self):
                self.calls = []

            def resolve_callback_from_continue_url(self, continue_url, stage):
                self.calls.append(("continue", continue_url, stage))
                return "http://localhost:1455/auth/callback?code=from-create&state=state"

            def resolve_post_registration_callback(self, did, sen_token):
                self.calls.append(("fallback", did, sen_token))
                raise AssertionError("fallback should not be called when create continue_url resolves")

        class Engine:
            _post_create_continue_url = "https://auth.openai.com/continue?state=create"

            def __init__(self):
                self.flow_runner = FlowRunner()
                self.logged = []

            def _get_flow_runner(self):
                return self.flow_runner

            def _get_workspace_id(self):
                return "ws_123"

            def _log(self, message, level="info"):
                self.logged.append((level, message))

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.resolve_post_registration_callback("did-1", "sentinel-1")

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=from-create&state=state",
        )
        self.assertEqual(result.workspace_id, "ws_123")
        self.assertEqual(
            runtime.engine.flow_runner.calls,
            [("continue", "https://auth.openai.com/continue?state=create", "注册后继续")],
        )

    def test_resolve_post_registration_callback_falls_back_when_continue_url_unresolved(self):
        module = load_cpa_register_runtime_module()

        class FallbackResult:
            callback_url = "http://localhost:1455/auth/callback?code=from-fallback&state=state"
            workspace_id = "ws_fallback"
            error_message = ""
            metadata = {"source": "oauth"}

        class FlowRunner:
            def __init__(self):
                self.calls = []

            def resolve_callback_from_continue_url(self, continue_url, stage):
                self.calls.append(("continue", continue_url, stage))
                return None

            def resolve_post_registration_callback(self, did, sen_token):
                self.calls.append(("fallback", did, sen_token))
                return FallbackResult()

        class Engine:
            _post_create_continue_url = "https://auth.openai.com/add-phone"

            def __init__(self):
                self.flow_runner = FlowRunner()

            def _get_flow_runner(self):
                return self.flow_runner

            def _get_workspace_id(self):
                return ""

            def _log(self, _message, level="info"):
                return level

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.resolve_post_registration_callback("did-2", "sentinel-2")

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=from-fallback&state=state",
        )
        self.assertEqual(result.workspace_id, "ws_fallback")
        self.assertEqual(result.metadata, {"source": "oauth"})
        self.assertEqual(
            runtime.engine.flow_runner.calls,
            [
                ("continue", "https://auth.openai.com/add-phone", "注册后继续"),
                ("fallback", "did-2", "sentinel-2"),
            ],
        )

    def test_resolve_post_registration_callback_uses_login_recovery_when_oauth_collection_fails(self):
        module = load_cpa_register_runtime_module()

        class FallbackResult:
            callback_url = None
            workspace_id = ""
            error_message = "跟随重定向链失败"
            metadata = {}

        class FlowRunner:
            def resolve_post_registration_callback(self, did, sen_token):
                return FallbackResult()

        class Engine:
            _post_create_continue_url = ""
            email = "user@example.com"
            password = "secret"

            def __init__(self):
                self.flow_runner = FlowRunner()
                self.recovery_calls = []
                self.logged = []

            def _get_flow_runner(self):
                return self.flow_runner

            def _get_workspace_id(self):
                return ""

            def _attempt_add_phone_login_bypass(self, did, sen_token):
                self.recovery_calls.append((did, sen_token))
                return "http://localhost:1455/auth/callback?code=recovered&state=state"

            def _log(self, message, level="info"):
                self.logged.append((level, message))

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.resolve_post_registration_callback("did-3", "sentinel-3")

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=recovered&state=state",
        )
        self.assertEqual(result.error_message, "")
        self.assertEqual(runtime.engine.recovery_calls, [("did-3", "sentinel-3")])

    def test_resolve_post_registration_callback_preserves_error_without_login_credentials(self):
        module = load_cpa_register_runtime_module()

        class FallbackResult:
            callback_url = None
            workspace_id = ""
            error_message = "跟随重定向链失败"
            metadata = {}

        class FlowRunner:
            def resolve_post_registration_callback(self, did, sen_token):
                return FallbackResult()

        class Engine:
            _post_create_continue_url = ""
            email = "user@example.com"
            password = ""

            def __init__(self):
                self.flow_runner = FlowRunner()

            def _get_flow_runner(self):
                return self.flow_runner

            def _get_workspace_id(self):
                return ""

            def _attempt_add_phone_login_bypass(self, did, sen_token):
                raise AssertionError("should not try login recovery without password")

            def _log(self, _message, level="info"):
                return level

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.resolve_post_registration_callback("did-4", "sentinel-4")

        self.assertIsNone(result.callback_url)
        self.assertEqual(result.error_message, "跟随重定向链失败")

    def test_resolve_post_registration_callback_uses_login_recovery_even_without_fallback_error(self):
        module = load_cpa_register_runtime_module()

        class FallbackResult:
            callback_url = None
            workspace_id = ""
            error_message = ""
            metadata = {}

        class FlowRunner:
            def resolve_post_registration_callback(self, did, sen_token):
                return FallbackResult()

        class Engine:
            _post_create_continue_url = ""
            email = "user@example.com"
            password = "secret"

            def __init__(self):
                self.flow_runner = FlowRunner()
                self.recovery_calls = []

            def _get_flow_runner(self):
                return self.flow_runner

            def _get_workspace_id(self):
                return ""

            def _attempt_login_recovery(self, did, sen_token):
                self.recovery_calls.append((did, sen_token))
                return "http://localhost:1455/auth/callback?code=recovered&state=state"

            def _attempt_add_phone_login_bypass(self, did, sen_token):
                raise AssertionError("should prefer generic login recovery over legacy add-phone bypass")

            def _log(self, _message, level="info"):
                return level

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.resolve_post_registration_callback("did-login", "sentinel-login")

        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=recovered&state=state",
        )
        self.assertEqual(result.error_message, "")
        self.assertEqual(runtime.engine.recovery_calls, [("did-login", "sentinel-login")])

    def test_execute_signup_sequence_skips_password_and_create_for_existing_account(self):
        module = load_cpa_register_runtime_module()

        class Engine:
            _is_existing_account = True
            _otp_sent_at = None

            def __init__(self):
                self.calls = []

            def _log(self, _message, level="info"):
                return level

            def _submit_signup_form(self, did, sen_token):
                self.calls.append(("submit", did, sen_token))
                return type("SignupResult", (), {"success": True, "error_message": ""})()

            def _register_password(self):
                raise AssertionError("should skip password registration for existing account")

            def _send_verification_code(self):
                raise AssertionError("should skip OTP send for existing account")

            def _wait_for_signup_verification_code(self):
                self.calls.append(("wait_code",))
                return "123456"

            def _validate_signup_verification_code_with_retry(self, code):
                self.calls.append(("validate", code))
                return True

            def _create_user_account(self):
                raise AssertionError("should skip account creation for existing account")

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.execute_signup_sequence("did-1", "sentinel-1")

        self.assertTrue(result.success)
        self.assertEqual(
            runtime.engine.calls,
            [
                ("submit", "did-1", "sentinel-1"),
                ("wait_code",),
                ("validate", "123456"),
            ],
        )
        self.assertIsNotNone(runtime.engine._otp_sent_at)

    def test_execute_signup_sequence_stops_on_password_failure(self):
        module = load_cpa_register_runtime_module()

        class Engine:
            _is_existing_account = False
            _otp_sent_at = None

            def __init__(self):
                self.calls = []

            def _log(self, _message, level="info"):
                return level

            def _submit_signup_form(self, did, sen_token):
                self.calls.append(("submit", did, sen_token))
                return type("SignupResult", (), {"success": True, "error_message": ""})()

            def _register_password(self):
                self.calls.append(("password",))
                return False, None

            def _send_verification_code(self):
                raise AssertionError("should stop before sending OTP")

            def _wait_for_signup_verification_code(self):
                raise AssertionError("should stop before waiting OTP")

            def _validate_signup_verification_code_with_retry(self, code):
                raise AssertionError("should stop before validating OTP")

            def _create_user_account(self):
                raise AssertionError("should stop before account creation")

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.execute_signup_sequence("did-2", "sentinel-2")

        self.assertFalse(result.success)
        self.assertEqual(result.error_message, "注册密码失败")
        self.assertEqual(
            runtime.engine.calls,
            [
                ("submit", "did-2", "sentinel-2"),
                ("password",),
            ],
        )

    def test_execute_signup_sequence_skips_otp_when_password_step_does_not_require_it(self):
        module = load_cpa_register_runtime_module()

        class Engine:
            _is_existing_account = False
            _otp_sent_at = None
            _signup_password_needs_otp = True

            def __init__(self):
                self.calls = []

            def _log(self, _message, level="info"):
                return level

            def _submit_signup_form(self, did, sen_token):
                self.calls.append(("submit", did, sen_token))
                return type("SignupResult", (), {"success": True, "error_message": ""})()

            def _register_password(self):
                self.calls.append(("password",))
                self._signup_password_needs_otp = False
                return True, "secret"

            def _send_verification_code(self):
                raise AssertionError("should skip OTP send when password step does not require OTP")

            def _wait_for_signup_verification_code(self):
                raise AssertionError("should skip OTP wait when password step does not require OTP")

            def _validate_signup_verification_code_with_retry(self, code):
                raise AssertionError("should skip OTP validation when password step does not require OTP")

            def _create_user_account(self):
                self.calls.append(("create_account",))
                return True

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.execute_signup_sequence("did-optional-otp", "sentinel-optional-otp")

        self.assertTrue(result.success)
        self.assertEqual(
            runtime.engine.calls,
            [
                ("submit", "did-optional-otp", "sentinel-optional-otp"),
                ("password",),
                ("create_account",),
            ],
        )

    def test_execute_signup_sequence_retries_retryable_signup_failure(self):
        module = load_cpa_register_runtime_module()

        class Engine:
            _is_existing_account = False
            _otp_sent_at = None

            def __init__(self):
                self.calls = []
                self.submit_attempts = 0

            def _log(self, _message, level="info"):
                return level

            def _submit_signup_form(self, did, sen_token):
                self.submit_attempts += 1
                self.calls.append(("submit", self.submit_attempts, did, sen_token))
                if self.submit_attempts == 1:
                    return type(
                        "SignupResult",
                        (),
                        {
                            "success": False,
                            "error_message": "CPA signup password page hit retryable error, 请重试当前流程",
                            "retryable": True,
                        },
                    )()
                return type(
                    "SignupResult",
                    (),
                    {"success": True, "error_message": "", "retryable": False},
                )()

            def _register_password(self):
                self.calls.append(("password",))
                return True, "secret"

            def _send_verification_code(self):
                self.calls.append(("send_otp",))
                return True

            def _wait_for_signup_verification_code(self):
                self.calls.append(("wait_code",))
                return "123456"

            def _validate_signup_verification_code_with_retry(self, code):
                self.calls.append(("validate", code))
                return True

            def _create_user_account(self):
                self.calls.append(("create_account",))
                return True

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.execute_signup_sequence("did-3", "sentinel-3")

        self.assertTrue(result.success)
        self.assertEqual(
            runtime.engine.calls,
            [
                ("submit", 1, "did-3", "sentinel-3"),
                ("submit", 2, "did-3", "sentinel-3"),
                ("password",),
                ("send_otp",),
                ("wait_code",),
                ("validate", "123456"),
                ("create_account",),
            ],
        )

    def test_execute_signup_sequence_fails_after_retryable_signup_exhaustion(self):
        module = load_cpa_register_runtime_module()

        class Engine:
            _is_existing_account = False
            _otp_sent_at = None

            def __init__(self):
                self.calls = []

            def _log(self, _message, level="info"):
                return level

            def _submit_signup_form(self, did, sen_token):
                self.calls.append(("submit", did, sen_token))
                return type(
                    "SignupResult",
                    (),
                    {
                        "success": False,
                        "error_message": "CPA signup password page hit retryable error, 请重试当前流程",
                        "retryable": True,
                    },
                )()

            def _register_password(self):
                raise AssertionError("should stop before password registration")

            def _send_verification_code(self):
                raise AssertionError("should stop before sending OTP")

            def _wait_for_signup_verification_code(self):
                raise AssertionError("should stop before waiting OTP")

            def _validate_signup_verification_code_with_retry(self, code):
                raise AssertionError("should stop before validating OTP")

            def _create_user_account(self):
                raise AssertionError("should stop before account creation")

        runtime = module.CPARegisterRuntime(Engine())
        result = runtime.execute_signup_sequence("did-4", "sentinel-4")

        self.assertFalse(result.success)
        self.assertIn("retryable", result.error_message.lower())
        self.assertEqual(
            runtime.engine.calls,
            [
                ("submit", "did-4", "sentinel-4"),
                ("submit", "did-4", "sentinel-4"),
                ("submit", "did-4", "sentinel-4"),
            ],
        )

    def test_complete_registration_flow_waits_for_post_signup_sync_before_collecting_callback(self):
        module = load_cpa_register_runtime_module()

        class FlowRunner:
            def __init__(self, calls):
                self.calls = calls

            def resolve_callback_from_continue_url(self, continue_url, stage):
                self.calls.append(("continue", continue_url, stage))
                return "http://localhost:1455/auth/callback?code=from-create&state=state"

            def resolve_post_registration_callback(self, did, sen_token):
                raise AssertionError("should not fall back when create-account continue_url resolves")

        class Engine:
            _post_create_continue_url = "https://auth.openai.com/continue?state=create"
            _is_existing_account = False

            def __init__(self):
                self.calls = []
                self.flow_runner = FlowRunner(self.calls)

            def _get_flow_runner(self):
                return self.flow_runner

            def _get_workspace_id(self):
                return "ws_123"

            def _wait_for_post_signup_sync(self):
                self.calls.append(("wait",))

            def _log(self, _message, level="info"):
                return level

        runtime = module.CPARegisterRuntime(Engine())
        runtime.execute_signup_sequence = lambda did, sen_token: module.CPASignupSequenceResult(success=True)

        result = runtime.complete_registration_flow("did-flow", "sentinel-flow")

        self.assertTrue(result.success)
        self.assertEqual(
            result.callback_url,
            "http://localhost:1455/auth/callback?code=from-create&state=state",
        )
        self.assertEqual(result.workspace_id, "ws_123")
        self.assertEqual(
            runtime.engine.calls,
            [
                ("wait",),
                ("continue", "https://auth.openai.com/continue?state=create", "注册后继续"),
            ],
        )


if __name__ == "__main__":
    unittest.main()
