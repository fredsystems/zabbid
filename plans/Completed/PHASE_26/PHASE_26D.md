# Phase 26D — No Bid Review Workflow

## Objective

Make the No Bid area **reviewable, actionable, and operationally transparent** so admins can resolve bootstrap blockers and complete the transition to canonicalization.

Phase 25B introduced the No Bid system area as the default landing place for users without assigned areas. Phase 26D provides the UI workflow to review, understand, and clear users from No Bid so bootstrap can complete.

This phase transforms No Bid from "confusing blocker" to "clear operational checkpoint."

---

## In-Scope

### Frontend Changes

1. **No Bid Review View**
   - Dedicated page or modal to view all users in No Bid
   - List users with initials, name, user type, crew
   - Show why each user is in No Bid (if trackable)
   - Provide "Assign to Area" action for each user
   - Link directly from bootstrap blocker message

2. **Bootstrap Blocker Integration**
   - Enhance `BootstrapCompleteness.tsx` blocker display
   - Link "Users in No Bid" blocker to review workflow
   - Clear explanation: "No Bid users must be assigned before bootstrap can complete"
   - Show count and sample initials (already exists)

3. **Area Assignment from No Bid**
   - Dropdown to select target area
   - Immediate assignment (if pre-canonicalization)
   - Override workflow (if post-canonicalization, but should not happen)
   - Success feedback and list refresh

4. **Empty State Messaging**
   - When No Bid has zero users: "✅ No users in No Bid. Ready for bootstrap completion."
   - Positive reinforcement (zero is success)

5. **Mobile-First Design**
   - User list scrollable on mobile
   - Assignment dropdowns usable on touch screens
   - Clear call-to-action buttons

### Backend Verification

- Verify `list_users` can filter by system area (No Bid)
- Confirm assignment endpoint respects lifecycle rules
- Ensure No Bid does not count toward expected area count (already correct)

---

## Out-of-Scope

- User deletion from No Bid (handled in Phase 26B)
- Bulk assignment operations (future enhancement)
- "Reviewed but not yet assigned" state (no explicit acknowledgment workflow)
- Historical tracking of No Bid assignments (audit log covers this)
- No Bid configuration or customization (system area is fixed)
- Override UI for assigning to No Bid (post-canonicalization assignment to No Bid is forbidden)

---

## Frontend Changes Details

### Components Affected

#### New Components

**`ui/src/components/NoBidReview.tsx`**

Primary review component:

- Fetch users in No Bid area for active bid year
- Display user list with metadata
- Provide "Assign to Area" action per user
- Show lifecycle state context
- Handle empty state (zero users)

Structure:

```typescript
interface NoBidReviewProps {
  connectionState: ConnectionState;
  lastEvent: LiveEvent | null;
}

export function NoBidReview({ connectionState, lastEvent }: NoBidReviewProps) {
  // Fetch active bid year
  // Fetch No Bid area ID
  // Fetch users in No Bid area
  // Render user list with assignment controls
}
```

**`ui/src/components/AssignFromNoBidModal.tsx`** (Optional)

Modal for assigning a user from No Bid to an operational area:

- User context (initials, name)
- Area selection dropdown (exclude No Bid)
- Confirm / Cancel buttons
- Success / error feedback

Alternative: Inline dropdown in user list row.

#### Modified Components

**`ui/src/components/BootstrapCompleteness.tsx`**

Enhance `renderBlockingReason` function:

- For `UsersInNoBidArea` blocker, add link to No Bid review
- Button or link: "Review and Assign Users"
- Route to No Bid review view

Example:

```tsx
case "UsersInNoBidArea":
  return (
    <div className="blocking-reason">
      <strong>Users in No Bid Area:</strong> {reason.user_count} users
      must be assigned to operational areas.
      <Link to={`/admin/no-bid-review/${reason.bid_year_id}`}>
        Review and Assign Users
      </Link>
    </div>
  );
```

**`ui/src/App.tsx`**

Add route:

```tsx
<Route
  path="/admin/no-bid-review/:bidYearId"
  element={
    <NoBidReview connectionState={connectionState} lastEvent={lastEvent} />
  }
/>
```

#### API Integration

**`ui/src/api.ts`**

Add function to fetch users in system area:

```typescript
export async function listUsersInSystemArea(
  bidYearId: number,
): Promise<ListUsersResponse>;
```

Uses existing `list_users` endpoint with No Bid area ID.

Alternatively, if backend provides dedicated endpoint:

```typescript
export async function listUsersInNoBid(
  bidYearId: number,
): Promise<ListUsersResponse>;
```

**Note**: Likely uses existing `/api/areas/{area_id}/users` endpoint. Need to resolve No Bid area ID first.

#### Type Definitions

**`ui/src/types.ts`**

No new types expected. Uses existing:

- `UserInfo`
- `ListUsersResponse`
- `AreaInfo`

---

## Domain & UX Invariants

### Rules That Must Not Be Violated

1. **No Bid is a system area**
   - Cannot be deleted, renamed, or modified
   - Exists for all bid years automatically
   - Not an operational assignment target

2. **Zero users in No Bid is success**
   - Empty No Bid area = ready for bootstrap completion
   - UI must celebrate this state, not flag as error

3. **No Bid does not count toward expected counts**
   - No Bid should not have "expected user count"
   - Users in No Bid do not satisfy area user count requirements

4. **Assignment out of No Bid follows normal rules**
   - Pre-canonicalization: direct assignment allowed
   - Post-canonicalization: override required (but should not happen)
   - Cannot assign TO No Bid post-canonicalization

5. **No Bid users are valid domain users**
   - Not "broken" or "invalid"
   - Simply unassigned to operational areas
   - Must have complete metadata (name, type, crew, seniority)

6. **Bootstrap blocked until No Bid is clear**
   - Cannot complete bootstrap with users in No Bid
   - Cannot canonicalize with users in No Bid
   - This is a hard rule (already enforced in Phase 25B)

### UX Patterns

#### Empty State (Success)

```text
+-----------------------------------+
| No Bid Review                     |
+-----------------------------------+
| ✅ All users assigned              |
|                                   |
| No users remain in the No Bid     |
| area. Bootstrap can proceed.      |
|                                   |
| [Back to Bootstrap Overview]      |
+-----------------------------------+
```

#### Non-Empty State (Action Required)

```text
+-----------------------------------+
| No Bid Review                     |
| Bid Year 2026 — Draft             |
+-----------------------------------+
| ⚠️ 3 users need assignment        |
|                                   |
| These users must be assigned to   |
| operational areas before bootstrap|
| can complete.                     |
+-----------------------------------+
| User: ABC                         |
| Name: Alice Brown                 |
| Type: CPC, Crew: 1                |
| Assign to: [Dropdown_______] [Go] |
+-----------------------------------+
| User: DEF                         |
| Name: David Edwards               |
| Type: CPC-IT, Crew: 2             |
| Assign to: [Dropdown_______] [Go] |
+-----------------------------------+
| User: GHI                         |
| Name: Grace Hill                  |
| Type: CPC, Crew: 3                |
| Assign to: [Dropdown_______] [Go] |
+-----------------------------------+
```

#### Blocker Message Link

```text
⚠️ Bootstrap Blocked

3 users are still assigned to "No Bid".
These users must be reviewed and assigned to an operational area.

Sample users: ABC, DEF, GHI

[Review and Assign Users]  ← Links to No Bid review
```

---

## Risks & Ambiguities

### 1. No Bid Area ID Resolution

**Ambiguity**: How does frontend discover the No Bid area ID?

**Options**:

1. Fetch all areas, find by `is_system_area === true` and `area_code === "NO BID"`
2. Backend provides dedicated endpoint: `/api/bid-years/{id}/no-bid-users`
3. Backend includes `system_area_id` in bootstrap completeness response

**Recommendation**: Option 1 (fetch areas, filter for No Bid). Simple, uses existing APIs.

---

### 2. Assignment Endpoint

**Ambiguity**: Does assignment from No Bid use the same `update_user` endpoint as normal area reassignment?

**Answer**: Yes. `update_user` changes `area_id`. No special endpoint needed.

**Implication**: No Bid review uses existing user editing flow (Phase 26B).

---

### 3. Post-Canonicalization No Bid Users

**Scenario**: What if users end up in No Bid after canonicalization?

**Analysis**: Should not happen. Bootstrap completion is blocked if No Bid is non-empty. Cannot canonicalize with users in No Bid.

**Edge Case**: If somehow it occurs (data corruption, manual intervention), assignment requires override.

**UI Behavior**: Show override workflow (Phase 26B). Should be rare/never.

---

### 4. "Reviewed" State

**Question**: Should admins be able to mark a user as "reviewed but not yet assigned"?

**Decision**: Not in Phase 26D. Either user is in No Bid or assigned to operational area. No intermediate state.

**Future Enhancement**: Could add "reviewed" flag in later phase if workflow demands it.

---

### 5. Bulk Assignment

**Question**: Should UI support assigning multiple users at once?

**Decision**: Not in Phase 26D. Individual assignment only. Bulk operations are future enhancement.

**Rationale**: Simplicity. Admins can assign users one-by-one. Bulk rarely needed.

---

### 6. Why Users Are in No Bid

**Ambiguity**: Should UI explain why a user is in No Bid?

**Options**:

1. Show import source (if CSV imported)
2. Show assignment history (audit log)
3. No explanation (just current state)

**Recommendation**: Option 3 for Phase 26D. Current state is sufficient. Historical tracking is audit log browsing (future phase).

---

## Exit Criteria

Phase 26D is complete when:

1. ✅ No Bid review view displays all users in No Bid area
2. ✅ Users can be assigned from No Bid to operational areas
3. ✅ Bootstrap blocker links to No Bid review
4. ✅ Empty No Bid state shows success message
5. ✅ Lifecycle state visible in review view
6. ✅ Assignment uses existing `update_user` flow (no special endpoint)
7. ✅ Mobile-friendly user list and assignment controls
8. ✅ Zero users in No Bid celebrated as success
9. ✅ Live events refresh user list (when users assigned)
10. ✅ `cargo xtask ci` passes
11. ✅ `pre-commit run --all-files` passes
12. ✅ Manual testing confirms workflow end-to-end

---

## Implementation Notes

### Suggested Implementation Order

1. **Resolve No Bid area ID**
   - Add helper function to fetch No Bid area ID
   - Use `list_areas` endpoint, filter by `is_system_area` and `area_code`

2. **Create `NoBidReview` component**
   - Fetch active bid year
   - Fetch No Bid area ID
   - Fetch users in No Bid
   - Render user list

3. **Add assignment controls**
   - Dropdown for area selection (exclude No Bid)
   - Call `update_user` or similar endpoint
   - Refresh list on success

4. **Integrate with bootstrap blocker**
   - Update `renderBlockingReason` in `BootstrapCompleteness.tsx`
   - Add link to No Bid review

5. **Add route** (`App.tsx`)
   - Route to `/admin/no-bid-review/:bidYearId`

6. **Handle empty state**
   - Show success message when No Bid is empty

7. **Mobile testing**
   - Test on small screens
   - Verify dropdowns and buttons are usable

---

## UI Design Patterns

### No Bid Review Page

**Desktop:**

```text
+----------------------------------------------+
| No Bid Review — Bid Year 2026 (Draft)       |
+----------------------------------------------+
| ⚠️ 3 users need assignment                   |
|                                              |
| Users below have not been assigned to an     |
| operational area. Assign them to complete    |
| bootstrap.                                   |
+----------------------------------------------+
| +------------------------------------------+ |
| | ABC — Alice Brown                        | |
| | Type: CPC, Crew: 1                       | |
| | Assign to: [Select Area_______] [Assign] | |
| +------------------------------------------+ |
|                                              |
| +------------------------------------------+ |
| | DEF — David Edwards                      | |
| | Type: CPC-IT, Crew: 2                    | |
| | Assign to: [Select Area_______] [Assign] | |
| +------------------------------------------+ |
|                                              |
| +------------------------------------------+ |
| | GHI — Grace Hill                         | |
| | Type: CPC, Crew: 3                       | |
| | Assign to: [Select Area_______] [Assign] | |
| +------------------------------------------+ |
+----------------------------------------------+
| [Back to Bootstrap Overview]                 |
+----------------------------------------------+
```

**Mobile (stacked):**

```text
+----------------------+
| No Bid Review        |
| 2026 — Draft         |
+----------------------+
| ⚠️ 3 users           |
+----------------------+
| ABC — Alice Brown    |
| CPC, Crew: 1         |
| Assign to:           |
| [Select Area______]  |
| [Assign]             |
+----------------------+
| DEF — David Edwards  |
| CPC-IT, Crew: 2      |
| Assign to:           |
| [Select Area______]  |
| [Assign]             |
+----------------------+
| (more users...)      |
+----------------------+
| [Back to Overview]   |
+----------------------+
```

### Empty State

```text
+----------------------------------------------+
| No Bid Review — Bid Year 2026 (Draft)       |
+----------------------------------------------+
| ✅ All users assigned                         |
|                                              |
| No users remain in the No Bid area.          |
| Bootstrap can proceed.                       |
|                                              |
| [Back to Bootstrap Overview]                 |
+----------------------------------------------+
```

---

## Testing Strategy

### Manual Testing Checklist

**Pre-Conditions:**

- Create bid year in Draft state
- Create at least one operational area
- Create users and leave some in No Bid

**No Bid Review View:**

- [ ] Navigate to No Bid review from bootstrap blocker link
- [ ] User list displays all No Bid users
- [ ] User metadata visible (initials, name, type, crew)
- [ ] Area dropdown shows operational areas only (not No Bid)
- [ ] Assigning a user removes them from list
- [ ] Empty state shows success message
- [ ] Back button returns to bootstrap overview

**Bootstrap Blocker Integration:**

- [ ] "Users in No Bid" blocker shows count
- [ ] Link to review view is visible
- [ ] Clicking link navigates to No Bid review
- [ ] Sample initials displayed

**Mobile:**

- [ ] User list scrollable on 375px width
- [ ] Dropdowns usable on touch screens
- [ ] Assign buttons tappable (>= 44px)
- [ ] No horizontal scroll

**Live Events:**

- [ ] Assigning user triggers live event
- [ ] User list refreshes after assignment
- [ ] Bootstrap blocker updates when last user assigned

### Automated Testing (Optional)

- Snapshot test for empty state
- Snapshot test for user list rendering
- Unit test for No Bid area ID resolution

---

## Dependencies

### Required Existing Code

- `list_areas` endpoint (to find No Bid area ID)
- `list_users` endpoint (scoped to area)
- `update_user` endpoint (for assignment)
- `UserInfo` type with capabilities
- Bootstrap completeness with `UsersInNoBidArea` blocker
- Live events for user updates

### No New Backend Endpoints Required

Phase 26D uses only existing APIs.

---

## Rollout Considerations

### Backward Compatibility

**Change**: New UI component and route.

**Impact**: Additive. No breaking changes.

**Existing Behavior**: Bootstrap blocker already exists. Just adding link and review view.

### Rollback Plan

If Phase 26D needs to be reverted:

1. Remove `NoBidReview` component
2. Remove route from `App.tsx`
3. Remove link from bootstrap blocker

Backend remains unchanged.

---

## Non-Goals

- Bulk assignment operations
- User deletion from No Bid review (handled in Phase 26B)
- CSV import directly to operational areas (separate feature)
- Historical tracking of No Bid assignments (audit log)
- "Reviewed" acknowledgment state
- Override UI for No Bid assignment (should not be needed)
- Performance optimization for large No Bid lists

---

## Mobile-First Compliance

All UI must follow mobile-first guidelines from `AGENTS.md`:

### Required Patterns

- **Scrollable lists**: User list must scroll on mobile
- **Touch targets**: Assign buttons >= 44px height
- **Dropdowns**: Native select on mobile (accessibility)
- **Stacked layout**: User cards stack vertically
- **No horizontal scroll**: All content within viewport width

### Component-Specific

**No Bid Review:**

- User cards stack vertically on mobile
- Area dropdown full-width on mobile
- Assign button below dropdown (not inline)
- Back button visible at top and bottom

**User Card:**

- Initials and name on separate lines
- Metadata (type, crew) on separate line
- Dropdown and button stacked

---

## Validation Checklist

Before marking Phase 26D complete, verify:

- [ ] `npm run build` succeeds
- [ ] `npm run lint` passes
- [ ] `cargo xtask ci` passes (backend unchanged)
- [ ] `pre-commit run --all-files` passes
- [ ] Manual test: No Bid review displays users correctly
- [ ] Manual test: Assignment removes user from No Bid
- [ ] Manual test: Empty state shows success message
- [ ] Manual test: Bootstrap blocker link works
- [ ] Mobile test: UI usable on 375px width
- [ ] Live events test: User list refreshes after assignment
- [ ] Code review confirms No Bid area resolution is correct

---

## Next Phase

**Phase 26E** will implement bid year metadata editing, allowing admins to add labels and notes for operational context.
