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


def test_start_task_accepts_local_first_hotmail_payload(monkeypatch):
    accepted = {}

    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.check_playwright_ready",
        lambda: True,
    )

    def fake_start(task, cancel_event=None, on_finish=None):
        accepted["task_id"] = task.task_id
        accepted["profile"] = task.profile
        accepted["cancel_event"] = cancel_event is not None
        accepted["has_on_finish"] = on_finish is not None

    monkeypatch.setattr(
        "tools.hotmail_local_helper.server.start_hotmail_task_background",
        fake_start,
    )
    client = TestClient(create_app())

    response = client.post(
        "/hotmail/start-task",
        headers={"Origin": "http://192.168.5.35:48761"},
        json={
            "batch_id": "batch-1",
            "task_id": "task-1",
            "profile": {
                "first_name": "Alice",
                "last_name": "Example",
                "birth_day": "8",
                "birth_month": "4",
                "birth_year": "1998",
                "password": "pw",
                "username_candidates": ["aliceexample"],
            },
            "target_domains": ["hotmail.com"],
            "verification_mailbox": {"email": "verify@example.com", "service_id": "svc-1"},
        },
    )

    assert response.status_code == 200
    assert response.json()["ok"] is True
    assert accepted["task_id"] == "task-1"
    assert accepted["has_on_finish"] is True
