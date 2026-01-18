# Phase 27I — UI Styling Audit and Violation Catalog

## Purpose

Identify UI styling violations and precision issues without fixing them, creating a complete catalog to enable targeted remediation.

## Scope

### Analysis Tasks

- Grep for inline styles: `grep -r "style={{" --include="*.tsx" --include="*.ts"`
- Grep for inline styles: `grep -r 'style="' --include="*.tsx" --include="*.ts"`
- Review `ui/src/components/admin/` for:
  - Fixed width constraints (width: 400px, max-width: 600px, etc.)
  - Hover/visited inconsistencies on buttons and links
  - Desktop-first layout assumptions
  - Hard-coded spacing values
- Manual inspection of UI at mobile viewport (375px width)
- Identify forms or tables that break on mobile
- Identify components that violate AGENTS.md styling rules (L591-600)

### Violation Categories

#### Inline Style Violations

Any use of `style={{}}` or `style=""` attributes in TSX/JSX code.

AGENTS.md L591-600 explicitly forbids inline styles:

- All styling must use SCSS modules
- Inline styles bypass design consistency
- Inline styles are not auditable

#### Width Constraint Violations

Fixed-width containers that prevent responsive behavior:

- Hard-coded pixel widths on containers
- Max-width constraints that are too narrow
- Assumptions about desktop viewport size

#### Hover/Visited Inconsistencies

- Buttons that underline on hover (should not)
- Text color changes that reduce readability
- Inconsistent hover states across similar components
- Missing focus states for accessibility

#### Mobile-First Violations

- Layouts that assume desktop viewport
- Horizontal scrolling required on mobile
- Touch targets smaller than 44px
- Text requiring zoom to read
- Navigation requiring precise pointer

### Output Artifact

Create `UI_VIOLATIONS.md` with:

- **Inline Styles Section**:
  - File path and line number
  - Excerpt of violating code
  - Severity (critical if functional, high if cosmetic)

- **Width Constraints Section**:
  - Component name and file path
  - Description of constraint
  - Impact on mobile usability

- **Hover/Visited Inconsistencies Section**:
  - Component name
  - Specific inconsistency observed
  - Expected behavior per AGENTS.md patterns

- **Mobile Violations Section**:
  - Component or page affected
  - Symptom (breaks, scrolls, illegible, etc.)
  - Screenshot reference or detailed description

- **Priority Ranking**:
  - Critical: Breaks functionality on mobile
  - High: Violates AGENTS.md rules (inline styles)
  - Medium: Usability issues
  - Low: Polish issues

## Explicit Non-Goals

- Do NOT fix any violations in this phase
- Do NOT refactor components in this phase
- Do NOT create new SCSS modules in this phase
- Do NOT test UI behavior or functionality
- Do NOT add new features or change layouts beyond cataloging issues

## Files Likely to Be Analyzed

### UI Components

- `ui/src/components/admin/users/`
- `ui/src/components/admin/areas/`
- `ui/src/components/admin/bid_years/`
- `ui/src/components/admin/operators/`
- `ui/src/components/admin/bootstrap/`
- Any other admin UI components

### Styles

- `ui/src/styles/` (review for patterns to compare against)
- Component-specific SCSS modules
- Global style variables and tokens

## Search Patterns

Execute the following searches to identify violations:

```bash
# Find all inline styles in TypeScript/TSX
grep -rn "style={{" ui/src/ --include="*.tsx" --include="*.ts"
grep -rn 'style="' ui/src/ --include="*.tsx" --include="*.ts"

# Find fixed width constraints in SCSS
grep -rn "width: [0-9]" ui/src/styles/ --include="*.scss"
grep -rn "max-width: [0-9]" ui/src/styles/ --include="*.scss"

# Find hover states
grep -rn ":hover" ui/src/styles/ --include="*.scss"

# Find media queries (to assess mobile-first approach)
grep -rn "@media" ui/src/styles/ --include="*.scss"
```

## Mobile Viewport Testing

Test key admin pages at these viewport sizes:

- 375px (iPhone SE, small mobile)
- 768px (tablet portrait)
- 1024px (tablet landscape / small desktop)

For each page, verify:

- All content visible without horizontal scroll
- All buttons and links are tappable (min 44px touch target)
- Text is readable without zoom
- Forms are usable
- Navigation is accessible

Document any failures with:

- Viewport size where issue appears
- Specific symptom
- Which component is affected

## Example Violation Entries

### Inline Style Example

````markdown
#### Violation: Inline Style in User Form

**Location**: `ui/src/components/admin/users/UserForm.tsx:45`

**Code**:

```tsx
<div style={{ width: '400px', marginTop: '20px' }}>
```

**Severity**: High (violates AGENTS.md L591-600)

**Impact**: Bypasses design system, hard-codes dimensions

### Width Constraint Example

```markdown
#### Violation: Fixed Width Container

**Location**: `ui/src/components/admin/areas/AreaList.tsx`

**Description**: Container has `max-width: 600px` hard-coded in SCSS

**Impact**: On larger screens, content is unnecessarily constrained

**Priority**: Medium
```
````

### Mobile Breakage Example

```markdown
#### Violation: Table Layout Breaks on Mobile

**Location**: `ui/src/components/admin/users/UserList.tsx`

**Symptom**: Table requires horizontal scrolling at 375px viewport

**Expected**: Should use card layout per AGENTS.md L602-607

**Priority**: Critical (breaks mobile usability)
```

## Completion Conditions

- `UI_VIOLATIONS.md` created with complete catalog
- All inline styles enumerated with file/line numbers
- All width hacks identified and categorized
- Mobile violations documented with viewport sizes
- All violations have priority rankings
- Document includes screenshots or detailed descriptions
- Document passes markdown linting (`cargo xtask ci`)
- Document passes pre-commit hooks
- Git commit contains only the violation catalog

## Dependencies

None (can run in parallel with other phases)

## Blocks

- Phase 27J (UI Precision Fixes)

Cannot fix UI violations without knowing what they are.

## Execution Notes

### Systematic Review Process

For each admin component:

1. Review TSX/JSX for inline styles
2. Review SCSS for width constraints
3. Test at mobile viewport
4. Document all violations found
5. Assign priority

Work through components systematically to ensure complete coverage.

### Reference Patterns from AGENTS.md

Use AGENTS.md L616-693 as reference for correct patterns:

- Form controls: Dark background, proper contrast
- Buttons: Consistent styling with states
- Item cards: Structured layout patterns
- Responsive: Mobile-first breakpoints

Compare actual implementation against these patterns.

### Screenshot Guidelines

For mobile violations, either:

- Include screenshot files in `docs/phase_27/ui_violations/`
- Provide detailed text description sufficient for remediation

Screenshots improve clarity but are not required if description is thorough.

### Severity Assessment

**Critical**: Prevents mobile users from completing tasks

- Forms that don't submit
- Buttons off-screen
- Text completely illegible

**High**: Violates AGENTS.md rules or severely impacts usability

- Inline styles (explicit rule violation)
- Requires horizontal scrolling
- Touch targets too small

**Medium**: Degrades experience but functional

- Inconsistent hover states
- Unnecessary width constraints
- Awkward but usable layouts

**Low**: Polish issues only

- Minor spacing inconsistencies
- Color contrast slightly off but readable
- Cosmetic improvements

### Completeness Check

Before marking phase complete, verify:

- All admin components reviewed
- All TSX/JSX files grepped for inline styles
- All SCSS files reviewed for constraints
- Mobile testing complete for all key pages
- Catalog is organized and readable
- Priorities are assigned consistently

### Documentation Quality

The catalog must be detailed enough that Phase 27J can implement fixes without re-analyzing components.

Include:

- Exact file paths and line numbers
- Code excerpts or screenshots
- Clear description of violation
- Expected correct behavior
- Priority for remediation

This enables efficient implementation in 27J.
