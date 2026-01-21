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

### B. Target Structure

Replace the monolithic page with a **multi-section workflow**.

Implementation options:

#### Option 1: Route-based sections

Each workflow step is a separate route with dedicated component.

Example routes:

- `/admin/bootstrap/bid-years`
- `/admin/bootstrap/areas`
- `/admin/bootstrap/users`
- `/admin/bootstrap/round-groups`
- `/admin/bootstrap/schedule`
- `/admin/bootstrap/readiness`

#### Option 2: Tab-based sections

Single route with tab-based navigation, each tab lazy-loads section component.

#### Option 3: Accordion/stepper

Single route with progressive disclosure, guided step-by-step flow.

#### Recommended: Option 1 (route-based)

Rationale:

- Deep-linkable
- Browser back/forward works intuitively
- Easier to reason about state boundaries
- Better mobile support (one task at a time)
- Clearer separation of concerns

### C. Workflow Sections

Define the following sections (each a separate component):

#### 1. Bid Year Setup (`BidYearSetup.tsx`)

**Purpose:** Configure bid year metadata and activate.

**Functionality:**

- List all bid years
- Create new bid year
- Set active bid year
- Edit bid year metadata (label, notes)
- Set expected area count
- Display lifecycle state

**Navigation:**

- Previous: None (entry point)
- Next: Area Setup

**Completion criteria:**

- Exactly one bid year is active
- Expected area count is set

#### 2. Area Setup (`AreaSetup.tsx`)

**Purpose:** Configure all operational areas.

**Functionality:**

- List all areas for active bid year
- Create new areas
- Edit area names
- Set expected user counts (per area)
- Show area count vs expected
- Display system area (No Bid)

**Navigation:**

- Previous: Bid Year Setup
- Next: User Import/Management

**Completion criteria:**

- Actual area count matches expected
- All non-system areas have expected user counts set

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
- No duplicate initials within bid year

#### 4. No Bid Review (`NoBidReview.tsx`)

**Purpose:** Resolve users in the No Bid system area.

**Functionality:**

- Already exists from Phase 26D
- Minor integration updates may be needed
- List users in No Bid
- Reassign to competitive areas
- Confirm to remain in No Bid (if that API exists)

**Navigation:**

- Previous: User Management
- Next: Round Groups Setup

**Completion criteria:**

- Zero users in No Bid, OR
- All No Bid users explicitly reviewed

#### 5. Round Groups Setup (`RoundGroupSetup.tsx`)

**Purpose:** Configure round groups and rounds.

**Functionality:**

- Delegate to components from 30B
- List round groups
- Create/edit/delete round groups
- Navigate to rounds management per group
- Show round count per group

**Navigation:**

- Previous: No Bid Review
- Next: Area Round Assignment

**Completion criteria:**

- At least one round group exists
- Each round group has at least one round

#### 6. Area Round Assignment (`AreaRoundAssignment.tsx`)

**Purpose:** Assign round groups to areas.

**Functionality:**

- Delegate to component from 30C
- List all non-system areas
- Assign round group to each area
- Show assignment status

**Navigation:**

- Previous: Round Groups Setup
- Next: Bid Schedule

**Completion criteria:**

- Every non-system area has exactly one round group assigned

#### 7. Bid Schedule (`BidScheduleSetup.tsx`)

**Purpose:** Declare bid timing and window.

**Functionality:**

- Set bid timezone (IANA selector)
- Set bid start date (date picker, must be Monday, must be future)
- Set daily bid window (wall-clock start/end times)
- Set bidders per area per day
- Display schedule summary
- Edit schedule (pre-Canonicalized only)

**Navigation:**

- Previous: Area Round Assignment
- Next: Readiness Review

**Completion criteria:**

- All schedule fields are set
- Start date is valid (Monday, future at confirmation time)

#### 8. Readiness Review (`ReadinessReview.tsx`)

**Purpose:** Review all blockers and confirm ready to bid.

**Functionality:**

- Display computed readiness state
- List all blocking reasons (if any)
- Link to relevant sections to resolve blockers
- Show "Ready to Bid" status badge
- Confirm Ready to Bid button (irreversible)
- Confirmation modal with summary and acknowledgment

**Navigation:**

- Previous: Bid Schedule
- Next: None (terminal state after confirmation)

**Completion criteria:**

- No blockers remain
- User has confirmed ready to bid
- System transitions to Canonicalized

### D. Shared Navigation Component

Create `ui/src/components/BootstrapNavigation.tsx`:

**Purpose:** Persistent navigation sidebar or header showing workflow steps.

**Functionality:**

- Display all workflow sections in order
- Highlight current section
- Show completion status per section (✅ complete, ⚠️ incomplete, ❌ blocked)
- Enable jumping to any section
- Mobile: collapsible menu or bottom navigation
- Desktop: persistent sidebar

**Visual design:**

- Stepper pattern (numbered steps)
- Color-coded status indicators
- Clear labels
- Touch-friendly on mobile

### E. Shared Readiness Widget

Create `ui/src/components/ReadinessWidget.tsx`:

**Purpose:** Persistent summary of overall readiness state.

**Functionality:**

- Display current lifecycle state badge
- Show readiness status (ready / not ready / blocked)
- Show count of remaining blockers
- Link to Readiness Review section
- Visible on all bootstrap sections

**Placement:**

- Top of page (below header)
- Sticky on desktop
- Collapsible on mobile

### F. Routing Integration

Update `ui/src/App.tsx`:

- Replace single `/admin/bootstrap` route with:
  - `/admin/bootstrap/bid-years`
  - `/admin/bootstrap/areas`
  - `/admin/bootstrap/users`
  - `/admin/bootstrap/no-bid-review`
  - `/admin/bootstrap/round-groups`
  - `/admin/bootstrap/area-rounds`
  - `/admin/bootstrap/schedule`
  - `/admin/bootstrap/readiness`

- Default `/admin/bootstrap` → redirect to `/admin/bootstrap/bid-years`

- Each route renders:
  - BootstrapNavigation (persistent)
  - ReadinessWidget (persistent)
  - Section component (content)

Update `ui/src/components/Navigation.tsx`:

- Change "Bootstrap" link to point to `/admin/bootstrap/bid-years`

### G. Code Migration Strategy

#### Extract existing logic from BootstrapCompleteness.tsx

1. Identify reusable sub-components:
   - `BidYearItem` → move to `BidYearSetup.tsx`
   - `CreateBidYearForm` → move to `BidYearSetup.tsx`
   - `AreaItem` → move to `AreaSetup.tsx`
   - `CreateAreaForm` → move to `AreaSetup.tsx`
   - `UserItem` → move to `UserManagement.tsx`
   - `CreateUserForm` → move to `UserManagement.tsx`
   - `EditUserForm` → move to `UserManagement.tsx`

2. Preserve all existing functionality:
   - No behavior changes
   - No API changes
   - Same validation logic
   - Same error handling

3. Update imports and exports

4. Delete `BootstrapCompleteness.tsx` once migration complete

**Do NOT:**

- Change API calls
- Change domain logic
- Remove functionality
- Simplify for the sake of simplification

### H. Styling

Create new SCSS modules:

- `ui/src/styles/bootstrap-navigation.module.scss`
- `ui/src/styles/readiness-widget.module.scss`
- `ui/src/styles/bid-year-setup.module.scss`
- `ui/src/styles/area-setup.module.scss`
- `ui/src/styles/user-management.module.scss`
- `ui/src/styles/bid-schedule-setup.module.scss`
- `ui/src/styles/readiness-review.module.scss`

Reuse existing styles from:

- `ui/src/styles/bootstrap.module.scss`

Follow AGENTS.md styling guidelines:

- Mobile-first responsive design
- Card-based layouts
- No inline styles
- Clear status indicators
- Touch-friendly controls

---

## UI Design Constraints

### Mobile-First

- Navigation: collapsible menu or bottom tabs
- Sections: one task at a time, vertical stacking
- Forms: single column on mobile
- All controls touch-friendly

### Workflow Guidance

- Clear "Previous" and "Next" buttons
- Show progress through workflow
- Highlight incomplete steps
- Disable "Next" if current step incomplete (optional, or allow free navigation)

### Lifecycle Awareness

- Lock completed sections post-Canonicalized
- Show clear indicators when editing blocked
- Preserve read-only access to locked sections

### Readiness Integration

- Persistent readiness widget on all screens
- Clear path from blocker to resolution
- Real-time updates when blockers resolve

---

## Testing & Validation

### Manual Validation

After implementation:

1. Complete full bootstrap workflow start to finish
2. Navigate backward and forward through sections
3. Verify browser back/forward works correctly
4. Test mobile responsiveness on all sections
5. Verify all existing functionality preserved
6. Test with 200+ users (CSV import)
7. Verify readiness widget updates in real-time
8. Test lifecycle locks engage after Canonicalized

### Regression Testing

- Verify all existing bootstrap tests still pass
- Verify no API calls changed
- Verify no domain logic changed

---

## Explicit Non-Goals

- No new backend APIs
- No domain logic changes
- No lifecycle changes
- No validation logic changes
- No styling redesign (preserve existing look and feel)
- No performance optimization

---

## Completion Conditions

This sub-phase is complete when:

- All 8 workflow sections exist as separate components
- BootstrapNavigation component exists and functional
- ReadinessWidget component exists and functional
- Routing updated and tested
- All existing functionality preserved
- `BootstrapCompleteness.tsx` deleted
- Styling follows AGENTS.md guidelines (no inline styles)
- Mobile usability verified
- Browser navigation works correctly
- Manual validation passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- All new files added via `git add`
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Existing functionality cannot be preserved
- API contracts would need to change
- Domain logic needs modification
- Lifecycle semantics are ambiguous
- Migration introduces breaking changes

---

## Risk Notes

- This is a large refactor with high risk of regressions
- Thorough testing required
- Preserve all existing behavior
- May need multiple commits to manage scope
- Coordinate with 30B and 30C to avoid duplication
- Consider doing this sub-phase in two parts if scope too large:
  - Part 1: Extract and route first 4 sections
  - Part 2: Add remaining sections and delete old component
