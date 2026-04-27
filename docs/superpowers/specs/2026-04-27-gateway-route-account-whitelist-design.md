# Gateway Route Account Whitelist Design

## Summary

Replace the current single-account `manual preferred account` routing hint with a multi-account route whitelist.

When the whitelist is set, only the selected accounts participate in gateway routing. Accounts outside the whitelist do not participate. Inside the filtered pool, the existing route strategies such as `ordered`, `balanced`, `weighted`, `least-latency`, and `cost-first` continue to work unchanged.

Clearing the whitelist restores the current default behavior: all otherwise eligible accounts can participate in routing.

## Goals

- Let the user choose multiple accounts that are allowed to participate in gateway routing.
- Make non-selected accounts completely ineligible for routing while the whitelist is active.
- Keep the existing route strategies and availability rules unchanged inside the filtered pool.
- Support a clear operation that restores default full-pool routing.
- Present the feature as a simple batch action in the accounts page instead of a per-row priority toggle.

## Non-Goals

- Do not introduce manual ordering inside the selected account set.
- Do not change the semantics of `ordered`, `balanced`, `weighted`, `least-latency`, or `cost-first`.
- Do not force unavailable, isolated, disabled, token-missing, or cooldown accounts into routing just because they were selected.
- Do not keep the old single-account priority UX as a first-class feature after this change.

## Product Shape

### Route Whitelist Semantics

Add a gateway state field named `manual_route_account_ids`.

Rules:

- `None` or an empty list means no whitelist is active.
- No whitelist means all normally eligible accounts continue to participate in routing.
- A non-empty list means only those account IDs may participate.
- Candidate filtering still applies after the whitelist gate, so unavailable accounts remain excluded.
- Clearing the whitelist is equivalent to restoring the default all-account candidate pool.

### User Experience

The accounts page becomes the primary control surface.

Behavior:

- users select one or more accounts with the existing table checkboxes
- a batch action `仅所选参与路由` writes the current selection as the whitelist
- a separate action `清空路由限制` removes the whitelist entirely
- whitelisted accounts show a compact `路由中` badge
- the page header shows either:
  - `全部可用账号参与路由`
  - `已限制为 N 个账号参与路由`

The old row action `设为优先 / 取消优先` is removed from the primary UX.

## Architecture

### 1. Gateway Routing State

Replace the in-memory single-value route hint:

- old: `manual_preferred_account_id: Option<String>`
- new: `manual_route_account_ids: Option<Vec<String>>`

The route state remains part of the existing gateway routing state container in `route_hint.rs`.

### 2. Candidate Filtering

Apply whitelist filtering before route strategy execution.

New high-level flow:

1. build the normal gateway candidate list
2. if a route whitelist exists and is non-empty, drop any candidate not in the whitelist
3. run the existing route strategy on the remaining candidates
4. continue with current new-account protection, health, cooldown, and retry behavior

This keeps the change isolated to candidate pool definition rather than route ordering.

### 3. Startup Snapshot and Dashboard State

Replace the startup snapshot field:

- old: `manualPreferredAccountId`
- new: `manualRouteAccountIds`

This keeps the dashboard and accounts page aligned with the same gateway state source.

## API Design

### RPC

Replace the manual single-account RPC family with a whitelist RPC family:

- `gateway/routeAccounts/get`
  - response: `{ "accountIds": string[] }`
- `gateway/routeAccounts/set`
  - request: `{ "accountIds": string[] }`
  - semantics: overwrite the full whitelist with the provided IDs
- `gateway/routeAccounts/clear`
  - response: success only
  - semantics: remove the whitelist and restore default routing

`gateway/routeStrategy/get` should also return `routeAccountIds` so the settings payload remains self-describing.

### Frontend Client Surface

Add:

- `getRouteAccountIds(): Promise<string[]>`
- `setRouteAccounts(accountIds: string[]): Promise<void>`
- `clearRouteAccounts(): Promise<void>`

Remove the primary use of:

- `getManualPreferredAccountId`
- `setManualPreferredAccount`
- `clearManualPreferredAccount`

## Migration and Compatibility

### Runtime Compatibility

To avoid silently dropping a user’s prior single-account override during upgrade:

- if legacy `manual_preferred_account_id` exists and the new whitelist is unset, expose it as a single-element whitelist
- once the new whitelist is written or cleared by the new UI, the legacy single-account value is no longer authoritative

This compatibility layer should stay narrow and temporary. The new UI only reads and writes the whitelist representation.

### UI Compatibility

The accounts page stops showing the old preferred-account pin action.

Any existing dashboard copy that implies one manually preferred account should be updated to reflect either:

- the whitelist count
- or the default all-account routing state

## Edge Cases

- Unknown account IDs in the whitelist are ignored during candidate filtering.
- Deleted accounts automatically disappear from effective routing because they no longer enter the candidate pool.
- If all whitelisted accounts are currently unavailable, the gateway behaves as if no eligible candidates exist and should surface the existing no-candidate error path.
- `set` with an empty list behaves the same as `clear`.
- Duplicate account IDs in input should be deduplicated before storing.
- Whitelist filtering must not bypass disabled, isolated, cooldown, token-missing, or unsupported-model checks.

## Testing Strategy

### Backend

- unit tests for whitelist read, write, clear, dedupe, and legacy single-account compatibility
- routing tests proving:
  - no whitelist leaves the candidate pool unchanged
  - a whitelist keeps only the selected accounts
  - unavailable whitelisted accounts are still excluded
  - an empty whitelist behaves like clear
  - route strategy still works on the filtered subset

### Frontend

- normalization tests for `manualRouteAccountIds`
- hook tests for account-page route whitelist state
- interaction tests or focused component tests proving:
  - batch selection can set the whitelist
  - clear removes the whitelist
  - whitelisted accounts render a `路由中` badge
  - summary text switches between unrestricted and restricted modes

### Regression Verification

- confirm `ordered` still follows account sort order inside the whitelist subset
- confirm `balanced` still rotates only across whitelisted candidates
- confirm no whitelist preserves the current routing behavior

## Rollout Notes

- This change is intentionally not persisted as a sorted route-order list; it is only a participation whitelist.
- The operational mental model becomes simpler: selected accounts may route, unselected accounts may not.
- Because the whitelist can intentionally shrink the pool to zero effective candidates, logs and UI status should make the restricted state visible enough for troubleshooting.
