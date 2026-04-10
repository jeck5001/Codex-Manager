import threading

from tools.hotmail_local_helper.hotmail_runner import (
    HotmailLocalTask,
    HotmailBackendCallbackClient,
    parse_hotmail_task,
    validate_hotmail_task_payload,
)


def test_validate_hotmail_task_payload_requires_profile():
    try:
        validate_hotmail_task_payload({"batch_id": "batch-1", "task_id": "task-1", "target_domains": ["hotmail.com"]})
    except ValueError as exc:
        assert "profile" in str(exc)
    else:
        raise AssertionError("expected ValueError")


def test_parse_hotmail_task_supports_snake_case_payload():
    task = parse_hotmail_task(
        {
            "batch_id": "batch-1",
            "task_id": "task-1",
            "profile": {"first_name": "Alice"},
            "target_domains": ["hotmail.com"],
            "verification_mailbox": {"email": "verify@example.com", "service_id": "svc-1"},
            "backend_callback_base": "http://192.168.5.35:9000/api/hotmail",
        }
    )

    assert task.batch_id == "batch-1"
    assert task.task_id == "task-1"
    assert task.verification_mailbox["email"] == "verify@example.com"


def test_backend_callback_client_posts_json(monkeypatch):
    observed = {}
    task = HotmailLocalTask(
        batch_id="batch-1",
        task_id="task-1",
        profile={"first_name": "Alice"},
        target_domains=["hotmail.com"],
        verification_mailbox={"email": "verify@example.com", "service_id": "svc-1"},
        backend_callback_base="http://192.168.5.35:9000/api/hotmail",
    )

    class FakeResponse:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return False

        def read(self):
            return b'{"ok": true}'

    def fake_urlopen(req, timeout=0):
        observed["url"] = req.full_url
        observed["timeout"] = timeout
        observed["body"] = req.data.decode("utf-8")
        return FakeResponse()

    monkeypatch.setattr("tools.hotmail_local_helper.hotmail_runner.urllib_request.urlopen", fake_urlopen)
    client = HotmailBackendCallbackClient(task)
    client.report_progress(status="running", current_step="opening_signup")

    assert observed["url"].endswith("/batches/batch-1/tasks/task-1/progress")
    assert '"current_step": "opening_signup"' in observed["body"]
