# Hotmail Web Local Helper Design

## Goal

Add a web-compatible "local handoff" path for Hotmail registration so operators using the Docker/web deployment can launch the manual takeover browser on the machine currently visiting Codex Manager, instead of on the Docker host.

## Relationship To Existing Design

This design extends the desktop-only handoff described in `2026-04-09-hotmail-local-handoff-design.md`.

That earlier design introduced the shared `local_handoff` payload and a Tauri-only launcher. This document adds a second launcher path for plain web runtimes. The backend payload stays the same. The new work is how a browser session served from Docker asks the operator's own machine to open the handoff locally.

## Problem

The current desktop local handoff only works when the frontend is running inside Tauri. In the user's actual deployment, the management UI is served from Docker and opened from a browser at an address like `http://192.168.5.35:48761`. In that environment, the page can fetch batch state from the backend, but it cannot directly launch a browser process on the operator's machine.

Without an additional client-side bridge, the only workable fallback is still remote VNC/noVNC into the register container. That path remains too unreliable for Microsoft's human verification flow, especially the long-press interaction.

## Recommended Approach

Introduce a tiny localhost helper service that runs on the operator's machine and exposes a minimal HTTP API for Hotmail handoff launches.

The Docker-hosted web UI detects this helper on `127.0.0.1` and sends the existing `local_handoff` payload to it. The helper then launches the same dedicated local Chromium handoff flow already designed for desktop use. This keeps the operational model simple:

1. The register backend exports `local_handoff`
2. The web UI displays `本机接管（Web）`
3. The browser posts the payload to the local helper on the current machine
4. The helper opens a dedicated local browser profile for manual verification
5. The operator returns to the management page and clicks `继续注册`

## Alternatives Considered

### 1. Run the browser on the Docker host

This is not recommended. It does not solve the real usability problem because the operator would still need VNC or noVNC to interact with the challenge.

### 2. Use a custom URI scheme

This could launch a local app from the browser, but it adds platform-specific registration, upgrade friction, and more opaque failure modes. It is harder to debug than a localhost HTTP service and provides less straightforward health checking.

### 3. Use a browser extension

This can bridge web pages to local machine capabilities, but it adds store packaging, extension permission management, and browser-specific maintenance. That is too heavy for the first version.

### 4. Localhost helper service

This is the recommended approach. It is easy to probe from the web UI, easy to explain to the user, and lets the implementation reuse the existing Python/Playwright handoff launcher with minimal duplication.

## Architecture

The backend remains the source of truth for Hotmail batch lifecycle and handoff state. No new backend transport is needed beyond the existing `local_handoff` payload already attached to paused batches.

The new component is a helper process running on the operator's own machine:

- bind address: `127.0.0.1`
- default port: `16788`
- runtime: Python
- browser engine: Playwright Chromium

The helper accepts a POST from the management page, materializes the handoff payload into a temporary working directory, and invokes the existing local handoff launcher logic. The browser instance it launches must use a dedicated profile directory and must not reuse the operator's normal browser profile.

The flow is intentionally one-way:

- the backend exports session state
- the helper consumes it and launches a local browser
- the user manually interacts with Microsoft
- the management page later triggers `continue`

The helper does not sync cookies or browser state back into the remote register process. It is a local manual assist, not a bidirectional session bridge.

## Helper API

The helper exposes two endpoints.

### `GET /health`

Used by the web UI to detect whether the helper is reachable and whether Playwright is ready.

Example response:

```json
{
  "ok": true,
  "service": "hotmail-local-helper",
  "version": "1",
  "playwright_ready": true
}
```

### `POST /open-handoff`

Consumes the existing `local_handoff` JSON object from the batch snapshot.

Example success response:

```json
{
  "ok": true,
  "handoff_id": "abc123",
  "profile_dir": "/tmp/codex-hotmail-handoff/abc123",
  "message": "browser launched"
}
```

Example failure response:

```json
{
  "ok": false,
  "error": "playwright_browser_missing",
  "message": "Chromium is not installed for the local helper"
}
```

The helper should reject malformed payloads early and return stable error codes for expected setup issues such as:

- `invalid_payload`
- `playwright_browser_missing`
- `browser_launch_failed`
- `origin_not_allowed`

## Security And Network Boundaries

The helper is intentionally narrow and local-only.

- listen only on `127.0.0.1`
- accept only JSON requests
- reject requests without an allowed `Origin`
- do not bind to `0.0.0.0`
- do not provide any endpoint other than health and launch
- do not expose generic shell execution or filesystem browsing

Allowed origins for the first version should include:

- the exact browser origin currently hosting the management page
- `http://127.0.0.1:*`
- `http://localhost:*`

The helper may optionally support an environment variable to override allowed origins for advanced deployments, but that is not required for the first version.

## Web Frontend Changes

The Hotmail page should expose two local handoff modes depending on runtime:

- Tauri runtime: keep `本地接管（推荐）`
- plain web runtime with `local_handoff` available: show `本机接管（Web）`

When the user clicks the web button, the frontend should:

1. Probe `GET http://127.0.0.1:16788/health`
2. If the helper is not available, show a clear setup dialog or inline instruction block
3. If the helper is healthy, POST the `local_handoff` payload to `/open-handoff`
4. Show a success toast telling the operator to finish the Microsoft step in the new browser window

The frontend must not silently fail. If the helper is missing, the UI should provide exact guidance, including:

- helper address
- expected health endpoint
- startup command
- reminder that the helper must run on the same machine as the browser currently visiting the page

Remote handoff remains visible as a fallback.

## Helper Packaging And Startup Model

The first version should ship as a repository-managed helper script, not as part of the Docker image and not as an OS installer.

Recommended layout:

- `vendor/codex-register/src/services/hotmail/local_handoff_cli.py`
  - existing launcher logic
- `tools/hotmail-local-helper/`
  - new helper entrypoint
  - minimal README or usage notes
  - optional bootstrap script for creating a virtualenv and installing Playwright

This choice keeps the feature operationally simple:

- Docker images stay focused on server-side services
- the helper runs where the operator's browser actually exists
- the same repository owns both the backend payload contract and the local launcher behavior

The initial user workflow should be explicit:

1. On the operator machine, install the helper dependencies once
2. Start the helper locally
3. Open the Docker-hosted Codex Manager page
4. Use `本机接管（Web）` whenever a Hotmail batch pauses for manual verification

Auto-start at login, tray integration, native packaging, and system service registration are out of scope for the first version.

## Error Handling

Expected failure handling should be specific and user-facing.

- If the helper is unreachable, the page should say the helper is not running locally
- If the helper is reachable but Playwright is not installed, the page should surface a setup action and command
- If the helper launches the browser but the user closes it without solving anything, the batch remains `action_required`
- If Microsoft invalidates the migrated context after local launch, the follow-up `continue` call should surface that the session could not proceed instead of pretending the handoff succeeded

The helper should write short structured logs for launch attempts so local troubleshooting is possible without opening the Docker containers.

## Testing

### Backend

No new backend transport behavior is required beyond preserving the existing `local_handoff` payload. Regression tests should confirm the payload remains present for supported paused batches.

### Helper

Add focused tests for:

- health endpoint response
- malformed payload rejection
- origin validation
- successful handoff launch invocation with the expected payload path and profile directory

The launch path should be unit-tested by mocking the actual browser invocation, not by opening a real browser in CI.

### Frontend

Add tests for:

- web runtime handoff button visibility
- helper health check success/failure behavior
- setup guidance rendering when the helper is unavailable
- POST payload wiring for `/open-handoff`

Run `pnpm run build:desktop` to ensure the new web-runtime logic does not regress the desktop bundle.

### Manual Verification

1. Start the helper on a local machine
2. Open the Docker-hosted Codex Manager page from that machine
3. Pause a Hotmail batch on a manual challenge
4. Click `本机接管（Web）`
5. Confirm a dedicated local Chromium window opens on the same machine
6. Return to the management page and click `继续注册`
7. Stop the helper and confirm the page shows actionable setup guidance instead of a generic error

## Scope Boundaries

Included in this design:

- a localhost helper for web deployments
- web UI detection and launch flow
- explicit setup and failure guidance

Out of scope:

- automatic challenge solving
- bypassing or emulating Microsoft's human verification
- reusing the operator's default browser profile
- installing the helper inside the Docker container
- native installers or tray apps
- bidirectional session synchronization after local launch

## Success Criteria

The feature is successful if:

1. A Docker-hosted Codex Manager page can trigger local Hotmail handoff on the operator's own machine
2. The web UI clearly distinguishes helper-not-running from launch/runtime failures
3. The helper reuses the existing dedicated-browser handoff model instead of inventing a second browser workflow
4. Remote handoff remains available as a fallback
