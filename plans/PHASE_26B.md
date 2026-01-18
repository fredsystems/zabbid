# Phase 26B — User Editing UI with Lifecycle Awareness

## Objective

Make user editing **honest, safe, and lifecycle-aware** in the frontend.

Phase 26A made backend capabilities authoritative and lifecycle-aware. Phase 26B exposes those capabilities in the UI, implements override workflows for post-canonicalization changes, and ensures admins understand what they can and cannot do directly.

This phase transforms user editing from "sometimes broken" to "always intentional."

---

## In-Scope

### Frontend Changes

1. **Comprehensive User Edit UI**
   - Create dedicated `UserEditView` component (or enhance existing)
   - Consume `UserCapabilities` from API responses
   - Display user metadata with inline editing
   - Show lifecycle state context

2. **Capability-Gated Actions**
   - Disable "Delete User" button when `can_delete === false`
   - Disable direct area reassignment when `can_move_area === false`
   - Show explanatory tooltips for disabled actions
   - Maintain visual affordance (buttons visible but disabled)

3. **Override UI for Area Reassignment**
   - Modal workflow for post-canonicalization area changes
   - Require override reason (min 10 characters)
   - Call existing `override_area_assignment` backend endpoint
   - Show success/error feedback
   - Update user list after successful override

4. **Override Visibility (Read-Only)**
   - Display "Overridden" badge on users with canonical overrides
   - Tooltip: "One or more canonical fields have been overridden"
   - No detailed override browsing (future phase)

5. **User Deletion Workflow**
   - Add "Delete User" action (moves to No Bid pre-canonicalization)
   - Disable post-canonicalization (per capabilities)
   - Confirmation modal with lifecycle warning
   - Explain that deletion moves user to No Bid

6. **Mobile-First Design**
   - All editing UI must work on mobile screens
   - Touch-friendly controls
   - Responsive layout (stacked on small screens)

### Backend Verification

- Verify `update_user` handler respects lifecycle rules
- Verify proper error messages when capabilities are violated
- No new backend endpoints required (override already exists)

---

## Out-of-Scope

- Bulk user editing
- User creation UI enhancements (already exists)
- Seniority editing restrictions (allowed in all states)
- Audit log browsing
- Override history display (Phase 26F)
- Area editing (Phase 26C)
- Bid year editing (Phase 26E)
- Override execution for fields other than area assignment
- Role-based UI hiding (capabilities drive gating, not roles)

---

## Frontend Changes Details

### Components Affected

#### New Components

**`ui/src/components/UserEditView.tsx`**

Primary user editing component with:

- User metadata display (read-only fields: initials, bid year)
- Editable fields: name, user type, crew, seniority dates, lottery value
- Area reassignment control (capability-gated)
- Delete user button (capability-gated)
- Lifecycle state badge
- Override badge (if applicable)

**`ui/src/components/OverrideAreaModal.tsx`**

Modal for post-canonicalization area reassignment:

- Area selection dropdown
- Reason text area (min 10 chars, required)
- Cancel / Confirm buttons
- Loading state during submission
- Error display

#### Modified Components

**`ui/src/components/UserDetailView.tsx`**

Add:

- "Edit User" button linking to edit view
- Override badge display
- Lifecycle context indicator

**`ui/src/components/UserListView.tsx`**

Add:

- Quick action buttons (respect capabilities)
- Override badge in user list
- Lifecycle-aware action tooltips

**`ui/src/components/BootstrapCompleteness.tsx`**

Existing `EditUserForm`:

- May be replaced or enhanced with new `UserEditView`
- Ensure consistency across edit surfaces

#### API Integration

**`ui/src/api.ts`**

Add functions:

```typescript
export async function overrideAreaAssignment(
  userId: number,
  newAreaId: number,
  reason: string,
): Promise<OverrideAreaAssignmentResponse>;

export async function deleteUser(userId: number): Promise<void>;
```

#### Type Definitions

**`ui/src/types.ts`**

Add response types:

```typescript
export interface OverrideAreaAssignmentResponse {
  audit_event_id: number;
  message: string;
}
```

Ensure `UserInfo` includes:

- `capabilities: UserCapabilities`
- Add `is_overridden?: boolean` flag (if backend supports)

---

## Domain & UX Invariants

### Rules That Must Not Be Violated

1. **Capabilities are authoritative**
   - Frontend MUST NOT bypass capability checks
   - Disabled actions indicate backend will reject
   - UI reflects truth, does not invent permissions

2. **Lifecycle state is visible**
   - Users must know why actions are restricted
   - Lifecycle state displayed prominently
   - Tooltips explain restrictions

3. **Override semantics preserved**
   - Overrides are intentional, audited actions
   - Require explicit reason (min 10 chars)
   - Success confirmation includes audit event ID
   - Overrides do NOT bypass domain rules (backend still validates)

4. **Deletion moves to No Bid**
   - Pre-canonicalization: delete moves user to No Bid
   - Post-canonicalization: delete is denied
   - UI must explain this behavior

5. **System areas immutable**
   - Cannot reassign to No Bid post-canonicalization
   - UI must not show No Bid in area selection post-canonicalization

6. **Mobile-first**
   - Touch targets >= 44px
   - Forms stack vertically on mobile
   - Modals are scrollable
   - No hover-only interactions

### UX Patterns

#### Disabled Action Pattern

```text
[Button: Disabled]  (?)
Tooltip: "This action is disabled after canonicalization. Use an override to reassign areas."
```

#### Override Required Pattern

```text
Area: North
[Change Area (Override Required)]
```

Clicking opens override modal.

#### Lifecycle Context

```text
Bid Year 2026 — Canonicalized 🔒
Editing restrictions apply. See tooltips for details.
```

---

## Risks & Ambiguities

### 1. Override Backend Behavior

**Ambiguity**: Does `override_area_assignment` validate that target area is not a system area?

**Investigation Needed**: Review `crates/api/src/handlers.rs` override implementation.

**Assumption**: Backend validates. If not, add validation in Phase 26B backend work.

---

### 2. User Deletion Endpoint

**Ambiguity**: Does a `delete_user` endpoint exist, or is deletion handled via area reassignment to No Bid?

**Investigation Needed**: Search for `delete_user` handler.

**Likely Resolution**: May need to add endpoint, or deletion is move to No Bid. Clarify before implementation.

---

### 3. Override Visibility

**Ambiguity**: How does frontend know if a user has overridden fields?

**Options**:

- Add `is_overridden` flag to `UserInfo` (backend change)
- Query override audit events (complex, out of scope)
- Defer to Phase 26F

**Recommendation**: Add simple `is_overridden: boolean` flag in backend if easy. Otherwise, defer detailed override display to Phase 26F.

---

### 4. Multi-Area Selection for Overrides

**Question**: Should override modal show all areas or only operational (non-system) areas?

**Recommendation**: Show only operational areas. No Bid must not be selectable post-canonicalization.

---

### 5. Seniority Editing

**Clarification**: Seniority fields are always editable (no lifecycle restriction).

**UI Implication**: No special gating needed. Normal form validation applies.

---

### 6. Inline Edit vs Dedicated View

**Design Decision**: Inline editing (like `EditUserForm` in `BootstrapCompleteness.tsx`) vs dedicated edit page?

**Recommendation**: Dedicated view (`UserEditView`) for consistency and mobile usability. Inline edit acceptable for simple fields.

---

## Exit Criteria

Phase 26B is complete when:

1. ✅ User editing UI consumes `UserCapabilities` from API
2. ✅ "Delete User" button disabled when `can_delete === false`
3. ✅ Direct area reassignment disabled when `can_move_area === false`
4. ✅ Override modal functional for area reassignment post-canonicalization
5. ✅ Override reason validated (min 10 chars)
6. ✅ Success feedback includes audit event reference
7. ✅ Override badge displayed on users with overridden fields
8. ✅ Lifecycle state visible in user edit context
9. ✅ Tooltips explain why actions are disabled
10. ✅ All UI is mobile-friendly (tested on small screens)
11. ✅ No frontend validation bypasses backend rules
12. ✅ User deletion workflow functional (if endpoint exists)
13. ✅ `cargo xtask ci` passes
14. ✅ `pre-commit run --all-files` passes
15. ✅ Manual testing confirms lifecycle gating works

---

## Implementation Notes

### Suggested Implementation Order

1. **Verify backend behavior**
   - Confirm `override_area_assignment` endpoint behavior
   - Check if `delete_user` endpoint exists
   - Identify if `is_overridden` flag is available

2. **Add API wrappers** (`api.ts`)
   - `overrideAreaAssignment` function
   - `deleteUser` function (if applicable)

3. **Create `OverrideAreaModal` component**
   - Standalone, reusable modal
   - Area selection, reason input, validation
   - Error handling

4. **Create or enhance `UserEditView` component**
   - Form for user metadata
   - Capability-gated controls
   - Lifecycle badge display
   - Link to override modal

5. **Update `UserDetailView`**
   - Add "Edit" link
   - Show override badge

6. **Update `UserListView`**
   - Add quick actions (delete, edit)
   - Capability-aware button states

7. **Mobile testing**
   - Test on 320px, 375px, 414px widths
   - Verify touch targets
   - Check modal scrolling

8. **Integration testing**
   - Test full edit flow in Draft state
   - Test full edit flow in Canonicalized state
   - Verify override flow end-to-end

---

## UI Design Patterns

### User Edit Form Layout

**Desktop:**

```text
+-----------------------------------+
| User: ABC (Bid Year 2026)         |
| Lifecycle: Canonicalized 🔒       |
+-----------------------------------+
| Name:        [___________]        |
| User Type:   [Dropdown___]        |
| Crew:        [___]                |
| Area:        North                |
|              [Change (Override)]  |
+-----------------------------------+
| Seniority Dates                   |
| Cumulative:  [___________]        |
| NATCA BU:    [___________]        |
| EOD/FAA:     [___________]        |
| SCD:         [___________]        |
| Lottery:     [___]                |
+-----------------------------------+
| [Save Changes] [Cancel]           |
| [Delete User] (disabled)          |
+-----------------------------------+
```

**Mobile (stacked):**

```text
+------------------+
| User: ABC        |
| 2026 Canonicalized🔒 |
+------------------+
| Name             |
| [____________]   |
|                  |
| User Type        |
| [Dropdown____]   |
|                  |
| Crew             |
| [____]           |
|                  |
| Area: North      |
| [Change (Ovr)]   |
+------------------+
| (Seniority...)   |
+------------------+
| [Save Changes]   |
| [Cancel]         |
| [Delete] (off)   |
+------------------+
```

### Override Modal

```text
+--------------------------------+
| Override Area Assignment       |
+--------------------------------+
| User ABC is currently in North.|
| Select new area and provide    |
| justification.                 |
|                                |
| New Area:                      |
| [Dropdown______________]       |
|                                |
| Reason (required, min 10 char):|
| [____________________________] |
| [____________________________] |
| [____________________________] |
|                                |
| [Cancel]    [Confirm Override] |
+--------------------------------+
```

---

## Testing Strategy

### Manual Testing Checklist

**Draft State:**

- [ ] Can edit user name, type, crew
- [ ] Can change area directly (no override)
- [ ] Can delete user (moves to No Bid)
- [ ] All buttons enabled for admins

**Canonicalized State:**

- [ ] Can edit user name, type, crew, seniority
- [ ] Cannot change area directly (button disabled)
- [ ] "Override Required" shown for area change
- [ ] Cannot delete user (button disabled)
- [ ] Tooltips explain restrictions

**Override Flow:**

- [ ] Modal opens when "Change Area (Override)" clicked
- [ ] Cannot submit without reason
- [ ] Reason < 10 chars shows error
- [ ] Success shows audit event ID
- [ ] User list refreshes after override

**Mobile:**

- [ ] Forms usable on 375px width
- [ ] Buttons are tappable
- [ ] Modals scroll correctly
- [ ] No horizontal scroll

### Automated Testing (Optional)

- Snapshot tests for lifecycle badges
- Unit tests for capability-based button states
- Integration tests for override modal submission

---

## Dependencies

### Required from Phase 26A

- ✅ `UserCapabilities` in API responses (lifecycle-aware)

### Required Existing Backend

- ✅ `override_area_assignment` endpoint
- ⚠️ `delete_user` endpoint (TBD)
- ⚠️ `is_overridden` flag in `UserInfo` (optional)

### Frontend Libraries

- React Router (already used)
- Existing modal/dialog pattern (or create new)
- No new dependencies expected

---

## Rollout Considerations

### Backward Compatibility

**Change**: UI gating based on capabilities.

**Impact**: Previously visible actions may become disabled.

**Mitigation**: Tooltips explain why. Backend already enforces rules.

### Rollback Plan

If Phase 26B needs to be reverted:

1. Restore previous edit components
2. Remove override modal
3. Remove capability checks from UI

Backend remains functional (Phase 26A capabilities are additive).

---

## Non-Goals

- Audit log browsing
- Detailed override history
- Bulk operations
- CSV import enhancements
- User creation improvements
- Proxy bidding
- Round management
- Performance optimization

---

## Mobile-First Compliance

All UI must follow mobile-first guidelines from `AGENTS.md`:

### Required Patterns

- **Responsive layouts**: Use flexbox, stack vertically on mobile
- **Touch targets**: Minimum 44px height for buttons
- **No hover-only**: All interactions must work via tap
- **Readable text**: No zoom required
- **Scrollable modals**: Long content must scroll, not overflow

### Component-Specific

**`UserEditView`:**

- Form fields stack vertically on mobile
- Labels above inputs (not side-by-side)
- Buttons full-width on mobile

**`OverrideAreaModal`:**

- Modal scrolls if content exceeds viewport
- Buttons stacked on mobile (Cancel / Confirm)
- Textarea resizes with content

**`UserListView`:**

- Action buttons condensed or moved to detail view on mobile
- Consider swipe actions or icon-only buttons

---

## Validation Checklist

Before marking Phase 26B complete, verify:

- [ ] `npm run build` succeeds
- [ ] `npm run lint` passes
- [ ] `cargo xtask ci` passes (backend unchanged)
- [ ] `pre-commit run --all-files` passes
- [ ] Manual test: Edit user in Draft state (all actions work)
- [ ] Manual test: Edit user in Canonicalized state (area change requires override)
- [ ] Manual test: Override workflow completes successfully
- [ ] Manual test: Disabled actions show tooltips
- [ ] Mobile test: UI usable on 375px width
- [ ] Code review confirms capability checks are correct
- [ ] No hardcoded role checks (capabilities only)
- [ ] Override modal validates reason length

---

## Next Phase

**Phase 26C** will implement area metadata editing with similar lifecycle awareness and gating patterns.
