import types
import unittest

from src.services.hotmail.engine import (
    PlaywrightHotmailBrowserSession,
    HotmailRegistrationEngine,
    classify_hotmail_page_state,
)
from src.services.hotmail.types import HotmailFailureCode, HotmailRegistrationProfile


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
