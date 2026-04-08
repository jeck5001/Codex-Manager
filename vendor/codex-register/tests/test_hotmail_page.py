import unittest
from pathlib import Path


class HotmailPageTests(unittest.TestCase):
    def test_hotmail_template_and_js_expose_batch_controls(self):
        root = Path(__file__).resolve().parents[1]
        template = (root / "templates" / "hotmail.html").read_text(encoding="utf-8")
        script = (root / "static" / "js" / "hotmail.js").read_text(encoding="utf-8")

        self.assertIn('id="hotmail-batch-form"', template)
        self.assertIn('id="hotmail-count"', template)
        self.assertIn("api.post('/hotmail/batches'", script)

    def test_primary_navigation_templates_include_hotmail_link(self):
        root = Path(__file__).resolve().parents[1]
        templates = [
            "index.html",
            "accounts.html",
            "email_services.html",
            "settings.html",
            "payment.html",
            "hotmail.html",
        ]

        for template_name in templates:
            content = (root / "templates" / template_name).read_text(encoding="utf-8")
            self.assertIn('href="/hotmail"', content, template_name)


if __name__ == "__main__":
    unittest.main()
