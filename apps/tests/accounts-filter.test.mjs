import test from "node:test";
import assert from "node:assert/strict";
import { filterAccounts } from "../src/views/accounts.js";

test("filterAccounts matches search and status", () => {
  const accounts = [{ id: "a", label: "alpha" }, { id: "b", label: "bravo" }];
  const usage = [{ accountId: "a", usedPercent: 90, secondaryUsedPercent: 10 }];
  const out = filterAccounts(accounts, usage, "alp", "low");
  assert.equal(out.length, 1);
  assert.equal(out[0].id, "a");
});
