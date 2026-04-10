import sys
import types
import unittest
from unittest.mock import patch

from src.services.hotmail.engine import (
    PlaywrightHotmailBrowserSession,
    HotmailRegistrationEngine,
    classify_hotmail_page_state,
)
from src.services.hotmail.types import (
    HotmailChallengeHandoff,
    HotmailFailureCode,
    HotmailRegistrationProfile,
)


class HotmailEngineTests(unittest.TestCase):
    def _build_profile(self) -> HotmailRegistrationProfile:
        return HotmailRegistrationProfile(
            first_name="Alice",
            last_name="Example",
            birth_day="8",
            birth_month="4",
            birth_year="1998",
            password="StrongPassw0rd!",
            username_candidates=["aliceexample", "aliceexample1"],
        )

    def test_classify_phone_verification_page(self):
        self.assertEqual(
            classify_hotmail_page_state("Add a phone number to help keep your account secure"),
            HotmailFailureCode.PHONE_VERIFICATION_REQUIRED,
        )

    def test_classify_unsupported_challenge(self):
        self.assertEqual(
            classify_hotmail_page_state("Complete the puzzle to continue"),
            HotmailFailureCode.UNSUPPORTED_CHALLENGE,
        )

    def test_classify_chinese_hold_button_challenge(self):
        self.assertEqual(
            classify_hotmail_page_state("证明你不是机器人 长按该按钮"),
            HotmailFailureCode.UNSUPPORTED_CHALLENGE,
        )

    def test_classify_account_creation_blocked_page(self):
        self.assertEqual(
            classify_hotmail_page_state(
                "Account creation has been blocked We have detected some unusual activity and have blocked the creation of this account."
            ),
            HotmailFailureCode.ACCOUNT_CREATION_BLOCKED,
        )

    def test_engine_tries_outlook_after_hotmail_availability_failure(self):
        engine = HotmailRegistrationEngine.__new__(HotmailRegistrationEngine)
        attempted = []

        def fake_attempt(domain: str) -> bool:
            attempted.append(domain)
            return domain == "outlook.com"

        engine._attempt_domain = fake_attempt

        self.assertEqual(engine._choose_domain_by_attempt(), "outlook.com")
        self.assertEqual(attempted, ["hotmail.com", "outlook.com"])

    def test_engine_returns_username_exhausted_when_no_domain_attempt_succeeds(self):
        engine = HotmailRegistrationEngine()
        result = engine.run()

        self.assertFalse(result.success)
        self.assertEqual(result.reason_code, HotmailFailureCode.USERNAME_UNAVAILABLE_EXHAUSTED.value)

    def test_engine_returns_success_when_email_verification_completed(self):
        mailbox_service = types.SimpleNamespace(
            create_email=lambda config=None: {
                "email": "verify@temp.example.com",
                "service_id": "mailbox-1",
            },
            get_verification_code=lambda **_kwargs: "654321",
        )
        verification_provider = types.SimpleNamespace(
            acquire_mailbox=lambda: types.SimpleNamespace(
                name="temp-mail-1",
                service_type="temp_mail",
                service=mailbox_service,
            )
        )

        class FakeBrowserSession:
            def __init__(self):
                self.credentials = []
                self.profile = None
                self.code = None

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def open_signup(self):
                return None

            def submit_account_credentials(self, *, email, password):
                self.credentials.append((email, password))
                return "profile_details"

            def submit_profile_details(self, *, profile):
                self.profile = profile
                return "email_verification"

            def submit_verification_code(self, code: str):
                self.code = code
                return "success"

        session = FakeBrowserSession()
        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: session,
            verification_provider=verification_provider,
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertTrue(result.success)
        self.assertEqual(result.artifact.email, "aliceexample@hotmail.com")
        self.assertEqual(result.artifact.password, "StrongPassw0rd!")
        self.assertEqual(result.artifact.target_domain, "hotmail.com")
        self.assertEqual(result.artifact.verification_email, "verify@temp.example.com")
        self.assertEqual(session.code, "654321")
        self.assertEqual(
            session.credentials,
            [("aliceexample@hotmail.com", "StrongPassw0rd!")],
        )

    def test_engine_handles_modern_birth_details_state(self):
        class FakeBrowserSession:
            def __init__(self):
                self.profile = None

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def open_signup(self):
                return None

            def submit_account_credentials(self, *, email, password):
                return "birth_details"

            def submit_profile_details(self, *, profile):
                self.profile = profile
                return "success"

        session = FakeBrowserSession()
        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: session,
            verification_provider=object(),
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertTrue(result.success)
        self.assertIsNotNone(session.profile)
        self.assertEqual(result.artifact.email, "aliceexample@hotmail.com")

    def test_engine_advances_split_profile_flow_until_success(self):
        class FakeBrowserSession:
            def __init__(self):
                self.profile_calls = 0

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def open_signup(self):
                return None

            def submit_account_credentials(self, *, email, password):
                return "birth_details"

            def submit_profile_details(self, *, profile):
                self.profile_calls += 1
                if self.profile_calls == 1:
                    return "name_details"
                return "success"

        session = FakeBrowserSession()
        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: session,
            verification_provider=object(),
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertTrue(result.success)
        self.assertEqual(session.profile_calls, 2)
        self.assertEqual(result.artifact.email, "aliceexample@hotmail.com")

    def test_click_first_falls_back_to_force_click(self):
        class FakeLocator:
            def __init__(self):
                self.calls = []

            def click(self, **kwargs):
                self.calls.append(kwargs)
                if not kwargs.get("force"):
                    raise RuntimeError("intercepts pointer events")
                return None

        session = PlaywrightHotmailBrowserSession()
        locator = FakeLocator()
        session._first_locator = lambda selectors: locator

        self.assertTrue(session._click_first(["#BirthMonthDropdown"]))
        self.assertEqual(locator.calls, [{}, {"force": True}])

    def test_browser_session_runs_headed_when_handoff_enabled(self):
        launch_observed = {}

        class FakeBrowser:
            def new_context(self, **_kwargs):
                class FakeContext:
                    def new_page(self):
                        return object()

                    def close(self):
                        return None

                return FakeContext()

            def close(self):
                return None

        class FakePlaywright:
            chromium = None

            def __init__(self):
                self.chromium = self

            def launch(self, **kwargs):
                launch_observed.update(kwargs)
                return FakeBrowser()

            def stop(self):
                return None

        class FakeStarter:
            def start(self):
                return FakePlaywright()

        fake_playwright_module = types.ModuleType("playwright")
        fake_sync_api_module = types.ModuleType("playwright.sync_api")
        fake_sync_api_module.sync_playwright = lambda: FakeStarter()

        session = PlaywrightHotmailBrowserSession()
        with patch.dict("os.environ", {"HOTMAIL_HANDOFF_ENABLED": "1"}, clear=False):
            with patch.dict(
                sys.modules,
                {
                    "playwright": fake_playwright_module,
                    "playwright.sync_api": fake_sync_api_module,
                },
            ):
                session.__enter__()
        session.__exit__(None, None, None)

        self.assertEqual(launch_observed.get("headless"), False)

    def test_engine_build_handoff_payload_prefers_public_vnc_url(self):
        engine = HotmailRegistrationEngine()
        handoff = types.SimpleNamespace(
            handoff_id="handoff-public",
            session=types.SimpleNamespace(page=types.SimpleNamespace(url="https://signup.live.com/signup")),
        )

        with patch.dict(
            "os.environ",
            {"HOTMAIL_HANDOFF_PUBLIC_URL": "http://192.168.5.35:7900/vnc.html?autoconnect=1"},
            clear=False,
        ):
            payload = engine.build_handoff_payload(handoff)

        self.assertEqual(
            payload["url"],
            "http://192.168.5.35:7900/vnc.html?autoconnect=1",
        )

    def test_engine_build_handoff_payload_includes_local_handoff_state(self):
        engine = HotmailRegistrationEngine()

        class FakePage:
            url = "https://signup.live.com/signup"

            def title(self):
                return "Let's prove you're human"

        class FakeSession:
            def __init__(self):
                self.page = FakePage()
                self.proxy_url = "http://127.0.0.1:7890"

            def build_debug_snapshot(self):
                return (
                    "url=https://signup.live.com/signup | "
                    "title=Let's prove you're human | "
                    "state=unsupported_challenge"
                )

            def export_handoff_state(self):
                return {
                    "user_agent": "Mozilla/5.0 test",
                    "cookies": [
                        {
                            "name": "MSPRequ",
                            "value": "cookie-value",
                            "domain": ".live.com",
                            "path": "/",
                            "expires": 1760000000,
                            "http_only": True,
                            "secure": True,
                            "same_site": "None",
                        }
                    ],
                    "origins": [
                        {
                            "origin": "https://signup.live.com",
                            "local_storage": [{"name": "k", "value": "v"}],
                        }
                    ],
                }

        handoff = HotmailChallengeHandoff(
            handoff_id="handoff-1",
            session=FakeSession(),
            profile=self._build_profile(),
            email="aliceexample@hotmail.com",
            domain="hotmail.com",
            state="unsupported_challenge",
        )

        payload = engine.build_handoff_payload(handoff)

        self.assertEqual(payload["handoff_id"], "handoff-1")
        self.assertIn("local_handoff", payload)
        self.assertEqual(payload["local_handoff"]["state"], "unsupported_challenge")
        self.assertEqual(payload["local_handoff"]["cookies"][0]["name"], "MSPRequ")
        self.assertEqual(
            payload["local_handoff"]["origins"][0]["local_storage"][0]["name"],
            "k",
        )

    def test_engine_build_handoff_payload_degrades_when_local_export_fails(self):
        engine = HotmailRegistrationEngine()

        class FakePage:
            url = "https://signup.live.com/signup"

            def title(self):
                return "Let's prove you're human"

        class FakeSession:
            def __init__(self):
                self.page = FakePage()
                self.proxy_url = None

            def build_debug_snapshot(self):
                return "state=unsupported_challenge"

            def export_handoff_state(self):
                raise RuntimeError("boom")

        handoff = HotmailChallengeHandoff(
            handoff_id="handoff-2",
            session=FakeSession(),
            profile=self._build_profile(),
            email="aliceexample@hotmail.com",
            domain="hotmail.com",
            state="unsupported_challenge",
        )

        payload = engine.build_handoff_payload(handoff)

        self.assertIn("local_handoff", payload)
        self.assertEqual(payload["local_handoff"]["cookies"], [])
        self.assertEqual(payload["local_handoff"]["origins"], [])

    def test_submit_profile_details_supports_name_input_selectors(self):
        class FakePage:
            def wait_for_timeout(self, _ms):
                return None

        class FakeSession(PlaywrightHotmailBrowserSession):
            def __init__(self):
                super().__init__()
                self.page = FakePage()
                self.state = "name_details"
                self.filled = []

            def _detect_state(self):
                return self.state

            def _fill_first(self, selectors, value):
                selector_set = set(selectors)
                if "input[name='firstNameInput']" in selector_set:
                    self.filled.append(("first", value))
                    return True
                if "input[name='lastNameInput']" in selector_set:
                    self.filled.append(("last", value))
                    return True
                return False

            def _click_primary_action(self):
                self.state = "success"
                return True

        session = FakeSession()
        result = session.submit_profile_details(profile=self._build_profile())

        self.assertEqual(result, "success")
        self.assertEqual(session.filled, [("first", "Alice"), ("last", "Example")])

    def test_engine_includes_debug_snapshot_for_page_structure_changed(self):
        class FakeBrowserSession:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def open_signup(self):
                return None

            def submit_account_credentials(self, *, email, password):
                return "page_structure_changed"

            def build_debug_snapshot(self):
                return (
                    "url=https://signup.live.com/debug "
                    "title=Debug Title "
                    "state=page_structure_changed "
                    "text=Unexpected interstitial page"
                )

        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: FakeBrowserSession(),
            verification_provider=object(),
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertFalse(result.success)
        self.assertEqual(result.reason_code, HotmailFailureCode.PAGE_STRUCTURE_CHANGED.value)
        self.assertIn("url=https://signup.live.com/debug", result.error_message)
        self.assertIn("title=Debug Title", result.error_message)

    def test_engine_promotes_snapshot_unsupported_challenge_over_page_structure_changed(self):
        class FakeBrowserSession:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def open_signup(self):
                return None

            def submit_account_credentials(self, *, email, password):
                return "page_structure_changed"

            def build_debug_snapshot(self):
                return (
                    "url=https://signup.live.com/signup "
                    "title=Let's prove you're human "
                    "state=unsupported_challenge "
                    "text=Press and hold the button."
                )

        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: FakeBrowserSession(),
            verification_provider=object(),
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertFalse(result.success)
        self.assertEqual(result.reason_code, HotmailFailureCode.UNSUPPORTED_CHALLENGE.value)
        self.assertIn("Hotmail signup failed: unsupported_challenge", result.error_message)
        self.assertIn("state=unsupported_challenge", result.error_message)

    def test_engine_fails_fast_on_phone_verification(self):
        class FakeBrowserSession:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def open_signup(self):
                return None

            def submit_account_credentials(self, *, email, password):
                return "phone_verification"

        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: FakeBrowserSession(),
            verification_provider=object(),
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertFalse(result.success)
        self.assertEqual(result.reason_code, HotmailFailureCode.PHONE_VERIFICATION_REQUIRED.value)

    def test_engine_maps_account_creation_blocked_state(self):
        class FakeBrowserSession:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def open_signup(self):
                return None

            def submit_account_credentials(self, *, email, password):
                return "account_creation_blocked"

        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: FakeBrowserSession(),
            verification_provider=object(),
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertFalse(result.success)
        self.assertEqual(result.reason_code, HotmailFailureCode.ACCOUNT_CREATION_BLOCKED.value)
        self.assertIn("account_creation_blocked", result.error_message)

    def test_engine_maps_missing_browser_dependency_to_unexpected_exception(self):
        engine = HotmailRegistrationEngine(
            browser_factory=lambda **_kwargs: (_ for _ in ()).throw(ModuleNotFoundError("playwright")),
            verification_provider=object(),
            profile_factory=self._build_profile,
        )

        result = engine.run()

        self.assertFalse(result.success)
        self.assertEqual(result.reason_code, HotmailFailureCode.UNEXPECTED_EXCEPTION.value)
        self.assertIn("playwright", result.error_message.lower())


if __name__ == "__main__":
    unittest.main()
