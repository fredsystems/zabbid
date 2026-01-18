# Phase 25E — Lifecycle-Aware Frontend Wiring

## Objective

Make the frontend **honest, predictable, and intention-revealing** with respect to the backend lifecycle, canonicalization, and No Bid semantics introduced in Phases 25A–25D.

This phase **does not add new domain behavior**.
It only **reflects existing backend rules in the UI** so actions no longer feel broken or arbitrary.

---

## In-Scope

- Expose bid year lifecycle state in the UI
- Make bootstrap blockers visible and understandable
- Surface No Bid semantics to admins
- Disable invalid actions post-canonicalization (without removing them)
- Add explanatory copy/tooltips for locked or restricted actions
- Zero override UI (read-only awareness only)

---

## Out-of-Scope

- Override execution UI (Phase 26+)
- Bid entry UI
- Round management
- Audit log browsing
- Role management changes
- Any new backend endpoints

---

## Core UX Principles

1. **Explain, don’t surprise**
2. **Disable, don’t error**
3. **Expose state before enforcing rules**
4. **Admins get clarity; public users get simplicity**

---

## Frontend Concepts Introduced

### 1. Lifecycle Awareness (Read-Only)

The frontend must understand and display:

```ts
type BidYearLifecycle =
  | "Draft"
  | "BootstrapComplete"
  | "Canonicalized"
  | "BiddingActive"
  | "BiddingClosed";
```

#### Display Rules

- Show lifecycle state badge at:
  - Bid year overview page
  - Bootstrap overview
  - Admin dashboard header

**Example:**

```text
Bid Year 2026 — Canonicalized 🔒
```

**Tooltip copy examples:**

- **Draft**
  “Bid year setup in progress. Structure can still change.”
- **BootstrapComplete**
  “Structure finalized. Ready for canonicalization.”
- **Canonicalized 🔒**
  “Bid structure locked. Changes require explicit overrides.”
- **BiddingActive**
  “Bidding in progress. Structural changes disabled.”
- **BiddingClosed**
  “Bidding complete. Read-only state.”

---

### 2. Bootstrap Blocker Visibility

#### Current Problem

Bootstrap fails, but the UI doesn’t explain _why_.

#### Solution

Use existing `bootstrap_completeness` data to show:

- Blocking reasons prominently
- Especially **users in No Bid**

#### UI Changes

##### Bootstrap Overview Page

Add a **Blocking Reasons Panel**:

- List:
  - “No active bid year”
  - “Expected area count not set”
  - “Area count mismatch”
  - **“Users remain in No Bid area”**

##### No Bid Blocker Display

```text
⚠️ Bootstrap Blocked

3 users are still assigned to "No Bid".
These users must be reviewed and assigned to an operational area.
```

- Button: **“View users needing review”**
- Links to filtered user list (No Bid only)

---

### 3. No Bid Area Semantics (Admin-Only)

#### Area List UI

For admins:

- Show “No Bid” area with:
  - `System Area` badge
  - Distinct styling (muted / warning)
- Disable:
  - Rename
  - Delete

Tooltip on disabled actions:

```text
System areas cannot be modified.
```

For non-admin/public views:

- Do **not** show No Bid at all

---

### 4. Lifecycle-Gated Actions (Disable, Don’t Remove)

#### Actions to Gate

| Action              | Allowed Before Canonicalized | Allowed After Canonicalized |
| ------------------- | ---------------------------- | --------------------------- |
| Create area         | ✅                           | ❌                          |
| Delete area         | ✅ (non-system only)         | ❌                          |
| Rename area         | ✅ (non-system only)         | ❌                          |
| Assign user to area | ✅                           | ❌ (override required)      |
| Delete user         | ✅ (moves to No Bid)         | ❌                          |

#### UI Behavior

- Buttons remain visible
- Disabled when invalid
- Tooltip explains **why**

Example tooltip:

```text
This action is disabled after canonicalization.
Use an override instead.
```

This preserves discoverability and intent.

---

### 5. User Detail View (Read-Only Awareness)

Add **informational indicators only**:

- If user has any overridden canonical fields:
  - Show badge: `Overridden`
  - Tooltip:

```text
One or more canonical fields have been overridden.
```

No edit UI yet.

---

### 6. Error Alignment (Safety Net)

Even with disabled UI:

- Backend errors should be surfaced clearly
- Translate canonical/lifecycle errors to:
  - “This bid year is locked”
  - “This action requires an override”
  - “Bootstrap is incomplete due to users in No Bid”

This ensures resilience if UI state desyncs.

---

## Technical Implementation Notes

### API Usage (Already Exists)

Frontend will rely on:

- `list_bid_years` → `lifecycle_state`
- `get_bootstrap_completeness`
- `list_areas` (includes `is_system_area`)
- `list_users_with_routing` (canonical-aware)

**No new endpoints required.**

---

## Testing Expectations

### Manual Validation Checklist

- [ ] Lifecycle badge updates correctly across transitions
- [ ] Bootstrap blocker messaging is accurate
- [ ] No Bid area visible to admins only
- [ ] No Bid blocks bootstrap completion visibly
- [ ] Actions disable correctly after canonicalization
- [ ] Tooltips explain restrictions clearly
- [ ] No UI crashes due to new enum values

### Automated (Optional for Phase 25E)

- Snapshot tests for lifecycle badge rendering
- Unit tests for action enable/disable logic

---

## Exit Criteria

Phase 25E is complete when:

1. ✅ Lifecycle state is visible in the UI
2. ✅ Bootstrap blockers are clearly explained
3. ✅ No Bid semantics are visible to admins
4. ✅ Invalid actions are disabled with explanation
5. ✅ Frontend no longer “feels broken” post-Phase 25
6. ✅ No backend changes required
7. ✅ No override UI implemented yet

---

## Why This Phase Matters

Phase 25 introduced **power and correctness**.

Phase 25E restores **trust and comprehension**.

After this phase:

- Admins understand _why_ they can’t do things
- The system feels intentional, not brittle
- You can safely layer override UI later without confusion
