import asyncio
import importlib.util
import re
import sys
import types
import unittest
from pathlib import Path


def _load_email_services_module():
    src_dir = Path(__file__).resolve().parents[1] / "src"
    if "src" not in sys.modules:
        src_pkg = types.ModuleType("src")
        src_pkg.__path__ = [str(src_dir)]
        sys.modules["src"] = src_pkg

    web_dir = src_dir / "web"
    if "src.web" not in sys.modules:
        web_pkg = types.ModuleType("src.web")
        web_pkg.__path__ = [str(web_dir)]
        sys.modules["src.web"] = web_pkg

    routes_dir = src_dir / "web" / "routes"
    pkg_name = "src.web.routes"
    if pkg_name not in sys.modules:
        routes_pkg = types.ModuleType(pkg_name)
        routes_pkg.__path__ = [str(routes_dir)]
        sys.modules[pkg_name] = routes_pkg

    spec = importlib.util.spec_from_file_location(
        "src.web.routes.email_services",
        routes_dir / "email_services.py",
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules["src.web.routes.email_services"] = module
    spec.loader.exec_module(module)
    return module


class EmailServicesTempMailFormLogicTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root = Path(__file__).resolve().parents[1]
        cls.template_text = (root / "templates" / "email_services.html").read_text(encoding="utf-8")
        cls.js_text = (root / "static" / "js" / "email_services.js").read_text(encoding="utf-8")
        cls.routes_module = _load_email_services_module()

    def test_add_temp_mail_form_uses_auto_domain_hint_and_no_manual_domain_input(self):
        self.assertIn('id="add-tempmail-fields"', self.template_text)
        self.assertIn("创建时自动生成固定子域名", self.template_text)
        self.assertNotIn('id="custom-tm-domain"', self.template_text)

    def test_edit_temp_mail_form_keeps_domain_visible_but_read_only(self):
        self.assertIn('id="edit-tm-domain"', self.template_text)
        self.assertRegex(
            self.template_text,
            r'<input[^>]*id="edit-tm-domain"[^>]*readonly',
        )

    def test_js_temp_mail_create_payload_does_not_submit_domain(self):
        fn = re.search(
            r"async function handleAddCustom\(e\) \{(?P<body>[\s\S]*?)\n\}",
            self.js_text,
        )
        self.assertIsNotNone(fn)
        match = re.search(
            r"if \(subType === 'moemail'\) \{[\s\S]*?\} else \{(?P<block>[\s\S]*?)\n\s*\}\n\n\s*const data =",
            fn.group("body"),
        )
        self.assertIsNotNone(match)
        tempmail_create_block = match.group("block")
        self.assertNotIn("tm_domain", tempmail_create_block)
        self.assertNotIn("domain:", tempmail_create_block)

    def test_js_temp_mail_edit_payload_does_not_submit_domain(self):
        fn = re.search(
            r"async function handleEditCustom\(e\) \{(?P<body>[\s\S]*?)\n\}",
            self.js_text,
        )
        self.assertIsNotNone(fn)
        match = re.search(
            r"if \(subType === 'moemail'\) \{[\s\S]*?\} else \{(?P<block>[\s\S]*?)\n\s*\}\n\n\s*const updateData =",
            fn.group("body"),
        )
        self.assertIsNotNone(match)
        tempmail_edit_block = match.group("block")
        self.assertNotIn("tm_domain", tempmail_edit_block)
        self.assertNotIn("domain:", tempmail_edit_block)

    def test_cloudflare_settings_ui_and_endpoints_are_present(self):
        self.assertIn("Cloudflare Temp Mail 设置", self.template_text)
        for field_id in (
            "cf-api-token",
            "cf-account-id",
            "cf-zone-id",
            "cf-worker-name",
            "cf-domain-base",
            "cf-subdomain-mode",
            "cf-subdomain-length",
            "cf-subdomain-prefix",
            "cf-sync-enabled",
            "cf-require-sync",
            "save-cf-settings-btn",
        ):
            self.assertIn(f'id="{field_id}"', self.template_text)

        self.assertIn("api.get('/settings/temp-mail/cloudflare')", self.js_text)
        self.assertIn("api.post('/settings/temp-mail/cloudflare'", self.js_text)

    def test_cloudflare_settings_save_is_blocked_until_load_success(self):
        self.assertRegex(
            self.template_text,
            r'<button[^>]*id="save-cf-settings-btn"[^>]*disabled[^>]*>',
        )
        load_body = self.js_text.split("async function loadCloudflareSettings() {", 1)[1].split(
            "// 保存 Cloudflare Temp-Mail 设置", 1
        )[0]
        save_body = self.js_text.split("async function handleSaveCloudflareSettings(e) {", 1)[1].split(
            "// 更新批量按钮", 1
        )[0]

        self.assertIn("let cloudflareSettingsReady = false;", self.js_text)
        self.assertIn("cloudflareSettingsReady = true;", load_body)
        self.assertIn("elements.saveCfSettingsBtn.disabled = false;", load_body)
        self.assertIn("if (!cloudflareSettingsReady)", save_body)
        self.assertIn("await loadCloudflareSettings();", save_body)
        self.assertNotIn("elements.saveCfSettingsBtn.disabled = false;", save_body)
        self.assertNotRegex(
            save_body,
            r"finally\s*\{[\s\S]*elements\.saveCfSettingsBtn\.disabled\s*=\s*false",
        )

    def test_temp_mail_type_metadata_no_longer_marks_domain_as_required_user_input(self):
        payload = asyncio.run(self.routes_module.get_service_types())
        temp_mail_type = next(item for item in payload["types"] if item["value"] == "temp_mail")
        self.assertFalse(
            any(
                field.get("name") == "domain" and field.get("required") is True
                for field in temp_mail_type["config_fields"]
            )
        )


if __name__ == "__main__":
    unittest.main()
