import asyncio
import importlib.util
import json
import re
import subprocess
import sys
import types
import unittest
from pathlib import Path


def _load_email_services_module():
    src_dir = Path(__file__).resolve().parents[1] / "src"
    for module_name in list(sys.modules):
        if module_name == "src" or module_name.startswith("src."):
            sys.modules.pop(module_name, None)

    web_dir = src_dir / "web"
    routes_dir = src_dir / "web" / "routes"
    src_pkg = types.ModuleType("src")
    src_pkg.__path__ = [str(src_dir)]
    sys.modules["src"] = src_pkg

    web_pkg = types.ModuleType("src.web")
    web_pkg.__path__ = [str(web_dir)]
    sys.modules["src.web"] = web_pkg

    routes_pkg = types.ModuleType("src.web.routes")
    routes_pkg.__path__ = [str(routes_dir)]
    sys.modules["src.web.routes"] = routes_pkg

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

    def test_cloudflare_domain_health_table_and_reset_action_are_present(self):
        for field_id in (
            "cf-domain-configs-container",
            "cf-domain-configs-empty",
        ):
            self.assertIn(f'id="{field_id}"', self.template_text)

        self.assertIn("function renderCloudflareDomainConfigs", self.js_text)
        self.assertIn("/settings/temp-mail/cloudflare/domain-configs/${domainConfigId}/reset-stats", self.js_text)
        self.assertIn("重置统计", self.js_text)

    def _run_cloudflare_button_lifecycle_scenario(self, reload_result: str):
        self.assertIn(reload_result, {"success", "failure"})
        export_snippet = """
;globalThis.__testExports = {
  loadCloudflareSettings,
  handleSaveCloudflareSettings,
  elements,
  getCloudflareSettingsReady: () => cloudflareSettingsReady,
};
"""
        script = f"""
const vm = require('vm');

const source = {json.dumps(self.js_text + export_snippet)};
const reloadResult = {json.dumps(reload_result)};
const elementIds = [
  'outlook-count', 'custom-count', 'tempmail-status', 'total-enabled',
  'toggle-outlook-import', 'outlook-import-body', 'outlook-import-data', 'outlook-import-enabled',
  'outlook-import-priority', 'outlook-import-btn', 'clear-import-btn', 'import-result',
  'outlook-accounts-table', 'select-all-outlook', 'batch-delete-outlook-btn',
  'custom-services-table', 'add-custom-btn', 'select-all-custom',
  'tempmail-form', 'tempmail-api', 'tempmail-enabled', 'test-tempmail-btn',
  'cf-settings-form', 'cf-api-token', 'cf-api-token-hint', 'cf-account-id', 'cf-zone-id',
  'cf-worker-name', 'cf-domain-base', 'cf-subdomain-mode', 'cf-subdomain-length',
  'cf-subdomain-prefix', 'cf-sync-enabled', 'cf-require-sync', 'save-cf-settings-btn',
  'add-custom-modal', 'add-custom-form', 'close-custom-modal', 'cancel-add-custom',
  'custom-sub-type', 'add-moemail-fields', 'add-tempmail-fields',
  'edit-custom-modal', 'edit-custom-form', 'close-edit-custom-modal', 'cancel-edit-custom',
  'edit-moemail-fields', 'edit-tempmail-fields', 'edit-custom-type-badge',
  'edit-custom-sub-type-hidden', 'edit-tm-domain',
  'edit-outlook-modal', 'edit-outlook-form', 'close-edit-outlook-modal', 'cancel-edit-outlook',
];

function createElement(id) {{
  return {{
    id,
    value: '',
    checked: false,
    disabled: false,
    textContent: '',
    innerHTML: '',
    style: {{}},
    dataset: {{}},
    classList: {{
      add() {{}},
      remove() {{}},
      contains() {{ return false; }},
    }},
    addEventListener() {{}},
    querySelectorAll() {{ return []; }},
    querySelector() {{ return null; }},
    appendChild() {{}},
    removeChild() {{}},
    focus() {{}},
  }};
}}

const elementsById = new Map(elementIds.map((id) => [id, createElement(id)]));
elementsById.get('save-cf-settings-btn').textContent = '保存设置';
elementsById.get('cf-subdomain-mode').value = 'random';
elementsById.get('cf-subdomain-length').value = '6';

const document = {{
  getElementById(id) {{
    if (!elementsById.has(id)) {{
      elementsById.set(id, createElement(id));
    }}
    return elementsById.get(id);
  }},
  addEventListener() {{}},
  createElement() {{
    return createElement('generated');
  }},
}};

let getCallCount = 0;
const postPayloads = [];
const toastMessages = [];
let resolveReload;
let rejectReload;

const settingsPayload = {{
  has_api_token: true,
  cloudflare_account_id: 'acc-1',
  cloudflare_zone_id: 'zone-1',
  cloudflare_worker_name: 'temp-email',
  temp_mail_domain_base: 'mail.example.com',
  temp_mail_subdomain_mode: 'random',
  temp_mail_subdomain_length: 6,
  temp_mail_subdomain_prefix: 'tm',
  temp_mail_sync_cloudflare_enabled: true,
  temp_mail_require_cloudflare_sync: true,
}};

const api = {{
  async get(url) {{
    if (url !== '/settings/temp-mail/cloudflare') {{
      throw new Error(`Unexpected GET ${{url}}`);
    }}
    getCallCount += 1;
    if (getCallCount === 1) {{
      return settingsPayload;
    }}
    return new Promise((resolve, reject) => {{
      resolveReload = () => resolve(settingsPayload);
      rejectReload = () => reject(new Error('reload failed'));
    }});
  }},
  async post(url, payload) {{
    if (url !== '/settings/temp-mail/cloudflare') {{
      throw new Error(`Unexpected POST ${{url}}`);
    }}
    postPayloads.push(payload);
    return {{ success: true }};
  }},
}};

const toast = {{
  success(message) {{
    toastMessages.push({{ level: 'success', message }});
  }},
  error(message) {{
    toastMessages.push({{ level: 'error', message }});
  }},
}};

const context = {{
  api,
  console,
  document,
  format: {{ date: () => '' }},
  Map,
  parseInt,
  Promise,
  Set,
  toast,
}};
context.globalThis = context;

vm.createContext(context);
vm.runInContext(source, context);

const {{
  loadCloudflareSettings,
  handleSaveCloudflareSettings,
  elements,
  getCloudflareSettingsReady,
}} = context.__testExports;

(async () => {{
  await loadCloudflareSettings();
  const initialState = {{
    ready: getCloudflareSettingsReady(),
    disabled: elements.saveCfSettingsBtn.disabled,
    label: elements.saveCfSettingsBtn.textContent,
  }};

  const savePromise = handleSaveCloudflareSettings({{
    preventDefault() {{}},
  }});
  await Promise.resolve();
  await Promise.resolve();
  if (!resolveReload && !rejectReload) {{
    await new Promise((resolve) => setTimeout(resolve, 0));
  }}
  if (!resolveReload || !rejectReload) {{
    throw new Error('reload promise was not created');
  }}

  const duringReload = {{
    ready: getCloudflareSettingsReady(),
    disabled: elements.saveCfSettingsBtn.disabled,
    label: elements.saveCfSettingsBtn.textContent,
    getCallCount,
    postCallCount: postPayloads.length,
  }};

  if (reloadResult === 'success') {{
    resolveReload();
  }} else {{
    rejectReload();
  }}

  await savePromise;

  const finalState = {{
    ready: getCloudflareSettingsReady(),
    disabled: elements.saveCfSettingsBtn.disabled,
    label: elements.saveCfSettingsBtn.textContent,
    hint: elements.cfApiTokenHint.textContent,
    getCallCount,
    postCallCount: postPayloads.length,
    toastMessages,
  }};

  process.stdout.write(JSON.stringify({{
    initialState,
    duringReload,
    finalState,
  }}));
}})().catch((error) => {{
  process.stderr.write(String(error.stack || error));
  process.exit(1);
}});
"""
        completed = subprocess.run(
            ["node", "-e", script],
            check=True,
            capture_output=True,
            text=True,
        )
        return json.loads(completed.stdout)

    def test_cloudflare_settings_save_is_blocked_until_reload_completes(self):
        self.assertRegex(
            self.template_text,
            r'<button[^>]*id="save-cf-settings-btn"[^>]*disabled[^>]*>',
        )
        states = self._run_cloudflare_button_lifecycle_scenario("success")
        self.assertTrue(states["initialState"]["ready"])
        self.assertFalse(states["initialState"]["disabled"])
        self.assertEqual(states["duringReload"]["postCallCount"], 1)
        self.assertEqual(states["duringReload"]["getCallCount"], 2)
        self.assertTrue(states["duringReload"]["disabled"])
        self.assertEqual(states["duringReload"]["label"], "保存中...")
        self.assertTrue(states["finalState"]["ready"])
        self.assertFalse(states["finalState"]["disabled"])
        self.assertEqual(states["finalState"]["label"], "保存设置")

    def test_cloudflare_settings_save_stays_blocked_when_reload_fails(self):
        states = self._run_cloudflare_button_lifecycle_scenario("failure")
        self.assertTrue(states["duringReload"]["disabled"])
        self.assertEqual(states["duringReload"]["label"], "保存中...")
        self.assertFalse(states["finalState"]["ready"])
        self.assertTrue(states["finalState"]["disabled"])
        self.assertEqual(states["finalState"]["label"], "保存设置")
        self.assertEqual(states["finalState"]["hint"], "加载失败，请稍后重试")

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
