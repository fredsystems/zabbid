# Phase 30D — Bootstrap UI Restructure

## Purpose

Replace the monolithic `BootstrapCompleteness.tsx` component (currently ~1900 lines)
with a **structured, task-oriented bootstrap workflow** that guides operators
through pre-bid configuration in a logical sequence.

This sub-phase delivers the **core UX improvement** required for Phase 30:
making bootstrap operable and understandable with 200+ users and complex
configuration requirements.

---

## Scope

### A. Current State Analysis

The existing `BootstrapCompleteness.tsx` component contains:

- Bid year management (create, edit, set active, expected areas)
- Area management (create, edit, expected users)
- User management (create, edit, delete, CSV import)
- Inline rendering of blocking reasons
- All functionality on a single, scrolling page

**Problems with current design:**

- Cognitive overload with 200+ users
- No clear task progression
- Difficult to navigate
- Mobile-unfriendly due to density
- Mixes completed and incomplete tasks
- No workflow guidance

---

### B. Target Structure

Replace the monolithic page with a **multi-section workflow**.

Implementation options:

#### Option 1: Route-based sections (Recommended)

Each workflow step is a separate route with dedicated component.

Example routes:

- `/admin/bootstrap/bid-years`
- `/admin/bootstrap/areas`
- `/admin/bootstrap/users`
- `/admin/bootstrap/no-bid-review`
- `/admin/bootstrap/round-groups`
- `/admin/bootstrap/area-round-groups`
- `/admin/bootstrap/schedule`
- `/admin/bootstrap/readiness`

**Rationale:**

- Deep-linkable
- Browser back/forward works intuitively
- Easier to reason about state boundaries
- Better mobile support (one task at a time)
- Clearer separation of concerns

---

### C. Workflow Sections

Each section is implemented as a dedicated component and route.

#### 1. Bid Year Setup (`BidYearSetup.tsx`)

**Purpose:** Configure bid year metadata and activate.

**Functionality:**

- List all bid years
- Create new bid year
- Set active bid year
- Edit bid year metadata (label, notes)
- Set expected area count (**non-system areas only**)
- Display lifecycle state

**Navigation:**

- Previous: None (entry point)
- Next: Area Setup

**Completion criteria:**

- Exactly one bid year is active
- Expected non-system area count is set

---

#### 2. Area Setup (`AreaSetup.tsx`)

**Purpose:** Configure all operational areas.

**Functionality:**

- List all areas for active bid year
- Create new areas
- Edit area names
- Set expected user counts (**non-system areas only**)
- Show area count vs expected (non-system only)
- Display system area (No Bid) distinctly

**Navigation:**

- Previous: Bid Year Setup
- Next: User Management

**Completion criteria:**

- Actual non-system area count matches expected
- All non-system areas have expected user counts set
- System areas do not block readiness

---

#### 3. User Management (`UserManagement.tsx`)

**Purpose:** Populate and configure user roster.

**Functionality:**

- CSV import (with preview)
- Manual user creation
- User editing (metadata, area assignment, participation flags)
- User deletion
- Bulk filtering by area
- Show user count per area vs expected

**Navigation:**

- Previous: Area Setup
- Next: No Bid Review

**Completion criteria:**

- All non-system areas have actual user count matching expected
- Any user-level validation warnings are visible and actionable

---

#### 4. No Bid Review (`NoBidReview.tsx`)

**Purpose:** Resolve users in the No Bid system area.

**Functionality:**

- Integrates existing No Bid review UI from Phase 29
- List users in No Bid
- Reassign to competitive areas
- Explicitly confirm user remains in No Bid

**Navigation:**

- Previous: User Management
- Next: Round Groups Setup

**Completion criteria:**

- Zero users in No Bid, **OR**
- All No Bid users explicitly reviewed

If there are zero users in No Bid, this step must automatically be considered complete.

---

#### 5. Round Groups Setup (`RoundGroupSetup.tsx`)

**Purpose:** Configure round groups and rounds.

**Functionality:**

- Delegates to components implemented in Phase 30B
- List round groups
- Create/edit/delete round groups
- Navigate to rounds management per group
- Show round count per group

This component acts as a **wrapper page** around the Phase 30B round group
management components, not a duplicate implementation.

**Navigation:**

- Previous: No Bid Review
- Next: Area → Round Group Assignment

**Completion criteria:**

- At least one round group exists
- Each round group has at least one round defined

---

#### 6. Area → Round Group Assignment (`AreaRoundGroupAssignment.tsx`)

**Purpose:** Assign exactly one round group to each non-system area.

**Functionality:**

- Delegates to component implemented in Phase 30C
- List all non-system areas
- Assign round group to each area
- Show assignment status and readiness impact

**Navigation:**

- Previous: Round Groups Setup
- Next: Bid Schedule

**Completion criteria:**

- Every non-system area has exactly one round group assigned

---

#### 7. Bid Schedule Setup (`BidScheduleSetup.tsx`)

**Purpose:** Declare bid timing and window.

**Functionality:**

- Set bid timezone (IANA selector)
- Set bid start date (date picker; must be Monday and future at confirmation)
- Set daily bid window (wall-clock start/end times)
- Set bidders per area per day
- Display schedule summary
- Edit schedule (pre-Canonicalized only)

**Navigation:**

- Previous: Area → Round Group Assignment
- Next: Readiness Review

**Completion criteria:**

- All schedule fields are set
- Start date is valid

---

#### 8. Readiness Review (`ReadinessReview.tsx`)

**Purpose:** Review all blockers and confirm readiness.

**Functionality:**

- Display computed readiness state (backend-derived only)
- List all blocking reasons
- Link blockers to relevant workflow sections
- Display lifecycle state badge
- Confirm Ready to Bid button (irreversible)
- Confirmation modal summarizing frozen inputs

**Navigation:**

- Previous: Bid Schedule
- Next: None (terminal state)

**Completion criteria:**

- No blockers remain
- Operator confirms Ready to Bid
- System transitions to Canonicalized

---

### D. Shared Navigation Component

Create `ui/src/components/BootstrapNavigation.tsx`.

**Functionality:**

- Display all workflow steps in order
- Highlight current step
- Show completion status per step
- Allow navigation to any step (navigation is never hard-blocked)
- Mobile: collapsible or bottom navigation
- Desktop: persistent sidebar

Navigation must never block movement; only confirmation is gated.

---

### E. Shared Readiness Widget

Create `ui/src/components/ReadinessWidget.tsx`.

**Functionality:**

- Display lifecycle state badge
- Display readiness state and blocker count
- Link to Readiness Review
- Visible on all bootstrap routes

The widget must rely exclusively on backend readiness evaluation
as the single source of truth.

---

### F. Routing Integration

Update `ui/src/App.tsx`:

- Replace `/admin/bootstrap` with:
  - `/admin/bootstrap/bid-years`
  - `/admin/bootstrap/areas`
  - `/admin/bootstrap/users`
  - `/admin/bootstrap/no-bid-review`
  - `/admin/bootstrap/round-groups`
  - `/admin/bootstrap/area-round-groups`
  - `/admin/bootstrap/schedule`
  - `/admin/bootstrap/readiness`

Default `/admin/bootstrap` redirects to `/admin/bootstrap/bid-years`.

Each route renders:

- BootstrapNavigation (persistent)
- ReadinessWidget (persistent)
- Section content

Update main navigation to link to `/admin/bootstrap/bid-years`.

---

### G. Code Migration Strategy

Extract existing logic from `BootstrapCompleteness.tsx` into section components.

**Rules:**

- Preserve all existing behavior
- No API changes
- No domain logic changes
- No validation changes
- No simplification for convenience

Delete `BootstrapCompleteness.tsx` only after full migration.

---

### H. Styling

Create SCSS modules per section:

- `bootstrap-navigation.module.scss`
- `readiness-widget.module.scss`
- `bid-year-setup.module.scss`
- `area-setup.module.scss`
- `user-management.module.scss`
- `bid-schedule-setup.module.scss`
- `readiness-review.module.scss`

Follow AGENTS.md styling rules:

- Mobile-first
- Card-based layouts
- No inline styles
- Clear lifecycle indicators

---

## Explicit Non-Goals

- No backend changes
- No domain changes
- No lifecycle changes
- No validation changes
- No redesign of existing visual language

---

## Completion Conditions

Phase 30D is complete when:

- Bootstrap workflow is fully segmented
- Navigation and readiness widgets function correctly
- All existing bootstrap functionality preserved
- Old monolithic component removed
- Lifecycle locks respected
- Mobile usability verified
- Manual validation passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Existing behavior cannot be preserved
- API contracts must change
- Domain invariants are violated
- Lifecycle semantics become ambiguous

---

## Risk Notes

- Large refactor with regression risk
- Must preserve semantics exactly
- Coordinate carefully with 30B and 30C
- Consider splitting execution if scope becomes unmanageable
