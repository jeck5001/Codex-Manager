import test from "node:test";
import assert from "node:assert/strict";

import {
  APP_NAV_ITEMS,
  normalizeVisibleMenuItems,
  sanitizeVisibleMenuItems,
} from "../../lib/navigation.ts";

test("APP_NAV_ITEMS exposes the Hotmail page entry", () => {
  const item = APP_NAV_ITEMS.find((entry) => entry.id === "hotmail");
  assert.ok(item, "hotmail nav item should exist");
  assert.equal(item?.name, "Hotmail");
  assert.equal(item?.href, "/hotmail/");
});

test("normalizeVisibleMenuItems appends Hotmail for legacy full-menu settings", () => {
  const legacyItems = APP_NAV_ITEMS.filter((entry) => entry.id !== "hotmail").map(
    (entry) => entry.id
  );
  const normalized = normalizeVisibleMenuItems(legacyItems);
  assert.ok(normalized.includes("hotmail"));
});

test("sanitizeVisibleMenuItems keeps Hotmail hidden when the user turns it off", () => {
  const itemsWithoutHotmail = APP_NAV_ITEMS.filter((entry) => entry.id !== "hotmail").map(
    (entry) => entry.id
  );
  const normalized = sanitizeVisibleMenuItems(itemsWithoutHotmail);
  assert.ok(!normalized.includes("hotmail"));
});
