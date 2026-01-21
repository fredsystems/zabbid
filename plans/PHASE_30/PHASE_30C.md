# Phase 30C — Area → Round Group Assignment UI

## Purpose

Implement the UI for assigning round groups to areas, enforcing the
constraint that each non-system area must have exactly one assigned
round group.

This sub-phase delivers the **area-to-round-group binding workflow**
required to complete pre-bid configuration.

Area list is scoped to a specific bid year (bidYearId route param), never global.

---

## Scope

### A. Area Round Group Assignment Component

Create or extend existing area management UI to include round group assignment.

Two implementation options:

#### Option 1: Extend AreaView.tsx

Add round group assignment controls directly to the existing area view.

#### Option 2: Create dedicated component

Create `ui/src/components/AreaRoundGroupAssignment.tsx`

#### Decision criteria

- If AreaView.tsx is already large (>500 lines), use Option 2
- If round group assignment is a distinct workflow step, use Option 2
- Otherwise, extend AreaView.tsx

### B. Required Functionality

The UI must provide:

1. **List all non-system areas** for the active bid year
   - Display area code and name
   - Display currently assigned round group (if any)
   - Show lifecycle state context

2. **Assign round group to area**
   - Dropdown or select control showing all available round groups
   - One assignment per area (replace existing assignment)
   - Validation: round group must exist
   - Lifecycle constraint: assignment blocked after Canonicalized

3. **Clear/unassign round group**
   - Allow unassign pre-Canonicalized even if it makes readiness blocked.
   - UI should reflect it immediately as a blocker (not treat it as an error).
   - Backend may reject unassign only if the lifecycle is locked or if the area is system.

4. **Visual indicators**
   - Areas with assignments: show round group name prominently
   - Areas without assignments: show "Not Assigned" state
   - Use color coding or badges for assignment status

5. **Readiness integration**
   - Display whether area is blocking readiness due to missing round group
   - Link to readiness review if applicable
   - If an area has no round group assigned, show a badge “Blocks Readiness” and a link/button “View readiness blockers”.

### C. Backend API Requirements

This sub-phase assumes one of the following API patterns exists:

#### Pattern A: Dedicated assignment endpoint

```http
POST /areas/:area_id/assign-round-group
{
  "round_group_id": <id> | null
}
```

#### Pattern B: Area update endpoint (existing)

```http
POST /areas/update
{
  "area_id": <id>,
  "round_group_id": <id> | null,
  ...other fields
}
```

#### Pattern C: Separate round group assignment endpoint

```http
POST /round_groups/:round_group_id/assign-area
{
  "area_id": <id>
}
```

Preferred: Pattern B (updateArea with round_group_id)

Acceptable: Pattern A (dedicated endpoint)

Not preferred: Pattern C (assign-area from round group) — because it tends to imply the group “owns” the relationship, which conflicts with your operator workflow and readiness gating.

If **none of these patterns exist**, this is a **blocking gap**.

Document the actual API pattern discovered during implementation.

### D. Frontend API Bindings

Add to `ui/src/api.ts` (if not already present):

```typescript
export async function assignRoundGroupToArea(
  sessionToken: string,
  areaId: number,
  roundGroupId: number | null,
): Promise<AssignRoundGroupToAreaResponse>;
```

Or extend existing `updateArea` function if that's the chosen pattern.

Add corresponding types to `ui/src/types.ts`:

- `AssignRoundGroupToAreaRequest` (if needed)
- `AssignRoundGroupToAreaResponse` (if needed)
- Update `AreaInfo` to include `round_group_id` and `round_group_name` (if not present)

### E. Integration Points

Update bootstrap workflow navigation:

- Ensure round group assignment is accessible from bootstrap flow
- Position logically after round groups are configured
- Before readiness review

If extending AreaView.tsx:

- Add round group assignment section to existing area cards
- Respect existing inline-edit patterns

If creating new component:

- Add route: `/admin/bid-year/:bidYearId/area-round-groups`
- Update Navigation.tsx with link
- Update App.tsx with route

### F. Styling

All styling must use SCSS modules:

- Extend `ui/src/styles/areas.module.scss` (if extending AreaView), or
- Create `ui/src/styles/area-round-groups.module.scss` (if new component)

Follow AGENTS.md styling guidelines:

- Mobile-first responsive design
- Card-based layouts
- Clear assignment status indicators
- Inline editing patterns
- No inline styles

---

## UI Design Constraints

### Mobile-First

- Area list: stacked cards
- Round group selector: full-width on mobile
- Assignment status: prominent, top of card

### Lifecycle Awareness

- Assignment controls disabled after Canonicalized
- Clear visual indication when locked
- Lifecycle badge visible

### Readiness Integration

- If an area is blocking readiness due to missing round group:
  - Highlight the area
  - Show explicit blocker message
  - Provide direct path to assign

### Empty States

- "No areas configured" (should not occur if bootstrap is in progress)
- "No round groups available" (blocks assignment, show create prompt)

### Error Handling

- Network errors: retry-friendly
- Validation errors: inline, per-field
- Backend rejections: surface structured errors
- Assignment conflicts: explain constraint violation

---

## Validation & Testing

### Manual Validation

After implementation:

1. Create 2+ round groups
2. Create 3+ non-system areas
3. Assign round groups to areas
4. Change assignments
5. Attempt to clear assignment (verify behavior)
6. Transition to Canonicalized
7. Attempt assignment (should block)
8. Verify readiness blocker appears for unassigned areas
9. Test mobile responsiveness

### Constraint Validation

- Verify exactly-one-per-area constraint
- Verify system areas are excluded
- Verify lifecycle locks engage correctly

---

## Backend Dependency Verification

Before starting implementation, verify:

1. Backend API for area → round group assignment exists
2. Lifecycle constraints are enforced server-side
3. Readiness checking includes round group assignment status
4. Area list response includes round group assignment data

If any of these are missing, **stop and document the gap**.

---

## Explicit Non-Goals

- No round group creation (handled in 30B)
- No area creation (already exists)
- No bid order integration
- No backend implementation
- No domain logic changes

---

## Completion Conditions

This sub-phase is complete when:

- Area → round group assignment UI exists and is functional
- All non-system areas can be assigned round groups
- Lifecycle constraints respected in UI
- Readiness integration functional (if applicable)
- Frontend API bindings complete
- Routing integrated (if new component)
- Navigation updated (if new component)
- Styling follows AGENTS.md guidelines (no inline styles)
- Mobile usability verified
- Manual validation passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- All new files added via `git add`
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Backend API for area → round group assignment does not exist
- API semantics are unclear or ambiguous
- Lifecycle enforcement missing in backend
- Readiness checking does not include round group assignment
- Domain invariants appear violated
- Multiple conflicting API patterns exist

---

## Risk Notes

- This sub-phase depends on Phase 29 area → round group assignment API
- If API is missing, this is a blocking gap for Phase 30
- Readiness integration assumes backend computes round group assignment status
- May require coordination with BootstrapCompleteness refactor (30D)
