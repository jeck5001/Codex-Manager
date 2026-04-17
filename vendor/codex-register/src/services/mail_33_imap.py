"""
33mail + IMAP 邮箱服务

使用 33mail 的别名能力生成注册邮箱，再通过真实收件箱的 IMAP 收取转发后的验证码邮件。
"""

from __future__ import annotations

import email
import imaplib
import logging
import random
import re
import string
import time
from datetime import datetime
from email.header import decode_header, make_header
from email.message import Message
from email.utils import parsedate_to_datetime
from typing import Any, Dict, List, Optional

from .base import BaseEmailService, EmailServiceError, EmailServiceType
from ..config.constants import OPENAI_VERIFICATION_KEYWORDS, OTP_CODE_PATTERN


logger = logging.getLogger(__name__)


class Mail33ImapService(BaseEmailService):
    """33mail 别名 + IMAP 收码实现。"""

    DEFAULT_POLL_INTERVAL = 3
    DEFAULT_TIMEOUT = 120
    DEFAULT_ALIAS_LENGTH = 12
    DEFAULT_SUBJECT_KEYWORD = "Your ChatGPT code is"
    _GENERIC_SUBJECT_FILTERS = {"openai", "chatgpt"}
    _SUBJECT_FILTER_ALIASES = tuple(
        dict.fromkeys(
            [
                DEFAULT_SUBJECT_KEYWORD.lower(),
                "chatgpt log-in code",
                "chatgpt log in code",
                "openai",
                "chatgpt",
                *[str(item or "").strip().lower() for item in OPENAI_VERIFICATION_KEYWORDS],
            ]
        )
    )

    def __init__(self, config: Dict[str, Any] = None, name: str = None):
        super().__init__(EmailServiceType.MAIL_33_IMAP, name)

        default_config = {
            "alias_domain": "",
            "real_inbox_email": "",
            "imap_host": "",
            "imap_port": 993,
            "imap_username": "",
            "imap_password": "",
            "imap_mailbox": "INBOX",
            "imap_ssl": True,
            "from_filter": "openai.com",
            "subject_keyword": self.DEFAULT_SUBJECT_KEYWORD,
            "otp_pattern": OTP_CODE_PATTERN,
            "poll_interval": self.DEFAULT_POLL_INTERVAL,
            "timeout": self.DEFAULT_TIMEOUT,
            "alias_length": self.DEFAULT_ALIAS_LENGTH,
        }

        self.config = {**default_config, **(config or {})}
        self._validate_required_config()

    def _validate_required_config(self):
        required_keys = [
            "alias_domain",
            "real_inbox_email",
            "imap_host",
            "imap_username",
            "imap_password",
        ]
        missing = [
            key for key in required_keys if not str(self.config.get(key) or "").strip()
        ]
        if missing:
            raise ValueError(f"缺少必需配置: {missing}")

    @staticmethod
    def _normalize_alias_domain(value: Any) -> str:
        domain = str(value or "").strip().strip()
        domain = domain.lstrip("@").strip(".").lower()
        return domain

    @staticmethod
    def _normalize_email_text(value: Any) -> str:
        return str(value or "").strip().lower()

    @classmethod
    def _split_filter_values(cls, value: Any) -> List[str]:
        if isinstance(value, (list, tuple, set)):
            values: List[str] = []
            for item in value:
                values.extend(cls._split_filter_values(item))
            return values

        normalized = cls._normalize_email_text(value)
        if not normalized:
            return []

        parts = re.split(r"[,，、;；\n]+", normalized)
        return [part.strip() for part in parts if part.strip()]

    @classmethod
    def _subject_filter_values(cls, value: Any) -> List[str]:
        filters: List[str] = []
        for token in cls._split_filter_values(value):
            filters.append(token)
            if token in cls._GENERIC_SUBJECT_FILTERS:
                filters.extend(cls._SUBJECT_FILTER_ALIASES)

        deduped: List[str] = []
        seen = set()
        for item in filters:
            if item and item not in seen:
                deduped.append(item)
                seen.add(item)
        return deduped

    @staticmethod
    def _matches_any_filter(filters: List[str], texts: List[str]) -> bool:
        return any(
            candidate and candidate in text
            for candidate in filters
            for text in texts
            if text
        )

    @staticmethod
    def _build_alias_local_part(length: int) -> str:
        normalized_length = max(4, int(length or Mail33ImapService.DEFAULT_ALIAS_LENGTH))
        return "".join(random.choice(string.ascii_lowercase + string.digits) for _ in range(normalized_length))

    def create_email(self, config: Dict[str, Any] = None) -> Dict[str, Any]:
        runtime_config = {**self.config, **(config or {})}
        alias_domain = self._normalize_alias_domain(runtime_config.get("alias_domain"))
        if not alias_domain:
            raise EmailServiceError("alias_domain 不能为空")

        local_part = self._build_alias_local_part(runtime_config.get("alias_length"))
        email_address = f"{local_part}@{alias_domain}"
        forward_to = str(runtime_config.get("real_inbox_email") or "").strip()

        self.update_status(True)
        return {
            "email": email_address,
            "service_id": email_address,
            "id": email_address,
            "alias": local_part,
            "domain": alias_domain,
            "forward_to": forward_to,
        }

    def _open_imap(self):
        host = str(self.config.get("imap_host") or "").strip()
        port = int(self.config.get("imap_port") or 993)
        username = str(self.config.get("imap_username") or "").strip()
        password = str(self.config.get("imap_password") or "").strip()
        use_ssl = bool(self.config.get("imap_ssl", True))

        if use_ssl:
            conn = imaplib.IMAP4_SSL(host, port)
        else:
            conn = imaplib.IMAP4(host, port)

        conn.login(username, password)
        mailbox = str(self.config.get("imap_mailbox") or "INBOX").strip() or "INBOX"
        status, _ = conn.select(mailbox)
        if status != "OK":
            raise EmailServiceError(f"选择邮箱目录失败: {mailbox}")
        return conn

    @staticmethod
    def _decode_header_value(value: Any) -> str:
        if not value:
            return ""
        try:
            return str(make_header(decode_header(str(value))))
        except Exception:
            return str(value)

    @staticmethod
    def _message_timestamp(message: Message) -> float:
        header = message.get("Date")
        if not header:
            return 0.0
        try:
            return parsedate_to_datetime(header).timestamp()
        except Exception:
            return 0.0

    @classmethod
    def _collect_message_body(cls, part: Message) -> str:
        if part.is_multipart():
            pieces = []
            for child in part.walk():
                if child.is_multipart():
                    continue
                content_disposition = str(child.get("Content-Disposition") or "").lower()
                if "attachment" in content_disposition:
                    continue
                payload = cls._decode_message_payload(child)
                if payload:
                    pieces.append(payload)
            return "\n".join(piece for piece in pieces if piece)
        return cls._decode_message_payload(part)

    @staticmethod
    def _decode_message_payload(part: Message) -> str:
        payload = part.get_payload(decode=True)
        if payload is None:
            raw = part.get_payload()
            return raw if isinstance(raw, str) else ""

        charset = part.get_content_charset() or "utf-8"
        try:
            return payload.decode(charset, errors="ignore")
        except Exception:
            try:
                return payload.decode("utf-8", errors="ignore")
            except Exception:
                return ""

    def _list_recent_messages(self) -> List[Dict[str, Any]]:
        conn = self._open_imap()
        try:
            status, data = conn.search(None, "ALL")
            if status != "OK":
                raise EmailServiceError("检索 IMAP 邮件失败")

            message_ids = data[0].split()[-20:]
            messages: List[Dict[str, Any]] = []
            for message_id in message_ids:
                fetch_status, payload = conn.fetch(message_id, "(RFC822)")
                if fetch_status != "OK" or not payload:
                    continue
                raw_bytes = None
                for item in payload:
                    if isinstance(item, tuple) and len(item) >= 2:
                        raw_bytes = item[1]
                        break
                if not raw_bytes:
                    continue
                parsed = email.message_from_bytes(raw_bytes)
                messages.append(
                    {
                        "id": message_id.decode(errors="ignore"),
                        "to": self._decode_header_value(
                            parsed.get("Delivered-To")
                            or parsed.get("X-Original-To")
                            or parsed.get("To")
                        ),
                        "from": self._decode_header_value(parsed.get("From")),
                        "subject": self._decode_header_value(parsed.get("Subject")),
                        "body": self._collect_message_body(parsed),
                        "timestamp": self._message_timestamp(parsed),
                    }
                )
            return messages
        finally:
            try:
                conn.close()
            except Exception:
                pass
            try:
                conn.logout()
            except Exception:
                pass

    def _extract_verification_code_from_messages(
        self,
        email: str,
        messages: List[Dict[str, Any]],
        otp_sent_at: Optional[float] = None,
    ) -> Optional[str]:
        target_email = self._normalize_email_text(email)
        from_filters = self._split_filter_values(self.config.get("from_filter"))
        subject_filters = self._subject_filter_values(self.config.get("subject_keyword"))
        pattern = str(self.config.get("otp_pattern") or OTP_CODE_PATTERN)

        eligible: List[Dict[str, Any]] = []
        for message in messages:
            recipient_text = self._normalize_email_text(message.get("to"))
            sender_text = self._normalize_email_text(message.get("from"))
            subject_text = self._normalize_email_text(message.get("subject"))
            body_text = self._normalize_email_text(message.get("body"))
            timestamp = float(message.get("timestamp") or 0)

            if target_email and not any(
                target_email in text
                for text in [recipient_text, subject_text, body_text]
            ):
                continue
            if from_filters and not self._matches_any_filter(from_filters, [sender_text]):
                continue
            if subject_filters and not self._matches_any_filter(
                subject_filters,
                [subject_text, body_text],
            ):
                continue
            if otp_sent_at and timestamp and timestamp + 1 < float(otp_sent_at):
                continue

            eligible.append(message)

        eligible.sort(key=lambda item: float(item.get("timestamp") or 0), reverse=True)
        for message in eligible:
            body = str(message.get("body") or "")
            match = re.search(pattern, body)
            if match:
                return match.group(1)
        return None

    def get_verification_code(
        self,
        email: str,
        email_id: str = None,
        timeout: int = 120,
        poll_interval: Optional[int] = None,
        pattern: str = r"(?<!\d)(\d{6})(?!\d)",
        otp_sent_at: Optional[float] = None,
    ) -> Optional[str]:
        original_pattern = self.config.get("otp_pattern")
        self.config["otp_pattern"] = pattern or original_pattern or OTP_CODE_PATTERN
        try:
            deadline = time.time() + max(10, int(timeout or self.config.get("timeout") or self.DEFAULT_TIMEOUT))
            sleep_seconds = max(
                1,
                int(
                    poll_interval
                    or self.config.get("poll_interval")
                    or self.DEFAULT_POLL_INTERVAL
                ),
            )
            while time.time() < deadline:
                try:
                    messages = self._list_recent_messages()
                    code = self._extract_verification_code_from_messages(
                        email=email,
                        messages=messages,
                        otp_sent_at=otp_sent_at,
                    )
                    if code:
                        self.update_status(True)
                        return code
                except Exception as exc:
                    logger.warning("33mail IMAP 收码失败: %s", exc)
                    self.update_status(False, exc)
                time.sleep(sleep_seconds)
            return None
        finally:
            self.config["otp_pattern"] = original_pattern

    def list_emails(self, **kwargs) -> List[Dict[str, Any]]:
        return []

    def delete_email(self, email_id: str) -> bool:
        return True

    def check_health(self) -> bool:
        try:
            conn = self._open_imap()
            try:
                conn.close()
            except Exception:
                pass
            try:
                conn.logout()
            except Exception:
                pass
            self.update_status(True)
            return True
        except Exception as exc:
            self.update_status(False, exc)
            logger.warning("33mail IMAP 健康检查失败: %s", exc)
            return False

    def get_service_info(self) -> Dict[str, Any]:
        return {
            "service_type": "mail_33_imap",
            "alias_domain": self._normalize_alias_domain(self.config.get("alias_domain")),
            "real_inbox_email": str(self.config.get("real_inbox_email") or "").strip(),
            "imap_host": str(self.config.get("imap_host") or "").strip(),
            "imap_port": int(self.config.get("imap_port") or 993),
            "imap_mailbox": str(self.config.get("imap_mailbox") or "INBOX").strip() or "INBOX",
            "imap_ssl": bool(self.config.get("imap_ssl", True)),
        }
