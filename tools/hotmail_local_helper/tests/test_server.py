from fastapi.testclient import TestClient

from tools.hotmail_local_helper.server import create_app


def test_health_reports_ready_when_browser_check_passes(monkeypatch):
    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.check_playwright_ready",
        lambda: True,
    )
    client = TestClient(create_app())

    response = client.get("/health", headers={"Origin": "http://192.168.5.35:48761"})

    assert response.status_code == 200
    assert response.json()["ok"] is True
    assert response.json()["playwright_ready"] is True


def test_open_handoff_rejects_disallowed_origin():
    client = TestClient(create_app())

    response = client.post(
        "/open-handoff",
        headers={"Origin": "http://evil.example"},
        json={"handoff_id": "abc", "url": "https://signup.live.com"},
    )

    assert response.status_code == 403
    assert response.json()["error"] == "origin_not_allowed"


def test_open_handoff_invokes_launcher(monkeypatch, tmp_path):
    launched = {}

    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.check_playwright_ready",
        lambda: True,
    )

    def fake_launch(payload_path: str, profile_dir: str) -> None:
        launched["payload_path"] = payload_path
        launched["profile_dir"] = profile_dir

    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.launch_local_handoff_background",
        fake_launch,
    )
    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.HANDOFF_ROOT",
        tmp_path,
    )

    client = TestClient(create_app())
    response = client.post(
        "/open-handoff",
        headers={"Origin": "http://192.168.5.35:48761"},
        json={"handoff_id": "abc", "url": "https://signup.live.com"},
    )

    assert response.status_code == 200
    assert response.json()["ok"] is True
    assert launched["payload_path"].endswith("payload.json")
    assert launched["profile_dir"].endswith("profile")
