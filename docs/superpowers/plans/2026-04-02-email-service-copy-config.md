# Email Service Copy Config Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a row-level "copy config" action on the email services page that copies the full service configuration JSON.

**Architecture:** Keep the UI change minimal by reusing the existing `readEmailServiceFull` mutation and shared clipboard utility. Extract JSON serialization into a tiny helper with a focused node test so the page only handles click orchestration and toast messaging.

**Tech Stack:** Next.js App Router, TypeScript, node:test, React Query, Sonner.

---

### Task 1: Add a serializable email-service copy payload helper

**Files:**
- Create: `apps/src/app/email-services/copy-config.ts`
- Test: `apps/src/app/email-services/copy-config.test.ts`

- [ ] **Step 1: Write the failing test**

- [ ] **Step 2: Run the test to verify it fails**

- [ ] **Step 3: Implement the minimal serializer**

- [ ] **Step 4: Run the test to verify it passes**

### Task 2: Wire the copy action into the email services page

**Files:**
- Modify: `apps/src/app/email-services/page.tsx`

- [ ] **Step 1: Add clipboard/helper imports**

- [ ] **Step 2: Add a `handleCopyConfig` action that reads full service data and copies JSON**

- [ ] **Step 3: Add `复制配置` to the row dropdown menu**

- [ ] **Step 4: Verify existing row actions still compile**

### Task 3: Verify frontend build

**Files:**
- Modify: `apps/src/app/email-services/page.tsx`
- Create: `apps/src/app/email-services/copy-config.ts`
- Test: `apps/src/app/email-services/copy-config.test.ts`

- [ ] **Step 1: Run the focused node test**

- [ ] **Step 2: Run desktop build verification**

