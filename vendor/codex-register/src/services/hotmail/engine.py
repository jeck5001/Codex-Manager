from contextlib import AbstractContextManager
from typing import Any, Callable, Optional
from urllib.parse import urlparse

from .profile import build_registration_profile, choose_target_domains
from .types import (
    HotmailAccountArtifact,
    HotmailFailureCode,
    HotmailRegistrationProfile,
    HotmailRegistrationResult,
)


USERNAME_UNAVAILABLE_MARKERS = (
    "someone already has this email address",
    "this email address is already taken",
    "this email is already taken",
    "this username is already taken",
    "try another name",
    "try another email address",
)
EMAIL_VERIFICATION_MARKERS = (
    "enter the code",
    "check your email",
    "verification code",
    "send code",
    "verify email",
)
SUCCESS_URL_MARKERS = (
    "account.microsoft.com",
    "outlook.live.com",
    "login.live.com",
)


def classify_hotmail_page_state(text: str) -> Optional[HotmailFailureCode]:
    normalized = str(text or "").lower()
    if "phone number" in normalized:
        return HotmailFailureCode.PHONE_VERIFICATION_REQUIRED
    if "puzzle" in normalized or "captcha" in normalized or "verify you are human" in normalized:
        return HotmailFailureCode.UNSUPPORTED_CHALLENGE
    return None


class PlaywrightHotmailBrowserSession(AbstractContextManager):
    def __init__(self, *, proxy_url: Optional[str] = None, callback_logger: Optional[Callable[[str], None]] = None):
        self.proxy_url = proxy_url
        self.callback_logger = callback_logger
        self.playwright = None
        self.browser = None
        self.context = None
        self.page = None

    def __enter__(self):
        from playwright.sync_api import sync_playwright

        self.playwright = sync_playwright().start()
        launch_kwargs: dict[str, Any] = {
            "headless": True,
            "args": [
                "--no-sandbox",
                "--disable-blink-features=AutomationControlled",
            ],
        }
        proxy = self._build_proxy_config(self.proxy_url)
        if proxy:
            launch_kwargs["proxy"] = proxy

        self.browser = self.playwright.chromium.launch(**launch_kwargs)
        self.context = self.browser.new_context(
            viewport={"width": 1440, "height": 960},
            ignore_https_errors=True,
        )
        self.page = self.context.new_page()
        return self

    def __exit__(self, exc_type, exc, tb):
        if self.context is not None:
            self.context.close()
        if self.browser is not None:
            self.browser.close()
        if self.playwright is not None:
            self.playwright.stop()
        return False

    @staticmethod
    def _build_proxy_config(proxy_url: Optional[str]) -> Optional[dict[str, str]]:
        if not proxy_url:
            return None

        parsed = urlparse(proxy_url)
        if not parsed.scheme or not parsed.hostname or not parsed.port:
            return None

        proxy: dict[str, str] = {
            "server": f"{parsed.scheme}://{parsed.hostname}:{parsed.port}",
        }
        if parsed.username:
            proxy["username"] = parsed.username
        if parsed.password:
            proxy["password"] = parsed.password
        return proxy

    def _log(self, message: str) -> None:
        if callable(self.callback_logger):
            self.callback_logger(message)

    def _first_locator(self, selectors: list[str]):
        assert self.page is not None
        for selector in selectors:
            locator = self.page.locator(selector)
            try:
                if locator.count() > 0:
                    return locator.first
            except Exception:
                continue
        return None

    def _fill_first(self, selectors: list[str], value: str) -> bool:
        locator = self._first_locator(selectors)
        if locator is None:
            return False
        locator.click()
        locator.fill("")
        locator.fill(value)
        return True

    def _click_first(self, selectors: list[str]) -> bool:
        locator = self._first_locator(selectors)
        if locator is None:
            return False
        locator.click()
        return True

    def _select_first(self, selectors: list[str], value: str) -> bool:
        locator = self._first_locator(selectors)
        if locator is None:
            return False
        try:
            locator.select_option(value=value)
            return True
        except Exception:
            try:
                locator.select_option(label=value)
                return True
            except Exception:
                return False

    def _page_text(self) -> str:
        assert self.page is not None
        try:
            return self.page.locator("body").inner_text(timeout=2_000)
        except Exception:
            return ""

    def _detect_state(self) -> str:
        assert self.page is not None
        text = self._page_text().lower()

        for marker in USERNAME_UNAVAILABLE_MARKERS:
            if marker in text:
                return "username_unavailable"

        failure = classify_hotmail_page_state(text)
        if failure == HotmailFailureCode.PHONE_VERIFICATION_REQUIRED:
            return "phone_verification"
        if failure == HotmailFailureCode.UNSUPPORTED_CHALLENGE:
            return "unsupported_challenge"

        if self._first_locator(
            [
                "input[name='FirstName']",
                "input[name='LastName']",
                "input[autocomplete='given-name']",
                "input[autocomplete='family-name']",
            ]
        ):
            return "profile_details"

        if self._first_locator(
            [
                "input[name='Code']",
                "input[name='otc']",
                "input[autocomplete='one-time-code']",
                "input[type='tel']",
            ]
        ):
            return "email_verification"

        if any(marker in text for marker in EMAIL_VERIFICATION_MARKERS):
            return "email_verification"

        current_url = (self.page.url or "").lower()
        if any(marker in current_url for marker in SUCCESS_URL_MARKERS) and "signup.live.com" not in current_url:
            return "success"
        if "your account has been created" in text or "welcome to your microsoft account" in text:
            return "success"
        return "page_structure_changed"

    def open_signup(self) -> None:
        assert self.page is not None
        self.page.goto("https://signup.live.com/signup", wait_until="domcontentloaded", timeout=60_000)
        self.page.wait_for_timeout(1_500)

    def submit_account_credentials(self, *, email: str, password: str) -> str:
        assert self.page is not None
        local_part, _, domain = email.partition("@")

        if not self._fill_first(
            [
                "input[name='MemberName']",
                "input[type='email']",
                "input[autocomplete='username']",
                "input[aria-label*='email' i]",
            ],
            local_part if self._first_locator(["select[name='domain']", "select"]) else email,
        ):
            return "page_structure_changed"

        self._select_first(
            [
                "select[name='domain']",
                "select[id*='domain']",
            ],
            domain,
        )
        self._click_first(
            [
                "button[type='submit']",
                "input[type='submit']",
                "button:has-text('Next')",
            ]
        )
        self.page.wait_for_timeout(1_200)
        state = self._detect_state()
        if state != "page_structure_changed":
            if state == "username_unavailable":
                return state
            if state != "success":
                password_filled = self._fill_first(
                    [
                        "input[type='password']",
                        "input[name='Password']",
                        "input[autocomplete='new-password']",
                    ],
                    password,
                )
                if password_filled:
                    self._click_first(
                        [
                            "button[type='submit']",
                            "input[type='submit']",
                            "button:has-text('Next')",
                        ]
                    )
                    self.page.wait_for_timeout(1_200)
                    state = self._detect_state()
        return state

    def submit_profile_details(self, *, profile: HotmailRegistrationProfile) -> str:
        if not self._fill_first(
            [
                "input[name='FirstName']",
                "input[autocomplete='given-name']",
            ],
            profile.first_name,
        ):
            return "page_structure_changed"
        self._fill_first(
            [
                "input[name='LastName']",
                "input[autocomplete='family-name']",
            ],
            profile.last_name,
        )
        self._select_first(["select[name='BirthMonth']", "select[id*='BirthMonth']"], profile.birth_month)
        self._fill_first(["input[name='BirthDay']", "input[id*='BirthDay']"], profile.birth_day)
        self._fill_first(["input[name='BirthYear']", "input[id*='BirthYear']"], profile.birth_year)
        self._select_first(["select[name='Country']", "select[id*='Country']"], profile.country)
        self._click_first(
            [
                "button[type='submit']",
                "input[type='submit']",
                "button:has-text('Next')",
            ]
        )
        assert self.page is not None
        self.page.wait_for_timeout(1_200)
        return self._detect_state()

    def submit_verification_code(self, code: str) -> str:
        if not self._fill_first(
            [
                "input[name='Code']",
                "input[name='otc']",
                "input[autocomplete='one-time-code']",
                "input[type='tel']",
            ],
            code,
        ):
            return "page_structure_changed"
        self._click_first(
            [
                "button[type='submit']",
                "input[type='submit']",
                "button:has-text('Next')",
                "button:has-text('Verify')",
            ]
        )
        assert self.page is not None
        self.page.wait_for_timeout(1_200)
        return self._detect_state()


class HotmailRegistrationEngine:
    def __init__(
        self,
        browser_factory=None,
        verification_provider=None,
        callback_logger=None,
        proxy_url=None,
        profile_factory=None,
        email_code_timeout: int = 180,
        email_code_poll_interval: int = 3,
    ):
        self.browser_factory = browser_factory
        self.verification_provider = verification_provider
        self.callback_logger = callback_logger
        self.proxy_url = proxy_url
        self.profile_factory = profile_factory
        self.email_code_timeout = email_code_timeout
        self.email_code_poll_interval = email_code_poll_interval
        self._attempt_domain = lambda domain: False

    def _log(self, message: str) -> None:
        if callable(self.callback_logger):
            self.callback_logger(message)

    def _choose_domain_by_attempt(self) -> Optional[str]:
        for domain in choose_target_domains():
            if self._attempt_domain(domain):
                return domain
        return None

    def _build_profile(self) -> HotmailRegistrationProfile:
        factory = self.profile_factory or build_registration_profile
        return factory()

    def _open_browser_session(self):
        factory = self.browser_factory or PlaywrightHotmailBrowserSession
        return factory(proxy_url=self.proxy_url, callback_logger=self.callback_logger)

    def _build_success_result(
        self,
        *,
        email: str,
        domain: str,
        profile: HotmailRegistrationProfile,
        verification_email: str = "",
    ) -> HotmailRegistrationResult:
        return HotmailRegistrationResult(
            success=True,
            artifact=HotmailAccountArtifact(
                email=email,
                password=profile.password,
                target_domain=domain,
                verification_email=verification_email,
                first_name=profile.first_name,
                last_name=profile.last_name,
            ),
        )

    def _build_failure_result(self, reason_code: HotmailFailureCode, message: str) -> HotmailRegistrationResult:
        return HotmailRegistrationResult(
            success=False,
            reason_code=reason_code.value,
            error_message=message,
        )

    def _state_to_failure(self, state: str) -> Optional[HotmailFailureCode]:
        mapping = {
            "phone_verification": HotmailFailureCode.PHONE_VERIFICATION_REQUIRED,
            "unsupported_challenge": HotmailFailureCode.UNSUPPORTED_CHALLENGE,
            "page_structure_changed": HotmailFailureCode.PAGE_STRUCTURE_CHANGED,
        }
        return mapping.get(state)

    def _complete_email_verification(
        self,
        *,
        session,
        profile: HotmailRegistrationProfile,
        email: str,
        domain: str,
    ) -> HotmailRegistrationResult:
        if self.verification_provider is None:
            return self._build_failure_result(
                HotmailFailureCode.UNEXPECTED_EXCEPTION,
                "No verification mailbox provider configured",
            )

        mailbox = self.verification_provider.acquire_mailbox()
        mailbox_info = mailbox.service.create_email()
        verification_email = str(mailbox_info.get("email") or "").strip()
        verification_service_id = (
            mailbox_info.get("service_id")
            or mailbox_info.get("id")
            or mailbox_info.get("token")
        )

        code = mailbox.service.get_verification_code(
            email=verification_email,
            email_id=verification_service_id,
            timeout=self.email_code_timeout,
            poll_interval=self.email_code_poll_interval,
        )
        if not code:
            return self._build_failure_result(
                HotmailFailureCode.EMAIL_VERIFICATION_TIMEOUT,
                f"Verification email timeout for {verification_email or mailbox.name}",
            )

        state = str(session.submit_verification_code(str(code))).strip().lower()
        if state == "success":
            return self._build_success_result(
                email=email,
                domain=domain,
                profile=profile,
                verification_email=verification_email,
            )

        failure = self._state_to_failure(state)
        if failure is not None:
            return self._build_failure_result(failure, f"Hotmail verification flow failed: {state}")

        return self._build_failure_result(
            HotmailFailureCode.PAGE_STRUCTURE_CHANGED,
            f"Unexpected verification state: {state}",
        )

    def _handle_post_credentials_state(
        self,
        *,
        session,
        state: str,
        profile: HotmailRegistrationProfile,
        email: str,
        domain: str,
    ) -> HotmailRegistrationResult:
        current_state = str(state or "").strip().lower()
        if current_state == "profile_details":
            current_state = str(session.submit_profile_details(profile=profile)).strip().lower()

        if current_state == "email_verification":
            return self._complete_email_verification(
                session=session,
                profile=profile,
                email=email,
                domain=domain,
            )

        if current_state == "success":
            return self._build_success_result(email=email, domain=domain, profile=profile)

        failure = self._state_to_failure(current_state)
        if failure is not None:
            return self._build_failure_result(failure, f"Hotmail signup failed: {current_state}")

        return self._build_failure_result(
            HotmailFailureCode.PAGE_STRUCTURE_CHANGED,
            f"Unexpected signup state: {current_state}",
        )

    def _attempt_browser_domain(
        self,
        *,
        session,
        profile: HotmailRegistrationProfile,
        domain: str,
    ) -> Optional[HotmailRegistrationResult]:
        for username in profile.username_candidates:
            email = f"{username}@{domain}"
            self._log(f"尝试注册 Hotmail 账号: {email}")
            state = str(
                session.submit_account_credentials(
                    email=email,
                    password=profile.password,
                )
            ).strip().lower()

            if state == "username_unavailable":
                continue

            return self._handle_post_credentials_state(
                session=session,
                state=state,
                profile=profile,
                email=email,
                domain=domain,
            )
        return None

    def run(self):
        if self.browser_factory is None and self.verification_provider is None and self.profile_factory is None:
            selected_domain = self._choose_domain_by_attempt()
            if not selected_domain:
                return HotmailRegistrationResult(
                    success=False,
                    reason_code=HotmailFailureCode.USERNAME_UNAVAILABLE_EXHAUSTED.value,
                    error_message="No domain attempt succeeded",
                )
            return HotmailRegistrationResult(
                success=False,
                reason_code=HotmailFailureCode.UNEXPECTED_EXCEPTION.value,
                error_message=f"Hotmail flow not implemented for {selected_domain}",
            )

        try:
            profile = self._build_profile()
            self._log(
                "Hotmail 注册资料: "
                f"first_name={profile.first_name} "
                f"last_name={profile.last_name} "
                f"birth={profile.birth_year}-{profile.birth_month}-{profile.birth_day} "
                f"username_candidates={profile.username_candidates}"
            )
            with self._open_browser_session() as session:
                session.open_signup()
                for domain in choose_target_domains():
                    result = self._attempt_browser_domain(
                        session=session,
                        profile=profile,
                        domain=domain,
                    )
                    if result is not None:
                        return result
        except TimeoutError as exc:
            return self._build_failure_result(HotmailFailureCode.BROWSER_TIMEOUT, f"Hotmail browser timeout: {exc}")
        except ModuleNotFoundError as exc:
            return self._build_failure_result(HotmailFailureCode.UNEXPECTED_EXCEPTION, str(exc))
        except Exception as exc:
            return self._build_failure_result(HotmailFailureCode.UNEXPECTED_EXCEPTION, str(exc))

        return self._build_failure_result(
            HotmailFailureCode.USERNAME_UNAVAILABLE_EXHAUSTED,
            "No username candidate succeeded for any configured Hotmail domain",
        )
