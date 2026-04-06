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
        self.assertIn("--only-shell", content)
        self.assertIn("libnss3", content)
        self.assertNotIn("    curl \\\n", content)

    def test_local_register_dockerfile_installs_playwright_browser_runtime(self):
        content = (REPO_ROOT / "docker" / "Dockerfile.register.local").read_text(encoding="utf-8")
        self.assertIn("playwright install", content)
        self.assertIn("PLAYWRIGHT_BROWSER_INSTALL_ARGS", content)

    def test_vendor_register_dockerignore_excludes_test_and_cache_directories(self):
        content = (REGISTER_ROOT / ".dockerignore").read_text(encoding="utf-8")
        self.assertIn("tests/", content)
        self.assertIn(".pytest_cache/", content)


if __name__ == "__main__":
    unittest.main()
