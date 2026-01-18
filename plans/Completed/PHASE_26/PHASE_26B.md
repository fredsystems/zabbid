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
- User creation UI enhancements
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

---

## API Integration

**`ui/src/api.ts`**

Add functions:

```ts
export async function overrideAreaAssignment(
  userId: number,
  newAreaId: number,
  reason: string,
): Promise<OverrideAreaAssignmentResponse>;

export async function deleteUser(userId: number): Promise<void>;
```

---

## Type Definitions

**`ui/src/types.ts`**

Add response types:

```ts
export interface OverrideAreaAssignmentResponse {
  audit_event_id: number;
  message: string;
}
```

Ensure `UserInfo` includes:

- `capabilities: UserCapabilities`
- `is_overridden?: boolean` (if backend supports)

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
   - Overrides do NOT bypass domain rules

4. **Deletion moves to No Bid**
   - Pre-canonicalization: delete moves user to No Bid
   - Post-canonicalization: delete is denied
   - UI must explain this behavior

5. **System areas immutable**
   - Cannot reassign to No Bid post-canonicalization
   - UI must not show No Bid in area selection post-canonicalization

6. **Mobile-first**
   - Touch targets ≥ 44px
   - Forms stack vertically on mobile
   - Modals scroll correctly
   - No hover-only interactions

---

## UX Patterns

### Disabled Action Pattern

```text
[Button: Disabled]  (?)
Tooltip: "This action is disabled after canonicalization. Use an override to reassign areas."
```

### Override Required Pattern

```text
Area: North
[Change Area (Override Required)]
```

### Lifecycle Context

```text
Bid Year 2026 — Canonicalized 🔒
Editing restrictions apply. See tooltips for details.
```

---

## Risks & Ambiguities

1. **Override backend behavior**
   - Verify target area validation (must not allow system area)

2. **User deletion endpoint**
   - Confirm existence or clarify deletion-as-move-to-No-Bid semantics

3. **Override visibility**
   - Prefer simple `is_overridden` flag over audit querying

4. **Area selection**
   - Only operational areas selectable in override modal

---

## Exit Criteria

Phase 26B is complete when:

1. User editing UI consumes lifecycle-aware capabilities
2. Delete and move actions are gated correctly
3. Override modal functions end-to-end
4. Override reason validation enforced
5. Lifecycle context is always visible
6. UI is mobile-friendly
7. Backend rules are never bypassed
8. All builds, tests, and linters pass

---

## Next Phase

**Phase 26C** will implement lifecycle-aware area metadata editing using the same gating and UX patterns.
