# Phase 30G — User Participation Flags UI & Bid Order Preview

## Purpose

Implement the UI for managing user participation flags
(`excluded_from_bidding` and `excluded_from_leave_calculation`) and
viewing deterministic bid order per area.

This sub-phase delivers the **final pre-confirmation configuration UI**
and the **bid order transparency** required for Phase 30.

---

## Scope

### A. User Participation Flags UI

Extend existing user management UI to include participation flags.

Implementation location:

- `ui/src/components/UserEditView.tsx` (existing user edit component), OR
- `ui/src/components/UserManagement.tsx` (if part of 30D restructure), OR
- New component if neither exists

#### Required Functionality

1. **Display current participation flags**
   - `excluded_from_bidding` (boolean)
   - `excluded_from_leave_calculation` (boolean)
   - Show as checkboxes or toggle switches
   - Display current state clearly

2. **Edit participation flags**
   - Allow toggling both flags independently (subject to constraint)
   - Validation: `excluded_from_leave_calculation` implies `excluded_from_bidding`
     - If user checks `excluded_from_leave_calculation`, `excluded_from_bidding` must also be checked
     - If user unchecks `excluded_from_bidding`, `excluded_from_leave_calculation` must also be unchecked
   - Show validation error if constraint violated

3. **Lifecycle constraints**
   - Flags are mutable pre-Canonicalized
   - Flags are **immutable after Canonicalized**
   - UI must disable controls when locked
   - Show clear indicator when locked

4. **Help text**
   - Explain what each flag means:
     - `excluded_from_bidding`: "User will not participate in bidding process"
     - `excluded_from_leave_calculation`: "User will not participate in leave calculation (implies excluded from bidding)"
   - Show constraint rule prominently

5. **Bulk operations** (optional enhancement)
   - If time permits, allow bulk flag updates
   - Filter users by area
   - Apply flag changes to multiple users
   - Confirm before applying

#### Backend API Integration (Participation Flags)

Add to `ui/src/api.ts`:

```typescript
export async function updateUserParticipation(
  sessionToken: string,
  userId: number,
  excludedFromBidding: boolean,
  excludedFromLeaveCalculation: boolean,
): Promise<UpdateUserParticipationResponse>;
```

Add corresponding types to `ui/src/types.ts`:

- `UpdateUserParticipationRequest`
- `UpdateUserParticipationResponse`
- Update `UserInfo` to include participation flags (if not present)

### B. Bid Order Preview Component

Create `ui/src/components/BidOrderPreview.tsx`.

This component must:

1. **Display context**
   - Bid year
   - Area (code and name)
   - Lifecycle state badge
   - Order type: "Derived" (pre-Canonicalized) or "Frozen" (post-Canonicalized)

2. **List users in bid order**
   - Display in order (1, 2, 3, ...)
   - Show user initials and name
   - Show tie-breaker inputs for transparency:
     - Cumulative NATCA BU date
     - NATCA BU date
     - EOD/FAA date
     - Service computation date
     - Lottery value (if applicable)
   - Highlight seniority conflicts (if any)
   - Show participation flags (excluded users may appear differently)

3. **Order semantics**
   - **Pre-Canonicalized**: Derived, read-only, informational
     - Show "This order is derived and may change"
     - Show "Order will be frozen at confirmation"
   - **Post-Canonicalized**: Frozen, canonical
     - Show "This order is frozen and canonical"
     - Show "Administrative adjustments may be permitted (explicit actions only)"

4. **Seniority conflict display**
   - If backend reports seniority conflicts:
     - Highlight affected users
     - Show conflict details
     - Explain why order is blocked
     - Link to resolution (update seniority data)

5. **Filtering and search**
   - Filter by participation status (all / bidding / excluded)
   - Search by initials or name
   - Mobile-friendly filters

6. **Area selection**
   - Dropdown to select area
   - Default to first area or previously viewed area
   - Persist selection across refreshes (session storage)

7. **Refresh**
   - Manual refresh button
   - Auto-refresh on live events (if applicable)
   - Loading state during refresh

#### Backend API Integration (Bid Order Preview)

Add to `ui/src/api.ts`:

```typescript
export async function getBidOrderPreview(
  sessionToken: string,
  bidYearId: number,
  areaId: number,
): Promise<GetBidOrderPreviewResponse>;
```

Add corresponding types to `ui/src/types.ts`:

- `BidOrderEntry` (user info + position + tie-breaker data)
- `GetBidOrderPreviewResponse`
- `SeniorityConflict` (if applicable)

### C. Integration with Bootstrap Workflow

If part of 30D restructure:

- User participation flags: integrated into User Management section
- Bid order preview: dedicated section or accessible from multiple sections

Routes:

- Participation flags: part of `/admin/bootstrap/users` or user edit routes
- Bid order preview: `/admin/bid-year/:bidYearId/bid-order` or `/admin/bid-year/:bidYearId/areas/:areaId/bid-order`

If standalone:

- Add routes in `ui/src/App.tsx`
- Update navigation to include bid order preview link

### D. Styling

Create or extend SCSS modules:

- `ui/src/styles/user-participation.module.scss` (if separate component)
- `ui/src/styles/bid-order-preview.module.scss`

Follow AGENTS.md styling guidelines:

- Mobile-first responsive design
- Card-based layouts for user list
- Clear order numbering (large, prominent)
- Tie-breaker data: collapsible or expandable on mobile
- Conflict highlighting: red border or background
- No inline styles
- Use existing design tokens

Key visual elements:

- Order number: Large, bold, left of user info
- User info: Initials (prominent), name, participation badges
- Tie-breaker data: Smaller text, secondary color, expandable
- Conflicts: Red badge or border, warning icon
- Order type badge: "Derived" (yellow) or "Frozen" (green)

---

## UI Design Constraints

### Mobile-First

- Bid order list: stacked cards, one user per row
- Order number: left-aligned, large font
- Tie-breaker data: tap to expand details
- Filters: collapsible on mobile
- Area selector: full-width dropdown
- Participation flags: toggle switches (touch-friendly)

### Lifecycle Awareness

- Participation flags: disabled after Canonicalized
- Bid order type: clearly labeled (derived vs frozen)
- Show lifecycle badge on all relevant screens

### Transparency

- Bid order must explain **why** users are ordered as they are
- Tie-breaker data must be visible (even if collapsed by default)
- Seniority conflicts must be explicit and actionable

### Participation Flag Constraint

- Visual enforcement of `excluded_from_leave_calculation => excluded_from_bidding`
- Disable invalid checkbox combinations
- Show inline validation error if attempted

### Error Handling

- Network errors: retry-friendly messages
- Validation errors: inline, per-field
- Backend rejections: surface structured errors
- Seniority conflicts: explain resolution path

---

## Validation & Testing

### Manual Validation — Participation Flags

After implementation:

1. Edit user participation flags
2. Attempt to check `excluded_from_leave_calculation` without `excluded_from_bidding` (should error)
3. Check both flags together (should succeed)
4. Uncheck `excluded_from_bidding` when both checked (should uncheck both)
5. Transition to Canonicalized
6. Attempt to edit flags (should be disabled)
7. Test mobile responsiveness

### Manual Validation — Bid Order Preview

After implementation:

1. View bid order for multiple areas
2. Verify users are in correct order
3. Check tie-breaker data is displayed
4. Test area selector
5. Test filters (participation status)
6. Test search by initials/name
7. View order pre-Canonicalized (should show "Derived")
8. Confirm ready to bid
9. View order post-Canonicalized (should show "Frozen")
10. Verify seniority conflict display (if applicable)
11. Test mobile responsiveness

### Constraint Validation

- Participation flag constraint enforced client-side
- Backend also enforces constraint (client is UX only)
- Lifecycle locks engage correctly
- Order display matches backend computation

### Edge Cases

- User with all tie-breaker dates identical (lottery as final tie-breaker)
- Users excluded from bidding (appear in order but marked)
- Area with zero users (empty state)
- Seniority conflict blocking readiness

---

## Backend Endpoint Verification

Before implementation, verify:

1. `POST /users/:id/participation` or equivalent exists
2. `GET /bid-years/:bid_year_id/areas/:area_id/bid-order` or equivalent exists
3. Backend validates participation flag constraint
4. Backend returns tie-breaker data in order response
5. Backend indicates whether order is derived or frozen
6. Backend reports seniority conflicts (if applicable)

If any endpoint is missing or semantics differ, **stop and document the gap**.

---

## Explicit Non-Goals

- No manual reordering (pre-Canonicalized)
- No post-Canonicalized adjustments UI (may be added in later sub-phase if needed)
- No bid window display (separate concern)
- No seniority conflict auto-resolution
- No backend implementation
- No domain logic changes

---

## Completion Conditions

This sub-phase is complete when:

- User participation flags UI exists and is functional
- Participation flag constraint enforced in UI
- BidOrderPreview component exists and renders
- Bid order displays correctly for all areas
- Tie-breaker data visible
- Order type (derived/frozen) clearly indicated
- Seniority conflict display functional (if applicable)
- Backend API integration complete
- Frontend API bindings complete
- Lifecycle constraints enforced in UI
- Mobile usability verified
- Manual validation passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- All new files added via `git add`
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Backend participation flags API does not exist
- Backend bid order preview API does not exist
- Participation flag constraint semantics unclear
- Bid order computation semantics conflict with expectations
- Tie-breaker data not available from backend
- Seniority conflict representation ambiguous
- Lifecycle enforcement missing in backend

---

## Risk Notes

- Participation flag constraint must be enforced both client and server
- Bid order computation is complex and must match backend exactly
- Tie-breaker data display may require significant UI space
- Seniority conflicts may be rare and hard to test
- Mobile display of detailed tie-breaker data may be challenging
- Order changes between pre and post-Canonicalized must be clear
