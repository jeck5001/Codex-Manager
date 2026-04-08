import unittest

from src.services.hotmail.profile import (
    HOTMAIL_DOMAIN_POLICY,
    build_username_candidates,
    choose_target_domains,
)
from src.services.hotmail.types import HotmailAccountArtifact, HotmailFailureCode


class HotmailProfileTests(unittest.TestCase):
    def test_domain_policy_constant_prefers_hotmail_then_outlook(self):
        self.assertEqual(HOTMAIL_DOMAIN_POLICY, ("hotmail.com", "outlook.com"))

    def test_choose_target_domains_prefers_hotmail_then_outlook(self):
        self.assertEqual(
            choose_target_domains(),
            ["hotmail.com", "outlook.com"],
        )

    def test_build_username_candidates_normalizes_ascii_safe_values(self):
        candidates = build_username_candidates("Alice", "Example", seed="ab12")
        self.assertIn("aliceexampleab12", candidates)
        self.assertTrue(all("@" not in item for item in candidates))

    def test_hotmail_artifact_txt_line_matches_outlook_import_format(self):
        artifact = HotmailAccountArtifact(
            email="demo@hotmail.com",
            password="StrongPassw0rd!",
            target_domain="hotmail.com",
            verification_email="code@temp.example.com",
        )
        self.assertEqual(artifact.to_txt_line(), "demo@hotmail.com----StrongPassw0rd!")

    def test_failure_code_phone_verification_is_stable(self):
        self.assertEqual(
            HotmailFailureCode.PHONE_VERIFICATION_REQUIRED.value,
            "phone_verification_required",
        )


if __name__ == "__main__":
    unittest.main()
