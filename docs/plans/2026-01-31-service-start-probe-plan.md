# Service Start Background Probe Plan

Goal: Avoid the false "connection failed" status right after starting service by probing in the background, while keeping the UI responsive.

## Tasks
1) Add a failing test for `waitForConnection` in `apps/src/services/connection.test.js`.
2) Implement `waitForConnection` and `skipInitialize` option in `apps/src/services/connection.js`.
3) Update UI flow to use background probe:
   - Add `serviceProbeId` to `apps/src/state.js`.
   - Update `apps/src/main.js` to call `startService(..., { skipInitialize: true })` and then probe via `waitForConnection`.
4) Re-run tests and confirm pass.

## Verification
- `node --test apps\src\services\connection.test.js`

## Rollback
- Revert the test and the new connection/probe logic in `apps/src/services/connection.js`, `apps/src/state.js`, `apps/src/main.js`.
