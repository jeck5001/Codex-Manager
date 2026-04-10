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
        self.assertIn('ARG PLAYWRIGHT_BROWSER_INSTALL_ARGS="chromium"', content)
        self.assertIn("libnss3", content)
        self.assertIn("x11vnc", content)
        self.assertIn("websockify", content)
        self.assertIn("novnc", content)
        self.assertIn("/app/docker/start-register.sh", content)
        self.assertNotIn("    curl \\\n", content)

    def test_local_register_dockerfile_installs_playwright_browser_runtime(self):
        content = (REPO_ROOT / "docker" / "Dockerfile.register.local").read_text(encoding="utf-8")
        self.assertIn("playwright install", content)
        self.assertIn("PLAYWRIGHT_BROWSER_INSTALL_ARGS", content)
        self.assertIn("/app/docker/start-register.sh", content)

    def test_ghcr_compose_exposes_hotmail_handoff_port(self):
        content = (REPO_ROOT / "docker" / "docker-compose.ghcr.yml").read_text(encoding="utf-8")
        self.assertIn('HOTMAIL_HANDOFF_ENABLED: "1"', content)
        self.assertIn('HOTMAIL_HANDOFF_PORT: "7900"', content)
        self.assertIn('HOTMAIL_HANDOFF_VNC_PORT: "5900"', content)
        self.assertIn('- "7900:7900"', content)
        self.assertIn('- "5900:5900"', content)

    def test_vendor_register_dockerignore_excludes_test_and_cache_directories(self):
        content = (REGISTER_ROOT / ".dockerignore").read_text(encoding="utf-8")
        self.assertIn("tests/", content)
        self.assertIn(".pytest_cache/", content)


if __name__ == "__main__":
    unittest.main()
