# Phase 22 — UI Refinement & Navigation Pass

## Goal

Polish, stabilize, and harden the UI now that the domain model and API surfaces are solid.

This phase exists to:

- Fix known usability and accessibility issues
- Align the UI with established styling guidelines
- Correct navigation and routing problems
- Improve ergonomics without introducing new domain concepts

**No domain rules, persistence schemas, or business logic are changed in this phase.**

---

## Scope

### UI Styling & Accessibility Fixes

The following issues **must be explicitly addressed**:

1. **Insufficient text contrast on buttons**
   - Login button text
   - Bootstrap user creation buttons
   - Operator page:
     - Delete operator button
     - Create operator button
     - Create operator form:
       - Cancel button (currently green, incorrect semantics)
       - Create button (green, insufficient contrast)

   **Root issue:**
   Text is not legible against bright or saturated background colors.

   **Required fixes:**
   - Ensure all buttons meet acceptable contrast ratios
   - Use dark text (`$color-bg-base`) on colored backgrounds
   - Reserve green only for success states, not neutral actions

2. **Date input robustness**
   - Prevent year field overflow when users continue typing
   - Clamp or validate date input gracefully
   - No silent truncation or broken layouts

---

### UI Styling Rules (Enforced)

The UI **must conform** to the styling rules already added to `AGENTS.md`, including:

- Card-based layouts instead of tables
- Logical sectioning with clear headings
- Progressive disclosure for create/edit forms
- Inline editing where appropriate
- Mobile-first layout assumptions
- Consistent button semantics:
  - Primary / Save / Cancel / Destructive clearly differentiated

These rules are **mandatory**, not advisory.

---

### Navigation & Routing

1. **Navigation improvements**
   - Introduce a clear, consistent navigation mechanism
     - Dropdown-based navigation is acceptable and encouraged
   - All admin views must provide a way to return to the main admin screen
   - The UI must clearly answer:
     > “Where am I right now?”

2. **Routing bug fixes**
   - Fix invalid route errors, including:
     - `No routes matched location "/bid-year/2026/areas"`
   - All navigation links must resolve correctly
   - No console errors during normal navigation

---

### Admin UX Improvements

- Clear visual context for:
  - Active bid year
  - Current section (operators, bootstrap, users, areas, etc.)
- Destructive actions must:
  - Be visually distinct
  - Require explicit confirmation
- Button color semantics must align with action intent:
  - Create ≠ Cancel ≠ Delete

---

### Bootstrap UX Refinements (UI Only)

- When **no bid years exist** and a new bid year is created:
  - Automatically set it as the active bid year (UI behavior only; domain already enforces active bid year)
- Bid year creation UI must include:
  - Expected area count input
- No bootstrap logic changes occur in this phase — this is strictly presentation and flow refinement

---

## Explicit Non-Goals

Phase 22 **does NOT** include:

- Domain logic changes
- API contract changes (beyond minor ergonomic fixes)
- Persistence schema changes
- Authorization or role model changes
- User identifier refactors
- CSV import logic changes
- Bidding logic or rounds
- Public-facing UI features

---

## Exit Criteria

Phase 22 is complete when:

- All listed UI contrast issues are fixed
- All buttons follow consistent semantic styling
- Navigation is intuitive, consistent, and error-free
- Routing errors are eliminated
- UI follows the styling guidelines in `AGENTS.md`
- The UI is fully usable on mobile screens
- No domain, persistence, or business logic has changed
- No new validation logic exists outside the backend
