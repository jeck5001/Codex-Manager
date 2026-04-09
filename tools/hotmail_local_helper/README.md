# Hotmail Local Helper

Small localhost helper for the Docker/web Hotmail handoff flow.

## Quick Start

```bash
cd tools/hotmail_local_helper
python3 -m venv .venv
source .venv/bin/activate
pip install -r ../../vendor/codex-register/requirements.txt
playwright install chromium
cd ../..
python3 -m tools.hotmail_local_helper
```

Then open `http://127.0.0.1:16788/health` on the same machine that will visit Codex Manager.
