from dataclasses import dataclass
from typing import Any, Callable, Iterable


SUPPORTED_VERIFICATION_SERVICE_TYPES = ("temp_mail", "custom_domain", "tempmail")


@dataclass
class HotmailVerificationMailbox:
    name: str
    service_type: str
    service: Any


class HotmailVerificationMailboxProvider:
    def __init__(
        self,
        *,
        list_enabled_services: Callable[[], Iterable[Any]],
        create_email_service: Callable[[Any], Any],
    ):
        self._list_enabled_services = list_enabled_services
        self._create_email_service = create_email_service

    def _choose_service(self) -> Any:
        for service in self._list_enabled_services():
            if not getattr(service, "enabled", True):
                continue
            if getattr(service, "service_type", "") not in SUPPORTED_VERIFICATION_SERVICE_TYPES:
                continue
            return service
        raise RuntimeError("No supported verification mailbox service")

    def acquire_mailbox(self) -> HotmailVerificationMailbox:
        service = self._choose_service()
        mailbox_service = self._create_email_service(service)
        return HotmailVerificationMailbox(
            name=str(getattr(service, "name", "") or ""),
            service_type=str(getattr(service, "service_type", "") or ""),
            service=mailbox_service,
        )


def build_default_hotmail_verification_provider() -> HotmailVerificationMailboxProvider:
    from ...config.constants import EmailServiceType
    from ...database.models import EmailService as EmailServiceModel
    from ...database.session import get_db
    from ..base import create_email_service

    def _list_enabled_services():
        with get_db() as db:
            return list(
                db.query(EmailServiceModel)
                .filter(EmailServiceModel.enabled == True)
                .order_by(EmailServiceModel.priority.asc())
                .all()
            )

    def _create_service(service):
        service_type = EmailServiceType(str(getattr(service, "service_type", "") or "").lower())
        return create_email_service(
            service_type,
            dict(getattr(service, "config", {}) or {}),
            getattr(service, "name", None),
        )

    return HotmailVerificationMailboxProvider(
        list_enabled_services=_list_enabled_services,
        create_email_service=_create_service,
    )
