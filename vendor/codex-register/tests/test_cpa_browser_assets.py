import json
import unittest
from pathlib import Path


REGISTER_ROOT = Path(__file__).resolve().parents[1]
ASSET_ROOT = REGISTER_ROOT / "src" / "browser_assets" / "openai_cpa_plugin"


class CPABrowserAssetsTests(unittest.TestCase):
    def test_manifest_and_required_assets_exist(self):
        manifest_path = ASSET_ROOT / "manifest.json"
        background_path = ASSET_ROOT / "background.js"
        bridge_path = ASSET_ROOT / "content" / "bridge.js"
        register_path = ASSET_ROOT / "content" / "register.js"
        utils_path = ASSET_ROOT / "content" / "utils.js"

        self.assertTrue(manifest_path.exists())
        self.assertTrue(background_path.exists())
        self.assertTrue(bridge_path.exists())
        self.assertTrue(register_path.exists())
        self.assertTrue(utils_path.exists())

    def test_manifest_references_vendored_content_scripts(self):
        manifest = json.loads((ASSET_ROOT / "manifest.json").read_text(encoding="utf-8"))
        content_scripts = manifest.get("content_scripts") or []
        listed_assets = {
            script
            for item in content_scripts
            for script in item.get("js", [])
        }

        self.assertIn("content/bridge.js", listed_assets)
        self.assertIn("content/utils.js", listed_assets)
        self.assertIn("content/register.js", listed_assets)
        self.assertEqual(manifest.get("background", {}).get("service_worker"), "background.js")


if __name__ == "__main__":
    unittest.main()
