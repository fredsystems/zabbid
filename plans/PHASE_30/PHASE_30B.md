# Phase 30B — Round Groups & Rounds UI

## Purpose

Implement the UI for creating, viewing, editing, and deleting round groups
and rounds, enabling operators to configure the bidding round structure
for each bid year.

This sub-phase delivers the **round configuration workflow** required for
Phase 29 pre-bid setup.

## Lifecycle terminology note

In UI, the backend lifecycle state `Canonicalized` is presented to operators as
“Ready to Bid (Confirmed)”. All lifecycle locks reference this state.

---

## Scope

### A. Round Groups Management UI

Create a new component: `ui/src/components/RoundGroupManagement.tsx`

This component must:

1. **List all round groups** for the active bid year
   - Display round group name
   - Display creation metadata
   - Do not display area association in this phase
   - Indicate lifecycle state context

2. **Create new round groups**
   - Inline or modal form
   - Required fields: name
   - Validation: name uniqueness, non-empty
   - Lifecycle constraint: creation blocked after Canonicalized

3. **Edit existing round groups**
   - Inline or modal form
   - Allow name updates only
   - Lifecycle constraint: editing blocked after Canonicalized
   - Display audit metadata (last updated)
   - When editing a round number, UI must clearly indicate that this reorders bidding sequence within the round group

4. **Delete round groups**
   - Confirmation required
   - Display associated areas warning if applicable
   - Lifecycle constraint: deletion blocked after Canonicalized
   - Backend may reject if rounds exist or areas assigned

5. **Navigate to rounds view**
   - Per round group, provide a link/button to view/manage rounds
   - Pass round_group_id to rounds component

### B. Rounds Management UI

Create a new component: `ui/src/components/RoundManagement.tsx`

This component must:

1. **Display context**
   - Show parent round group name
   - Show bid year context
   - Provide navigation back to round groups

2. **List all rounds** for the selected round group
   - Display round number (sequence position)
   - Display round name
   - Sort by sequence ascending
   - Show creation/update metadata

3. **Create new rounds**
   - Inline or modal form
   - Required fields: round number, name
   - Validation:
     - round number must be unique within group
     - round number must be positive integer
     - name must be non-empty
   - Lifecycle constraint: creation blocked after Canonicalized

4. **Edit existing rounds**
   - Inline or modal form
   - Allow round number and name updates
   - Validation: same as create
   - Lifecycle constraint: editing blocked after Canonicalized
   - Display audit metadata

5. **Delete rounds**
   - Confirmation required
   - Lifecycle constraint: deletion blocked after Canonicalized

### C. Integration with Bootstrap Workflow

Update the bootstrap navigation to include:

- A dedicated route for round groups management
- Route pattern: `/admin/bid-year/:bidYearId/round-groups`
- Sub-route for rounds: `/admin/bid-year/:bidYearId/round-groups/:roundGroupId/rounds`

Update `ui/src/components/Navigation.tsx`:

- Add "Round Groups" link (visible to admins only)
- Position logically within bootstrap flow

Update `ui/src/App.tsx`:

- Add routes for round groups and rounds components
- Ensure session token and capabilities are passed
- Include connection state and live events for real-time updates

### D. Frontend API Bindings

All round group operations are scoped to a bid year; round groups are not global.

Add to `ui/src/api.ts`:

```typescript
export async function createRoundGroup(
  sessionToken: string,
  bidYearId: number,
  name: string,
): Promise<CreateRoundGroupResponse>;

export async function listRoundGroups(
  sessionToken: string,
  bidYearId: number,
): Promise<ListRoundGroupsResponse>;

export async function updateRoundGroup(
  sessionToken: string,
  roundGroupId: number,
  name: string,
): Promise<UpdateRoundGroupResponse>;

export async function deleteRoundGroup(
  sessionToken: string,
  roundGroupId: number,
): Promise<DeleteRoundGroupResponse>;

export async function createRound(
  sessionToken: string,
  roundGroupId: number,
  roundNumber: number,
  name: string,
): Promise<CreateRoundResponse>;

export async function listRounds(
  sessionToken: string,
  roundGroupId: number,
): Promise<ListRoundsResponse>;

export async function updateRound(
  sessionToken: string,
  roundId: number,
  roundNumber: number,
  name: string,
): Promise<UpdateRoundResponse>;

export async function deleteRound(
  sessionToken: string,
  roundId: number,
): Promise<DeleteRoundResponse>;
```

Add corresponding TypeScript types to `ui/src/types.ts`:

- `RoundGroupInfo`
- `RoundInfo`
- Request/response types for all CRUD operations

### E. Styling

All styling must use SCSS modules, following the patterns from AGENTS.md:

- Create `ui/src/styles/round-groups.module.scss`
- Create `ui/src/styles/rounds.module.scss`
- Use existing design tokens and variables
- Mobile-first responsive design
- Card-based layouts for items
- Inline editing patterns
- Clear lifecycle state indicators

No inline styles permitted.

---

## UI Design Constraints

### Mobile-First

- Round group list: stacked cards on mobile
- Round list: stacked cards on mobile
- Forms: single column on mobile, may expand on desktop
- All actions touch-friendly

### Lifecycle Awareness

- All create/edit/delete controls must respect lifecycle state
- Display clear indicators when actions are blocked
- Show lifecycle badge prominently
- Use consistent badge styling from existing components

### Error Handling

- Network errors: display retry-friendly messages
- Validation errors: inline, per-field
- Backend rejections: surface structured error messages
- Transient errors: auto-dismiss after 5 seconds

### Empty States

- "No round groups configured" with clear call-to-action
- "No rounds defined for this group" with create prompt

---

## Backend Endpoint Mapping

This sub-phase assumes the following Phase 29 endpoints exist:

- `POST /round_groups` — create
- `GET /round_groups?bid_year_id=<id>` — list
- `PUT /round_groups/:id` or `POST /round_groups/update` — update
- `DELETE /round_groups/:id` — delete
- `POST /rounds` — create
- `GET /rounds?round_group_id=<id>` — list
- `PUT /rounds/:id` or `POST /rounds/update` — update
- `DELETE /rounds/:id` — delete

If any endpoint does not exist or has different semantics, **stop and document the gap**.

---

## Testing Strategy

### Manual Validation

After implementation:

1. Create 2–3 round groups
2. Add rounds to each group
3. Edit round group names
4. Edit round numbers and names
5. Attempt edits after transitioning to Canonicalized (should block)
6. Delete rounds
7. Delete round groups (should block if rounds exist)
8. Verify mobile responsiveness

### Lifecycle Validation

- Confirm all mutations blocked after Canonicalized
- Verify lifecycle badge updates in real-time
- Test with active bid year in Bootstrap_Complete state
- Attempt to delete a round group with existing rounds (backend should reject)

---

## Explicit Non-Goals

- No area → round group assignment UI (deferred to 30C)
- No bid order integration
- No user-facing round group display
- No backend implementation or fixes
- No domain logic changes

---

## Completion Conditions

This sub-phase is complete when:

- Round groups management component exists and renders
- Rounds management component exists and renders
- All CRUD operations functional via UI
- Frontend API bindings complete and tested
- Routing integrated
- Navigation updated
- Styling follows AGENTS.md guidelines (no inline styles)
- Mobile usability verified
- Lifecycle constraints enforced in UI
- Manual validation passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- All new files added via `git add`
- Changes committed

---

## Stop-and-Ask Conditions

Stop immediately if:

- Phase 29 round group/round APIs do not exist
- API semantics conflict with UI requirements
- Lifecycle enforcement is missing or inconsistent in backend
- Round group → area assignment semantics are unclear
- Domain invariants appear violated

---

## Risk Notes

- This sub-phase depends on Phase 29 APIs being complete and correct
- If APIs are missing or broken, execution will block
- Round group → area assignment is intentionally deferred to 30C
- Large monolithic BootstrapCompleteness component may cause routing conflicts
