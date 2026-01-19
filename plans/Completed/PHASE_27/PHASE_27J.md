# Phase 27J — UI Precision Fixes

## Purpose

Fix UI styling violations identified in Phase 27I to ensure compliance with AGENTS.md styling rules and mobile-first requirements.

## Scope

### Implementation Tasks Based on 27I Catalog

Based on findings from Phase 27I (UI violation catalog):

- Remove all inline styles identified in 27I
- Migrate inline styles to SCSS modules
- Remove arbitrary width constraints
- Fix hover/visited inconsistencies on buttons and links
- Ensure mobile-first compliance for all flagged components
- Verify touch targets meet minimum size (44px)
- Fix any layouts that break at mobile viewports

### SCSS Patterns to Follow

Per AGENTS.md L616-693:

#### Form Controls

- Dark background (`$color-bg-base`) not white
- Proper text contrast (`$color-text-primary`)
- Consistent border and focus states
- Adequate padding for touch targets

#### Buttons

- Primary/Create: Colored background with dark text
- Save: Teal background with dark text for readability
- Edit: Subtle, bordered style for secondary actions
- Cancel: Border-only style, clearly different from primary
- Consistent hover states with box shadows
- Proper disabled states (opacity: 0.5)

#### Item Cards

- Structured header/body layout
- Consistent spacing and borders
- Responsive behavior at breakpoints

#### Responsive Behavior

- Mobile (< 600px): Stack all elements vertically
- Tablet (600px+): Side-by-side labels and inputs
- Desktop (768px+): More generous padding, wider max-widths

## Explicit Non-Goals

- Do NOT add new features
- Do NOT change domain behavior or API interactions
- Do NOT refactor unrelated components beyond violations
- Do NOT introduce new design patterns not documented in AGENTS.md
- Do NOT modify backend code

## Files Likely to Be Affected

### Component Files

Files identified in Phase 27I:

- `ui/src/components/admin/users/*.tsx`
- `ui/src/components/admin/areas/*.tsx`
- `ui/src/components/admin/bid_years/*.tsx`
- `ui/src/components/admin/operators/*.tsx`
- `ui/src/components/admin/bootstrap/*.tsx`

### SCSS Modules

May need to create or modify:

- Component-specific SCSS modules (e.g., `UserForm.module.scss`)
- Shared SCSS partials if patterns are reusable
- Variable imports from design token files

### Potentially New Files

- New SCSS modules for components that currently use inline styles
- Shared SCSS partials for common patterns (if multiple components need same fix)

## Fix Patterns

### Pattern 1: Inline Style to SCSS Module

Before:

```tsx
<div style={{ width: "400px", marginTop: "20px" }}>{content}</div>
```

After:

```tsx
// Component.tsx
import styles from "./Component.module.scss";

<div className={styles.container}>{content}</div>;
```

```scss
// Component.module.scss
@import "../../styles/variables";

.container {
  max-width: 400px; // Note: use max-width not width for responsiveness
  margin-top: $spacing-md;
}
```

### Pattern 2: Fixed Width to Fluid Width

Before:

```scss
.form-container {
  width: 600px;
}
```

After:

```scss
.form-container {
  max-width: 600px;
  width: 100%; // Fluid on smaller screens
}
```

### Pattern 3: Hover/Visited Consistency

Before:

```scss
.button:hover {
  text-decoration: underline;
  color: blue;
}
```

After:

```scss
.button {
  text-decoration: none;
  color: $color-text-primary;

  &:hover,
  &:visited {
    text-decoration: none;
    color: $color-text-primary;
    background-color: $color-accent-primary-hover;
  }
}
```

### Pattern 4: Mobile-First Responsive

Before (desktop-first):

```scss
.layout {
  display: grid;
  grid-template-columns: 1fr 1fr;

  @media (max-width: 768px) {
    grid-template-columns: 1fr;
  }
}
```

After (mobile-first):

```scss
.layout {
  display: grid;
  grid-template-columns: 1fr; // Mobile default

  @media (min-width: 600px) {
    grid-template-columns: 1fr 1fr; // Enhance for larger screens
  }
}
```

## Priority Order for Fixes

Fix violations in priority order from Phase 27I:

1. **Critical Priority** (mobile breakage, functionality blocked)
2. **High Priority** (inline style violations, AGENTS.md rule violations)
3. **Medium Priority** (usability issues, inconsistencies)
4. **Low Priority** (polish, cosmetic improvements)

Stop after critical and high priority violations are fixed. Medium and low priority can be deferred if time-constrained.

## Completion Conditions

- Zero inline styles remain in TSX/JSX files
- All width constraints are fluid or grid-based (no hard-coded widths)
- Hover/visited behavior consistent across admin UI
- All critical and high priority violations from 27I resolved
- Mobile verification passes for all fixed components (375px, 768px, 1024px viewports)
- No regressions in existing functionality
- Git commit focused on UI precision only
- Commit message references Phase 27I violation numbers or descriptions

## Dependencies

- Phase 27I must be complete (requires violation catalog)

Cannot fix violations without knowing what they are.

## Blocks

None (can run in parallel with 27H after 27I completes)

## Execution Notes

### Incremental Approach

Fix components incrementally:

1. Start with critical mobile breakage
2. Fix all inline style violations (high priority)
3. Fix width constraints causing responsive issues
4. Fix hover/visited inconsistencies
5. Address remaining medium/low priority items if time permits

Commit fixes in logical groups (e.g., "Fix inline styles in user components") for easier review.

### SCSS Module Organization

For components without existing SCSS modules:

- Create module named `ComponentName.module.scss`
- Place in same directory as component
- Import design tokens from `../../styles/variables`

For shared patterns:

- Consider creating shared partial in `ui/src/styles/components/`
- Import into component modules as needed

Keep SCSS modular and component-scoped.

### Mobile Verification Process

For each fixed component:

1. Open browser dev tools
2. Set viewport to 375px (iPhone SE)
3. Verify layout works without horizontal scroll
4. Verify all buttons are tappable
5. Verify text is readable
6. Test at 768px (tablet)
7. Test at 1024px (desktop)

Document any remaining issues in commit message or follow-up task.

### Design Token Usage

Always use design tokens from variables file:

- Spacing: `$spacing-xs`, `$spacing-sm`, `$spacing-md`, `$spacing-lg`
- Colors: `$color-bg-base`, `$color-text-primary`, `$color-accent-primary`
- Borders: `$color-border`, `$radius-md`

Do NOT hard-code values like `#333` or `12px` directly in SCSS.

### Hover State Consistency

Ensure all interactive elements (buttons, links) have consistent hover behavior:

- Buttons: Background color change, NO underline
- Links (in text): Underline acceptable
- Icon buttons: Background or color change, NO underline

Follow patterns from existing Bootstrap Completeness implementation.

### Touch Target Size

All interactive elements must be at least 44px in height/width for mobile:

```scss
.button {
  min-height: 44px;
  padding: $spacing-sm $spacing-md;
}
```

This ensures usability on touch devices.

### When to Create New SCSS Modules

Create new SCSS module when:

- Component currently uses only inline styles
- Component has complex styling that would clutter shared partials
- Component styling is unique and not reusable

Use shared partials when:

- Multiple components need identical patterns
- Pattern is documented in AGENTS.md (form controls, buttons, cards)

### Verification After Fixes

After implementing all fixes:

1. Run `npm run build` to ensure no SCSS compilation errors
2. Manually test all fixed components in browser
3. Verify mobile usability at 375px viewport
4. Verify no functional regressions
5. Run any UI-related tests (if they exist)

### Handling Edge Cases

If a fix would require significant layout refactoring:

- Document the scope of required changes
- Assess if it's truly critical or can be deferred
- Ask user for guidance if refactor seems disproportionate

Some violations may require component restructuring beyond simple styling fixes.

### Documentation

Add comments in SCSS for non-obvious decisions:

```scss
// Mobile-first: stack vertically by default, then enhance for larger screens
.form-layout {
  display: flex;
  flex-direction: column;

  @media (min-width: 600px) {
    flex-direction: row;
  }
}
```

This helps future maintainers understand responsive strategy.

### Post-Fix Validation

After all fixes complete:

1. Review UI in browser at all three viewport sizes
2. Verify zero inline styles via grep: `grep -r "style={{" ui/src/`
3. Check for unintentional regressions
4. Ensure all fixes align with AGENTS.md patterns
5. Verify git commit includes only style-related changes

If validation fails, address issues before marking phase complete.

## Success Criteria

This phase succeeds when:

- All critical and high priority violations from 27I are resolved
- Zero inline styles remain in UI code
- Mobile usability verified for all admin components
- No functional regressions introduced
- All changes use SCSS modules and design tokens
- Code passes pre-commit hooks and linting
