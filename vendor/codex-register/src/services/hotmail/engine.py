from contextlib import AbstractContextManager
import os
from typing import Any, Callable, Optional
from urllib.parse import urlparse
import re
import uuid

from .profile import build_registration_profile, choose_target_domains
from .types import (
    HotmailAccountArtifact,
    HotmailChallengeHandoff,
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
PROFILE_PROGRESS_STATES = {"profile_details", "birth_details", "name_details"}
MONTH_LABEL_MAP = {
    "1": ("January", "1月"),
    "2": ("February", "2月"),
    "3": ("March", "3月"),
    "4": ("April", "4月"),
    "5": ("May", "5月"),
    "6": ("June", "6月"),
    "7": ("July", "7月"),
    "8": ("August", "8月"),
    "9": ("September", "9月"),
    "10": ("October", "10月"),
    "11": ("November", "11月"),
    "12": ("December", "12月"),
}
COUNTRY_LABEL_MAP = {
    "united states": ("United States", "美国"),
}


def classify_hotmail_page_state(text: str) -> Optional[HotmailFailureCode]:
    normalized = str(text or "").lower()
    if (
        "phone number" in normalized
        or "mobile number" in normalized
        or "电话号码" in normalized
        or "手机号码" in normalized
    ):
        return HotmailFailureCode.PHONE_VERIFICATION_REQUIRED
    if (
        "puzzle" in normalized
        or "captcha" in normalized
        or "verify you are human" in normalized
        or "prove you're not a robot" in normalized
        or "hold the button" in normalized
        or "证明你不是机器人" in normalized
        or "长按该按钮" in normalized
        or "按住该按钮" in normalized
        or "长按按钮" in normalized
    ):
        return HotmailFailureCode.UNSUPPORTED_CHALLENGE
    if (
        "account creation has been blocked" in normalized
        or "blocked the creation of this account" in normalized
        or "we have detected some unusual activity" in normalized
        or "我们检测到一些异常活动" in normalized
        or "已阻止创建此帐户" in normalized
    ):
        return HotmailFailureCode.ACCOUNT_CREATION_BLOCKED
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
        from patchright.sync_api import sync_playwright

        from ...core.browser_runtime import resolve_register_chrome_executable_path

        self.playwright = sync_playwright().start()
        launch_kwargs: dict[str, Any] = {
            "headless": self._launch_headless(),
            "args": [
                "--no-sandbox",
                "--disable-blink-features=AutomationControlled",
            ],
        }
        chrome_path = resolve_register_chrome_executable_path()
        if chrome_path:
            launch_kwargs["executable_path"] = chrome_path
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

    @staticmethod
    def _launch_headless() -> bool:
        return str(os.getenv("HOTMAIL_HANDOFF_ENABLED", "")).strip().lower() not in {
            "1",
            "true",
            "yes",
            "on",
        }

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
        if not self._click_locator(locator):
            return False
        locator.fill("")
        locator.fill(value)
        return True

    def _click_locator(self, locator) -> bool:
        try:
            locator.click()
            return True
        except Exception:
            try:
                locator.click(force=True)
                return True
            except Exception:
                try:
                    locator.evaluate("el => el.click()")
                    return True
                except Exception:
                    return False

    def _click_first(self, selectors: list[str]) -> bool:
        locator = self._first_locator(selectors)
        if locator is None:
            return False
        return self._click_locator(locator)

    def _click_primary_action(self) -> bool:
        return self._click_first(
            [
                "#nextButton",
                "button[type='submit']",
                "input[type='submit']",
                "button:has-text('Next')",
                "button:has-text('Create account')",
                "button:has-text('Verify')",
                "button:has-text('下一步')",
                "button:has-text('同意并继续')",
                "button:has-text('验证')",
            ]
        )

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

    def _select_role_option(self, labels: list[str]) -> bool:
        assert self.page is not None
        for label in labels:
            try:
                option = self.page.get_by_role("option", name=label, exact=True)
                if option.count() > 0:
                    option.first.click()
                    return True
            except Exception:
                continue
            try:
                option = self.page.get_by_text(label, exact=True)
                if option.count() > 0:
                    option.first.click()
                    return True
            except Exception:
                continue
        return False

    def _select_dropdown_option(
        self,
        trigger_selectors: list[str],
        labels: list[str],
        *,
        fallback_value: Optional[str] = None,
    ) -> bool:
        if self._select_first(trigger_selectors, fallback_value or labels[0]):
            return True
        if not self._click_first(trigger_selectors):
            return False
        assert self.page is not None
        self.page.wait_for_timeout(300)
        return self._select_role_option(labels)

    def _click_data_transfer_consent(self) -> bool:
        if self._detect_state() != "data_transfer_consent":
            return False
        clicked = self._click_first(
            [
                "#nextButton",
                "button:has-text('同意并继续')",
                "button:has-text('Agree and continue')",
                "button[type='submit']",
            ]
        )
        if clicked:
            assert self.page is not None
            self.page.wait_for_timeout(1_500)
        return clicked

    def _page_text(self) -> str:
        assert self.page is not None
        try:
            return self.page.locator("body").inner_text(timeout=2_000)
        except Exception:
            return ""

    @staticmethod
    def _normalize_handoff_cookie(cookie: Any) -> Optional[dict[str, Any]]:
        if not isinstance(cookie, dict):
            return None

        name = str(cookie.get("name") or "").strip()
        value = str(cookie.get("value") or "").strip()
        domain = str(cookie.get("domain") or "").strip()
        path = str(cookie.get("path") or "").strip() or "/"
        if not name or not value:
            return None

        expires_raw = cookie.get("expires")
        expires: Optional[int] = None
        if isinstance(expires_raw, (int, float)):
            try:
                expires_int = int(expires_raw)
                expires = expires_int if expires_int > 0 else None
            except Exception:
                expires = None

        same_site = str(cookie.get("sameSite") or cookie.get("same_site") or "").strip()
        if not same_site:
            same_site = "Lax"

        return {
            "name": name,
            "value": value,
            "domain": domain,
            "path": path,
            "expires": expires,
            "http_only": bool(cookie.get("httpOnly") or cookie.get("http_only")),
            "secure": bool(cookie.get("secure")),
            "same_site": same_site,
        }

    def _collect_origin_storage(self) -> list[dict[str, Any]]:
        assert self.page is not None
        try:
            result = self.page.evaluate(
                """() => {
                    try {
                        const origin = window.location.origin || "";
                        const entries = [];
                        for (let index = 0; index < window.localStorage.length; index += 1) {
                            const name = window.localStorage.key(index);
                            if (!name) {
                                continue;
                            }
                            entries.push({
                                name,
                                value: window.localStorage.getItem(name) ?? "",
                            });
                        }
                        return [{ origin, local_storage: entries }];
                    } catch (error) {
                        return [];
                    }
                }"""
            )
        except Exception:
            return []

        if not isinstance(result, list):
            return []

        origins: list[dict[str, Any]] = []
        for item in result:
            if not isinstance(item, dict):
                continue
            origin = str(item.get("origin") or "").strip()
            raw_entries = item.get("local_storage")
            if not isinstance(raw_entries, list):
                raw_entries = item.get("localStorage")
            entries: list[dict[str, str]] = []
            if isinstance(raw_entries, list):
                for entry in raw_entries:
                    if not isinstance(entry, dict):
                        continue
                    name = str(entry.get("name") or "").strip()
                    value = str(entry.get("value") or "")
                    if not name:
                        continue
                    entries.append({"name": name, "value": value})
            if origin or entries:
                origins.append({"origin": origin, "local_storage": entries})
        return origins

    def export_handoff_state(self) -> dict[str, Any]:
        assert self.context is not None
        assert self.page is not None

        cookies: list[dict[str, Any]] = []
        try:
            for cookie in self.context.cookies():
                normalized = self._normalize_handoff_cookie(cookie)
                if normalized is not None:
                    cookies.append(normalized)
        except Exception:
            cookies = []

        try:
            user_agent = str(self.page.evaluate("() => navigator.userAgent") or "").strip()
        except Exception:
            user_agent = ""

        return {
            "user_agent": user_agent,
            "cookies": cookies,
            "origins": self._collect_origin_storage(),
        }

    def build_debug_snapshot(self) -> str:
        if self.page is None:
            return ""

        try:
            title = str(self.page.title() or "").strip()
        except Exception:
            title = ""

        text = " ".join(self._page_text().split())
        state = ""
        try:
            state = str(self._detect_state() or "").strip()
        except Exception:
            state = ""

        parts = [
            f"url={str(self.page.url or '').strip()}",
            f"title={title}",
            f"state={state}",
            f"text={text[:600]}",
        ]
        return " | ".join(part for part in parts if part and not part.endswith("="))

    def _detect_state(self) -> str:
        assert self.page is not None
        text = self._page_text().lower()

        for marker in USERNAME_UNAVAILABLE_MARKERS:
            if marker in text:
                return "username_unavailable"

        if (
            "个人数据导出许可" in text
            or "同意并继续" in text and "拒绝并退出" in text
            or "personal data export license" in text
        ):
            return "data_transfer_consent"

        failure = classify_hotmail_page_state(text)
        if failure == HotmailFailureCode.PHONE_VERIFICATION_REQUIRED:
            return "phone_verification"
        if failure == HotmailFailureCode.UNSUPPORTED_CHALLENGE:
            return "unsupported_challenge"

        if self._first_locator(
            [
                "input[name='email']",
                "input[type='email']",
                "input[autocomplete='email']",
                "input[aria-label*='email' i]",
                "input[aria-label='电子邮件']",
            ]
        ):
            return "email_entry"

        if self._first_locator(
            [
                "input[type='password']",
                "input[name='Password']",
                "input[autocomplete='new-password']",
            ]
        ):
            return "password_entry"

        if self._first_locator(
            [
                "#lastNameInput",
                "#firstNameInput",
                "input[name='lastNameInput']",
                "input[name='firstNameInput']",
            ]
        ):
            return "name_details"

        if self._first_locator(
            [
                "input[name='BirthYear']",
                "#BirthMonthDropdown",
                "#BirthDayDropdown",
                "#countryDropdownId",
                "input[aria-label='出生年份']",
            ]
        ):
            return "birth_details"

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
                "input[inputmode='numeric']",
                "input[aria-label*='code' i]",
            ]
        ):
            return "email_verification"

        if any(
            marker in text
            for marker in (
                *EMAIL_VERIFICATION_MARKERS,
                "输入代码",
                "检查你的电子邮件",
                "验证码",
                "发送代码",
                "验证电子邮件",
            )
        ):
            return "email_verification"

        current_url = (self.page.url or "").lower()
        if any(marker in current_url for marker in SUCCESS_URL_MARKERS) and "signup.live.com" not in current_url:
            return "success"
        if (
            "your account has been created" in text
            or "welcome to your microsoft account" in text
            or "你的 microsoft 帐户已创建" in text
            or "欢迎使用 microsoft 帐户" in text
        ):
            return "success"
        return "page_structure_changed"

    def open_signup(self) -> None:
        assert self.page is not None
        self.page.goto("https://signup.live.com/signup", wait_until="domcontentloaded", timeout=60_000)
        self.page.wait_for_timeout(1_500)
        self._click_data_transfer_consent()

    def submit_account_credentials(self, *, email: str, password: str) -> str:
        assert self.page is not None
        self._click_data_transfer_consent()

        if not self._fill_first(
            [
                "input[name='email']",
                "input[type='email']",
                "input[autocomplete='email']",
                "input[autocomplete='username']",
                "input[aria-label*='email' i]",
                "input[aria-label='电子邮件']",
                "input[name='MemberName']",
            ],
            email,
        ):
            return "page_structure_changed"

        self._click_primary_action()
        self.page.wait_for_timeout(1_200)
        state = self._detect_state()
        if state == "username_unavailable":
            return state
        if state == "password_entry":
            password_filled = self._fill_first(
                [
                    "input[type='password']",
                    "input[name='Password']",
                    "input[autocomplete='new-password']",
                ],
                password,
            )
            if password_filled:
                self._click_primary_action()
                self.page.wait_for_timeout(1_200)
                state = self._detect_state()
        return state

    def submit_profile_details(self, *, profile: HotmailRegistrationProfile) -> str:
        state = self._detect_state()
        if state == "birth_details":
            if not self._fill_first(
                ["input[name='BirthYear']", "input[id*='BirthYear']", "input[aria-label='出生年份']"],
                profile.birth_year,
            ):
                return "page_structure_changed"

            country_labels = list(
                COUNTRY_LABEL_MAP.get(profile.country.strip().lower(), (profile.country,))
            )
            self._select_dropdown_option(
                ["#countryDropdownId", "button[name='countryDropdownName']", "select[name='Country']"],
                country_labels,
                fallback_value=country_labels[0],
            )

            month_labels = list(MONTH_LABEL_MAP.get(profile.birth_month, (profile.birth_month,)))
            if not self._select_dropdown_option(
                ["#BirthMonthDropdown", "button[name='BirthMonth']", "select[name='BirthMonth']"],
                month_labels,
                fallback_value=profile.birth_month,
            ):
                return "page_structure_changed"

            day_labels = [f"{profile.birth_day}日", profile.birth_day]
            if not self._select_dropdown_option(
                ["#BirthDayDropdown", "button[name='BirthDay']", "select[name='BirthDay']"],
                day_labels,
                fallback_value=profile.birth_day,
            ):
                return "page_structure_changed"

            self._click_primary_action()
            assert self.page is not None
            self.page.wait_for_timeout(1_200)
            state = self._detect_state()

        if state in {"name_details", "profile_details"}:
            first_name_ok = self._fill_first(
                [
                    "#firstNameInput",
                    "input[name='firstNameInput']",
                    "input[name='FirstName']",
                    "input[autocomplete='given-name']",
                    "input[aria-label='First name']",
                    "input[aria-label='名字']",
                ],
                profile.first_name,
            )
            last_name_ok = self._fill_first(
                [
                    "#lastNameInput",
                    "input[name='lastNameInput']",
                    "input[name='LastName']",
                    "input[autocomplete='family-name']",
                    "input[aria-label='Last name']",
                    "input[aria-label='姓氏']",
                ],
                profile.last_name,
            )
            if not first_name_ok and not last_name_ok:
                return "page_structure_changed"

            self._click_primary_action()
            assert self.page is not None
            self.page.wait_for_timeout(1_200)
            state = self._detect_state()

        if state == "birth_details":
            return "page_structure_changed"

        assert self.page is not None
        return state

    def submit_verification_code(self, code: str) -> str:
        if not self._fill_first(
            [
                "input[name='Code']",
                "input[name='otc']",
                "input[autocomplete='one-time-code']",
                "input[type='tel']",
                "input[inputmode='numeric']",
            ],
            code,
        ):
            return "page_structure_changed"
        self._click_primary_action()
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

    @staticmethod
    def _close_session(session) -> None:
        exit_fn = getattr(session, "__exit__", None)
        if callable(exit_fn):
            try:
                exit_fn(None, None, None)
            except Exception:
                return

    @staticmethod
    def _get_session_url(session) -> str:
        page = getattr(session, "page", None)
        return str(getattr(page, "url", "") or "").strip()

    @staticmethod
    def _get_session_title(session) -> str:
        page = getattr(session, "page", None)
        if page is None:
            return ""
        try:
            return str(page.title() or "").strip()
        except Exception:
            return ""

    def build_handoff_payload(self, handoff: HotmailChallengeHandoff) -> dict[str, Any]:
        session = handoff.session
        public_url = str(os.getenv("HOTMAIL_HANDOFF_PUBLIC_URL", "") or "").strip()
        state = self._promote_state_from_snapshot(str(getattr(handoff, "state", "") or ""), session)
        local_state: dict[str, Any] = {}
        export_state = getattr(session, "export_handoff_state", None)
        if callable(export_state):
            try:
                local_state = export_state() or {}
            except Exception:
                local_state = {}
        return {
            "handoff_id": handoff.handoff_id,
            "url": public_url,
            "title": self._get_session_title(session),
            "instructions": (
                "请在运行 register 服务的主机上处理当前微软验证页，"
                "处理完成后回到面板点击“继续注册”；如果当前部署没有可交互桌面/浏览器流，只能放弃本次。"
            ),
            "local_handoff": {
                "handoff_id": handoff.handoff_id,
                "url": self._get_session_url(session),
                "title": self._get_session_title(session),
                "user_agent": str(local_state.get("user_agent") or "").strip(),
                "proxy_url": str(getattr(session, "proxy_url", "") or "").strip(),
                "state": state,
                "cookies": list(local_state.get("cookies") or []),
                "origins": list(local_state.get("origins") or []),
            },
        }

    def _build_handoff_result(
        self,
        *,
        session,
        profile: HotmailRegistrationProfile,
        email: str,
        domain: str,
        state: str,
        message: str,
        handoff: Optional[HotmailChallengeHandoff] = None,
    ) -> HotmailRegistrationResult:
        challenge_handoff = handoff or HotmailChallengeHandoff(
            handoff_id=uuid.uuid4().hex,
            session=session,
            profile=profile,
            email=email,
            domain=domain,
            state=state,
        )
        challenge_handoff.state = state
        return HotmailRegistrationResult(
            success=False,
            reason_code=HotmailFailureCode.UNSUPPORTED_CHALLENGE.value,
            error_message=message,
            handoff_context=challenge_handoff,
        )

    @staticmethod
    def _append_debug_snapshot(message: str, session) -> str:
        snapshot_builder = getattr(session, "build_debug_snapshot", None)
        if not callable(snapshot_builder):
            return message
        try:
            snapshot = str(snapshot_builder() or "").strip()
        except Exception:
            snapshot = ""
        if not snapshot:
            return message
        return f"{message} | {snapshot}"

    @staticmethod
    def _extract_state_from_snapshot(session) -> str:
        snapshot_builder = getattr(session, "build_debug_snapshot", None)
        if not callable(snapshot_builder):
            return ""
        try:
            snapshot = str(snapshot_builder() or "")
        except Exception:
            snapshot = ""
        if not snapshot:
            return ""
        match = re.search(r"(?:^|\s|\|)state=([^|]+)", snapshot)
        if not match:
            return ""
        return match.group(1).strip().split()[0].lower()

    def _promote_state_from_snapshot(self, state: str, session) -> str:
        normalized = str(state or "").strip().lower()
        if normalized != "page_structure_changed":
            return normalized
        snapshot_state = self._extract_state_from_snapshot(session)
        if snapshot_state and snapshot_state != normalized:
            return snapshot_state
        return normalized

    def _state_to_failure(self, state: str) -> Optional[HotmailFailureCode]:
        mapping = {
            "phone_verification": HotmailFailureCode.PHONE_VERIFICATION_REQUIRED,
            "unsupported_challenge": HotmailFailureCode.UNSUPPORTED_CHALLENGE,
            "account_creation_blocked": HotmailFailureCode.ACCOUNT_CREATION_BLOCKED,
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
        handoff: Optional[HotmailChallengeHandoff] = None,
        verification_email: str = "",
        verification_service_id: str = "",
        verification_code_provider=None,
        callback_reporter=None,
    ) -> HotmailRegistrationResult:
        if callable(callback_reporter):
            callback_reporter("running", "waiting_email_code")
        if callable(verification_code_provider):
            code = verification_code_provider(
                timeout=self.email_code_timeout,
                poll_interval=self.email_code_poll_interval,
            )
        else:
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

        raw_state = str(state or "").strip().lower()
        state = self._promote_state_from_snapshot(raw_state, session)
        failure = self._state_to_failure(state)
        if failure is not None:
            message = f"Hotmail verification flow failed: {state}"
            if failure == HotmailFailureCode.PAGE_STRUCTURE_CHANGED or state != raw_state:
                message = self._append_debug_snapshot(message, session)
            if failure == HotmailFailureCode.UNSUPPORTED_CHALLENGE:
                return self._build_handoff_result(
                    session=session,
                    profile=profile,
                    email=email,
                    domain=domain,
                    state=state,
                    message=message,
                    handoff=handoff,
                )
            return self._build_failure_result(failure, message)

        return self._build_failure_result(
            HotmailFailureCode.PAGE_STRUCTURE_CHANGED,
            self._append_debug_snapshot(f"Unexpected verification state: {state}", session),
        )

    def _handle_post_credentials_state(
        self,
        *,
        session,
        state: str,
        profile: HotmailRegistrationProfile,
        email: str,
        domain: str,
        handoff: Optional[HotmailChallengeHandoff] = None,
        verification_email: str = "",
        verification_service_id: str = "",
        verification_code_provider=None,
        callback_reporter=None,
    ) -> HotmailRegistrationResult:
        current_state = str(state or "").strip().lower()
        for _ in range(3):
            if current_state not in PROFILE_PROGRESS_STATES:
                break
            current_state = str(session.submit_profile_details(profile=profile)).strip().lower()
        raw_state = current_state
        current_state = self._promote_state_from_snapshot(current_state, session)

        if current_state == "email_verification":
            return self._complete_email_verification(
                session=session,
                profile=profile,
                email=email,
                domain=domain,
                handoff=handoff,
                verification_email=verification_email,
                verification_service_id=verification_service_id,
                verification_code_provider=verification_code_provider,
                callback_reporter=callback_reporter,
            )

        if current_state == "success":
            return self._build_success_result(email=email, domain=domain, profile=profile)

        failure = self._state_to_failure(current_state)
        if failure is not None:
            message = f"Hotmail signup failed: {current_state}"
            if failure == HotmailFailureCode.PAGE_STRUCTURE_CHANGED or current_state != raw_state:
                message = self._append_debug_snapshot(message, session)
            if failure == HotmailFailureCode.UNSUPPORTED_CHALLENGE:
                return self._build_handoff_result(
                    session=session,
                    profile=profile,
                    email=email,
                    domain=domain,
                    state=current_state,
                    message=message,
                    handoff=handoff,
                )
            return self._build_failure_result(failure, message)

        return self._build_failure_result(
            HotmailFailureCode.PAGE_STRUCTURE_CHANGED,
            self._append_debug_snapshot(f"Unexpected signup state: {current_state}", session),
        )

    def _attempt_browser_domain(
        self,
        *,
        session,
        profile: HotmailRegistrationProfile,
        domain: str,
        verification_email: str = "",
        verification_service_id: str = "",
        verification_code_provider=None,
        callback_reporter=None,
    ) -> Optional[HotmailRegistrationResult]:
        for username in profile.username_candidates:
            email = f"{username}@{domain}"
            self._log(f"尝试注册 Hotmail 账号: {email}")
            if callable(callback_reporter):
                callback_reporter("running", "submitting_credentials", log_line=f"attempt {email}")
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
                verification_email=verification_email,
                verification_service_id=verification_service_id,
                verification_code_provider=verification_code_provider,
                callback_reporter=callback_reporter,
            )
        return None

    def run_local_first(
        self,
        *,
        profile: HotmailRegistrationProfile,
        target_domains: list[str],
        verification_email: str,
        verification_service_id: str = "",
        verification_code_provider=None,
        callback_reporter=None,
    ):
        session = None
        keep_session = False
        try:
            session = self._open_browser_session()
            session.__enter__()
            if callable(callback_reporter):
                callback_reporter("running", "opening_signup")
            session.open_signup()
            for domain in target_domains or choose_target_domains():
                result = self._attempt_browser_domain(
                    session=session,
                    profile=profile,
                    domain=domain,
                    verification_email=verification_email,
                    verification_service_id=verification_service_id,
                    verification_code_provider=verification_code_provider,
                    callback_reporter=callback_reporter,
                )
                if result is not None:
                    keep_session = result.handoff_context is not None
                    return result
            return self._build_failure_result(
                HotmailFailureCode.USERNAME_UNAVAILABLE_EXHAUSTED,
                "No domain attempt succeeded",
            )
        except TimeoutError as exc:
            return self._build_failure_result(HotmailFailureCode.BROWSER_TIMEOUT, f"Hotmail browser timeout: {exc}")
        except Exception as exc:
            return self._build_failure_result(HotmailFailureCode.UNEXPECTED_EXCEPTION, str(exc))
        finally:
            if session is not None and not keep_session:
                self._close_session(session)

    def resume_handoff(self, handoff: HotmailChallengeHandoff) -> HotmailRegistrationResult:
        session = handoff.session
        keep_session = False
        try:
            current_state = self._promote_state_from_snapshot(
                getattr(session, "_detect_state", lambda: "page_structure_changed")(),
                session,
            )
            result = self._handle_post_credentials_state(
                session=session,
                state=current_state,
                profile=handoff.profile,
                email=handoff.email,
                domain=handoff.domain,
                handoff=handoff,
            )
            keep_session = result.handoff_context is not None
            return result
        except TimeoutError as exc:
            return self._build_failure_result(HotmailFailureCode.BROWSER_TIMEOUT, f"Hotmail browser timeout: {exc}")
        except Exception as exc:
            return self._build_failure_result(HotmailFailureCode.UNEXPECTED_EXCEPTION, str(exc))
        finally:
            if not keep_session:
                self._close_session(session)

    def abandon_handoff(self, handoff: HotmailChallengeHandoff) -> None:
        self._close_session(handoff.session)

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

        session = None
        keep_session = False
        try:
            profile = self._build_profile()
            self._log(
                "Hotmail 注册资料: "
                f"first_name={profile.first_name} "
                f"last_name={profile.last_name} "
                f"birth={profile.birth_year}-{profile.birth_month}-{profile.birth_day} "
                f"username_candidates={profile.username_candidates}"
            )
            session = self._open_browser_session()
            session.__enter__()
            session.open_signup()
            for domain in choose_target_domains():
                result = self._attempt_browser_domain(
                    session=session,
                    profile=profile,
                    domain=domain,
                )
                if result is not None:
                    keep_session = result.handoff_context is not None
                    return result
        except TimeoutError as exc:
            return self._build_failure_result(HotmailFailureCode.BROWSER_TIMEOUT, f"Hotmail browser timeout: {exc}")
        except ModuleNotFoundError as exc:
            return self._build_failure_result(HotmailFailureCode.UNEXPECTED_EXCEPTION, str(exc))
        except Exception as exc:
            return self._build_failure_result(HotmailFailureCode.UNEXPECTED_EXCEPTION, str(exc))
        finally:
            if session is not None and not keep_session:
                self._close_session(session)

        return self._build_failure_result(
            HotmailFailureCode.USERNAME_UNAVAILABLE_EXHAUSTED,
            "No username candidate succeeded for any configured Hotmail domain",
        )
