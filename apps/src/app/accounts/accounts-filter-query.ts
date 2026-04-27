export interface AccountsFilterQueryState {
  search: string;
  groupFilter: string;
  statusFilter: string;
  governanceFilter: string;
  statusReasonFilter: string;
  cooldownReasonFilter: string;
  tagFilter: string;
}

export function buildAccountsFilterUrl(
  state: AccountsFilterQueryState,
  pathname = "/accounts",
): string {
  const params = new URLSearchParams();
  const search = String(state.search || "").trim();
  const groupFilter = String(state.groupFilter || "").trim();
  const statusFilter = String(state.statusFilter || "").trim();
  const governanceFilter = String(state.governanceFilter || "").trim();
  const statusReasonFilter = String(state.statusReasonFilter || "").trim();
  const cooldownReasonFilter = String(state.cooldownReasonFilter || "").trim();
  const tagFilter = String(state.tagFilter || "").trim();

  if (statusFilter && statusFilter !== "all") {
    params.set("status", statusFilter);
  }
  if (governanceFilter && governanceFilter !== "all") {
    params.set("governanceReason", governanceFilter);
  }
  if (statusReasonFilter && statusReasonFilter !== "all") {
    params.set("statusReason", statusReasonFilter);
  }
  if (cooldownReasonFilter && cooldownReasonFilter !== "all") {
    params.set("cooldownReason", cooldownReasonFilter);
  }
  if (search) {
    params.set("query", search);
  }
  if (groupFilter && groupFilter !== "all") {
    params.set("group", groupFilter);
  }
  if (tagFilter && tagFilter !== "all") {
    params.set("tag", tagFilter);
  }

  const query = params.toString();
  return query ? `${pathname}?${query}` : pathname;
}
