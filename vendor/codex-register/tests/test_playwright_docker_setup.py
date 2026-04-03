import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
REGISTER_ROOT = REPO_ROOT / "vendor" / "codex-register"


class PlaywrightDockerSetupTests(unittest.TestCase):
    def test_register_requirements_include_playwright_dependency(self):
        content = (REGISTER_ROOT / "requirements.txt").read_text(encoding="utf-8")
        self.assertIn("playwright>=", content)

    def test_vendor_register_dockerfile_installs_playwright_browser_runtime(self):
        content = (REGISTER_ROOT / "Dockerfile").read_text(encoding="utf-8")
        self.assertIn("playwright install", content)
        self.assertIn("chromium", content)
        self.assertIn("libnss3", content)

    def test_local_register_dockerfile_installs_playwright_browser_runtime(self):
        content = (REPO_ROOT / "docker" / "Dockerfile.register.local").read_text(encoding="utf-8")
        self.assertIn("playwright install", content)
        self.assertIn("chromium", content)


if __name__ == "__main__":
    unittest.main()
