import types
import unittest

from src.services.hotmail.verification import HotmailVerificationMailboxProvider


class HotmailVerificationTests(unittest.TestCase):
    def test_provider_prefers_temp_mail_like_services_for_verification(self):
        service = types.SimpleNamespace(
            name="temp-mail-1",
            service_type="temp_mail",
            enabled=True,
            config={"domain": "tm.example.com"},
        )
        provider = HotmailVerificationMailboxProvider(
            list_enabled_services=lambda: [service],
            create_email_service=lambda selected: selected,
        )

        mailbox = provider.acquire_mailbox()

        self.assertEqual(mailbox.name, "temp-mail-1")

    def test_provider_raises_when_no_supported_service_exists(self):
        provider = HotmailVerificationMailboxProvider(
            list_enabled_services=lambda: [],
            create_email_service=lambda selected: selected,
        )

        with self.assertRaisesRegex(RuntimeError, "No supported verification mailbox service"):
            provider.acquire_mailbox()


if __name__ == "__main__":
    unittest.main()
