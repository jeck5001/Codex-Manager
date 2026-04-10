from __future__ import annotations

import json
import threading
import time
from dataclasses import dataclass
from typing import Any, Optional
from urllib import error as urllib_error
from urllib import request as urllib_request

from src.services.hotmail.engine import HotmailRegistrationEngine
from src.services.hotmail.types import HotmailAccountArtifact, HotmailRegistrationProfile


@dataclass
class HotmailLocalTask:
    batch_id: str
    task_id: str
    profile: dict[str, Any]
    target_domains: list[str]
    proxy: str = ""
    verification_mailbox: Optional[dict[str, Any]] = None
    backend_callback_base: str = ""
    backend_callback_token: str = ""


class HotmailBackendCallbackClient:
    def __init__(self, task: HotmailLocalTask):
        self.task = task

    def _headers(self) -> dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self.task.backend_callback_token:
            headers["Authorization"] = f"Bearer {self.task.backend_callback_token}"
        return headers

    def _post(self, suffix: str, payload: dict[str, Any]) -> dict[str, Any]:
        if not self.task.backend_callback_base:
            return {}
        body = json.dumps(payload).encode("utf-8")
        req = urllib_request.Request(
            f"{self.task.backend_callback_base}/batches/{self.task.batch_id}/tasks/{self.task.task_id}/{suffix}",
            data=body,
            headers=self._headers(),
            method="POST",
        )
        with urllib_request.urlopen(req, timeout=300) as response:
            data = response.read().decode("utf-8")
        return json.loads(data or "{}")

    def report_progress(
        self,
        *,
        status: str,
        current_step: str,
        manual_action_required: bool = False,
        log_line: str = "",
        action_required_reason: str = "",
    ) -> dict[str, Any]:
        return self._post(
            "progress",
            {
                "status": status,
                "current_step": current_step,
                "manual_action_required": manual_action_required,
                "log_line": log_line,
                "action_required_reason": action_required_reason,
            },
        )

    def fetch_verification_code(self, *, timeout: int, poll_interval: int) -> str:
        response = self._post(
            "verification-code",
            {
                "timeout": timeout,
                "poll_interval": poll_interval,
            },
        )
        return str(response.get("code") or "").strip()

    def report_result(
        self,
        *,
        success: bool,
        artifact: Optional[HotmailAccountArtifact] = None,
        failure_code: str = "",
        failure_message: str = "",
    ) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "success": success,
            "failure_code": failure_code,
            "failure_message": failure_message,
            "artifact": None,
        }
        if artifact is not None:
            payload["artifact"] = {
                "email": artifact.email,
                "password": artifact.password,
                "target_domain": artifact.target_domain,
                "verification_email": artifact.verification_email,
                "first_name": artifact.first_name,
                "last_name": artifact.last_name,
            }
        return self._post("result", payload)


def parse_hotmail_task(payload: dict[str, Any]) -> HotmailLocalTask:
    return HotmailLocalTask(
        batch_id=str(payload.get("batch_id") or payload.get("batchId") or "").strip(),
        task_id=str(payload.get("task_id") or payload.get("taskId") or "").strip(),
        profile=dict(payload.get("profile") or {}),
        target_domains=list(payload.get("target_domains") or payload.get("targetDomains") or []),
        proxy=str(payload.get("proxy") or "").strip(),
        verification_mailbox=dict(payload.get("verification_mailbox") or payload.get("verificationMailbox") or {}),
        backend_callback_base=str(
            payload.get("backend_callback_base") or payload.get("backendCallbackBase") or ""
        ).strip(),
        backend_callback_token=str(
            payload.get("backend_callback_token") or payload.get("backendCallbackToken") or ""
        ).strip(),
    )


def validate_hotmail_task_payload(payload: dict[str, Any]) -> HotmailLocalTask:
    task = parse_hotmail_task(payload)
    if not task.batch_id or not task.task_id:
        raise ValueError("batch_id and task_id are required")
    if not task.profile:
        raise ValueError("profile is required")
    if not task.target_domains:
        raise ValueError("target_domains is required")
    if not task.verification_mailbox or not task.verification_mailbox.get("email"):
        raise ValueError("verification_mailbox.email is required")
    return task


def _profile_from_payload(task: HotmailLocalTask) -> HotmailRegistrationProfile:
    return HotmailRegistrationProfile(
        first_name=str(task.profile.get("first_name") or task.profile.get("firstName") or ""),
        last_name=str(task.profile.get("last_name") or task.profile.get("lastName") or ""),
        birth_day=str(task.profile.get("birth_day") or task.profile.get("birthDay") or ""),
        birth_month=str(task.profile.get("birth_month") or task.profile.get("birthMonth") or ""),
        birth_year=str(task.profile.get("birth_year") or task.profile.get("birthYear") or ""),
        password=str(task.profile.get("password") or ""),
        username_candidates=list(task.profile.get("username_candidates") or task.profile.get("usernameCandidates") or []),
        country=str(task.profile.get("country") or "United States"),
    )


def _reporter(callback_client: HotmailBackendCallbackClient):
    def inner(status: str, current_step: str, *, manual_action_required: bool = False, log_line: str = "") -> None:
        callback_client.report_progress(
            status=status,
            current_step=current_step,
            manual_action_required=manual_action_required,
            log_line=log_line,
            action_required_reason="unsupported_challenge" if manual_action_required else "",
        )

    return inner


def run_hotmail_local_task(task: HotmailLocalTask, cancel_event: Optional[threading.Event] = None) -> dict[str, Any]:
    callback_client = HotmailBackendCallbackClient(task)
    reporter = _reporter(callback_client)
    reporter("running", "opening_signup")
    profile = _profile_from_payload(task)
    engine = HotmailRegistrationEngine(proxy_url=task.proxy)

    def verification_code_provider(*, timeout: int, poll_interval: int) -> str:
        return callback_client.fetch_verification_code(timeout=timeout, poll_interval=poll_interval)

    result = engine.run_local_first(
        profile=profile,
        target_domains=task.target_domains,
        verification_email=str(task.verification_mailbox.get("email") or ""),
        verification_service_id=str(task.verification_mailbox.get("service_id") or ""),
        verification_code_provider=verification_code_provider,
        callback_reporter=reporter,
    )

    while result.handoff_context is not None and not (cancel_event and cancel_event.is_set()):
        reporter(
            "action_required",
            "manual_verification",
            manual_action_required=True,
            log_line=str(result.error_message or "waiting for local manual verification"),
        )
        time.sleep(5)
        result = engine.resume_handoff(result.handoff_context)

    if cancel_event and cancel_event.is_set():
        return callback_client.report_result(
            success=False,
            failure_code="cancelled",
            failure_message="Hotmail local-first task cancelled",
        )

    if result.success and result.artifact:
        return callback_client.report_result(success=True, artifact=result.artifact)

    return callback_client.report_result(
        success=False,
        failure_code=str(result.reason_code or "unexpected_exception"),
        failure_message=str(result.error_message or result.reason_code or "Hotmail local-first task failed"),
    )


def start_hotmail_task_background(
    task: HotmailLocalTask,
    cancel_event: Optional[threading.Event] = None,
    on_finish=None,
) -> threading.Thread:
    def runner():
        try:
            run_hotmail_local_task(task, cancel_event)
        finally:
            if callable(on_finish):
                on_finish(task)

    thread = threading.Thread(
        target=runner,
        daemon=True,
        name=f"hotmail-local-task-{task.task_id}",
    )
    thread.start()
    return thread
