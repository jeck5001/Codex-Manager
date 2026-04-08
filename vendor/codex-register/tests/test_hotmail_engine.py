import unittest

from src.services.hotmail.engine import (
    HotmailRegistrationEngine,
    classify_hotmail_page_state,
)
from src.services.hotmail.types import HotmailFailureCode


class HotmailEngineTests(unittest.TestCase):
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

    def test_engine_tries_outlook_after_hotmail_availability_failure(self):
        engine = HotmailRegistrationEngine.__new__(HotmailRegistrationEngine)
        attempted = []

        def fake_attempt(domain: str) -> bool:
            attempted.append(domain)
            return domain == "outlook.com"

        engine._attempt_domain = fake_attempt

        self.assertEqual(engine._choose_domain_by_attempt(), "outlook.com")
        self.assertEqual(attempted, ["hotmail.com", "outlook.com"])


if __name__ == "__main__":
    unittest.main()
