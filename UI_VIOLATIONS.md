# UI Styling Violations Catalog — Phase 27I

**Generated:** Phase 27I
**Purpose:** Comprehensive audit of UI styling violations to enable targeted remediation in Phase 27J

---

## Executive Summary

This document catalogs all UI styling violations found in the admin interface.
Analysis includes systematic grep searches, SCSS review, and manual component inspection.

### Violation Counts by Category

- **Inline Styles:** 14 violations (High Priority)
- **Width Constraints:** 7 violations (Medium Priority)
- **Hover/Focus Issues:** 1 violation (Low Priority)
- **Mobile Violations:** 1 violation (Critical Priority)

### Overall Assessment

The UI is generally well-structured with good use of SCSS modules and design tokens.
Most violations are isolated inline styles that should be moved to SCSS.
One critical mobile violation exists in the responsive table pattern.

---

## Critical Priority Violations

### Violation C1: Responsive Table Forces Horizontal Scroll on Mobile

**Location:** `ui/src/styles/_cards.scss:259`

**Code:**

```scss
.responsive-table-wrapper {
  overflow-x: auto;
  margin: $spacing-md 0;
  border-radius: $radius-md;
  box-shadow: $shadow-sm;
  -webkit-overflow-scrolling: touch;

  table {
    margin: 0;
    min-width: 600px; // Force horizontal scroll on small screens
  }
}
```

**Issue:** This explicitly forces horizontal scrolling on screens smaller than 600px, which violates AGENTS.md mobile-first requirements (L572-607).

**Why It Matters:**

- Horizontal scrolling is explicitly forbidden per AGENTS.md L591-600
- Small mobile viewports (375px) will require horizontal scrolling
- Violates mobile-first design principle
- User experience degradation on mobile devices

**Expected Behavior:** Table should degrade to card layout on mobile per AGENTS.md L602-607 patterns.

**Impact:** Any page using `.responsive-table-wrapper` will break on mobile.

**Priority:** **Critical** — Breaks mobile usability

**Remediation Strategy:**

- Remove `min-width: 600px` constraint
- Use mobile/desktop toggle pattern (already exists in same file at L266)
- Show card layout on mobile, table on desktop

---

## High Priority Violations (Inline Styles)

All inline styles violate AGENTS.md L667-676: "Inline styles (`style={{ ... }}` or `style="..."`) are **not permitted**"

### Violation H1: Inline Style in UserListView Error Message

**Location:** `ui/src/components/UserListView.tsx:204`

**Code:**

```tsx
<p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
  Check the connection status indicator in the header. The UI will automatically
  refresh when the backend becomes available.
</p>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Bypasses design system, hard-codes colors and spacing

**Recommended Fix:** Create `.connection-hint` class in SCSS

---

### Violation H2: Inline Style in UserDetailView Error Message

**Location:** `ui/src/components/UserDetailView.tsx:209`

**Code:**

```tsx
<p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
  Check the connection status indicator in the header. The UI will automatically
  refresh when the backend becomes available.
</p>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Identical to H1, duplicate code

**Recommended Fix:** Create shared `.connection-hint` class in SCSS

---

### Violation H3: Inline Style in BootstrapOverview Error Message

**Location:** `ui/src/components/BootstrapOverview.tsx:135`

**Code:**

```tsx
<p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
  Check the connection status indicator in the header. The UI will automatically
  refresh when the backend becomes available.
</p>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Identical to H1/H2, duplicate code

**Recommended Fix:** Create shared `.connection-hint` class in SCSS

---

### Violation H4: Inline Style in AreaView Error Message

**Location:** `ui/src/components/AreaView.tsx:214`

**Code:**

```tsx
<p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
  Check the connection status indicator in the header. The UI will automatically
  refresh when the backend becomes available.
</p>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Identical to H1/H2/H3, duplicate code

**Recommended Fix:** Create shared `.connection-hint` class in SCSS

---

### Violation H5: Inline Style in BootstrapCompleteness Error Banner

**Location:** `ui/src/components/BootstrapCompleteness.tsx:172`

**Code:**

```tsx
<div className="error-banner" style={{ marginTop: "1rem" }}>
  <strong>No Active Bid Year</strong>
  <p>
    All mutations require an active bid year. Create a bid year below and set it
    as active before creating areas or users.
  </p>
</div>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Bypasses design system for spacing

**Recommended Fix:** Add spacing to `.error-banner` class in SCSS

---

### Violation H6: Inline Font Family in BootstrapCompleteness

**Location:** `ui/src/components/BootstrapCompleteness.tsx:936`

**Code:**

```tsx
<dd style={{ fontFamily: "monospace" }}>{area.area_code}</dd>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Bypasses design system typography, should use `$font-family-mono` variable

**Recommended Fix:** Create `.monospace-value` class using `$font-family-mono`

---

### Violation H7: Inline Italic Placeholder Style (BootstrapCompleteness)

**Location:** `ui/src/components/BootstrapCompleteness.tsx:942`

**Code:**

```tsx
<span style={{ fontStyle: "italic", color: "#888" }}>Not set</span>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Hard-codes color, bypasses design tokens

**Recommended Fix:** Create `.placeholder-text` class using `$color-text-overlay` or `$color-text-muted`

---

### Violation H8: Inline Italic Placeholder Style (BootstrapCompleteness, duplicate)

**Location:** `ui/src/components/BootstrapCompleteness.tsx:950`

**Code:**

```tsx
<span style={{ fontStyle: "italic", color: "#888" }}>N/A (System Area)</span>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Identical to H7, duplicate code

**Recommended Fix:** Use shared `.placeholder-text` class

---

### Violation H9: Inline Flex Layout in BootstrapCompleteness

**Location:** `ui/src/components/BootstrapCompleteness.tsx:962`

**Code:**

```tsx
<div
  style={{ display: "flex", gap: "0.5rem", marginTop: "0.5rem" }}
>
  <button ...>Edit Name</button>
  <button ...>Set Expected Users</button>
</div>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Layout logic in inline styles instead of SCSS

**Recommended Fix:** Create `.action-buttons` class with flex layout

---

### Violation H10: Inline Style in BootstrapCompleteness (multi-property)

**Location:** `ui/src/components/BootstrapCompleteness.tsx:1010-1014`

**Code:**

```tsx
<input
  style={{
    fontFamily: "monospace",
    marginTop: "0.25rem",
  }}
  ...
/>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Multiple style properties inline

**Recommended Fix:** Create `.monospace-input` class

---

### Violation H11: Inline Style in BootstrapCompleteness (multi-property, duplicate)

**Location:** `ui/src/components/BootstrapCompleteness.tsx:1060-1064`

**Code:**

```tsx
<input
  style={{
    fontFamily: "monospace",
    marginTop: "0.25rem",
  }}
  ...
/>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Identical to H10, duplicate code

**Recommended Fix:** Use shared `.monospace-input` class

---

### Violation H12: Inline Style in BootstrapCompleteness Warning

**Location:** `ui/src/components/BootstrapCompleteness.tsx:1202`

**Code:**

```tsx
<div className="warning-message" style={{ marginBottom: "1rem" }}>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Bypasses design system spacing

**Recommended Fix:** Add `margin-bottom` to `.warning-message` class

---

### Violation H13: Inline Italic Style in AreaView

**Location:** `ui/src/components/AreaView.tsx:302`

**Code:**

```tsx
<span style={{ fontStyle: "italic", color: "#888" }}>Not set</span>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Identical pattern to H7/H8

**Recommended Fix:** Use shared `.placeholder-text` class

---

### Violation H14: Inline Monospace Style in AreaView

**Location:** `ui/src/components/AreaView.tsx:366`

**Code:**

```tsx
<dd style={{ fontFamily: "monospace" }}>{area.area_code}</dd>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Identical to H6

**Recommended Fix:** Use shared `.monospace-value` class

---

### Violation H15: Inline Style in AreaView Button Container

**Location:** `ui/src/components/AreaView.tsx:428`

**Code:**

```tsx
<div style={{ marginLeft: "1rem", fontSize: "0.8em" }}>
```

**Severity:** High (violates AGENTS.md L667-676)

**Impact:** Layout and typography in inline styles

**Recommended Fix:** Create appropriate SCSS class for button metadata display

---

## Medium Priority Violations (Width Constraints)

### Violation M1: Max-Width Constraint in Bootstrap Overview

**Location:** `ui/src/styles/_bootstrap-overview.scss:15`

**Code:**

```scss
.bootstrap-overview {
  max-width: 1200px;
  margin: 0 auto;
  padding: $spacing-md;
```

**Impact:** Container width limited to 1200px on larger screens

**Assessment:** Acceptable constraint for readability, consistent with other modules

**Priority:** Medium (not harmful, but worth reviewing for consistency)

---

### Violation M2: Max-Width Constraint in CSV Import

**Location:** `ui/src/styles/_csv-import.scss:16`

**Code:**

```scss
.csv-import-view {
  max-width: 1200px;
```

**Impact:** Identical to M1

**Assessment:** Acceptable, maintains consistency

**Priority:** Medium

---

### Violation M3: Max-Width Constraint in User Edit View

**Location:** `ui/src/styles/_user-edit.scss:23`

**Code:**

```scss
.user-edit-view {
  max-width: 800px;
```

**Impact:** Narrower than other views

**Assessment:** Reasonable for form-focused view, but inconsistent with 1200px pattern

**Priority:** Medium

---

### Violation M4: Max-Width Constraint in Override Modal

**Location:** `ui/src/styles/_user-edit.scss:324`

**Code:**

```scss
.override-modal {
  max-width: 600px;
```

**Impact:** Modal width constraint

**Assessment:** Appropriate for modal component

**Priority:** Low (modals should be constrained)

---

### Violation M5: Max-Width Constraint in Layout

**Location:** `ui/src/styles/_layout.scss:140`

**Code:**

```scss
.app-main {
  max-width: 1400px;
```

**Impact:** Global content width constraint

**Assessment:** Acceptable, slightly wider than module constraints (1200px)

**Priority:** Medium (consider standardizing to 1200px or 1400px across all modules)

---

### Violation M6: Max-Width Constraint in Operator Management

**Location:** `ui/src/styles/_operators.scss:19`

**Code:**

```scss
.operator-management {
  max-width: 1200px;
```

**Impact:** Identical to M1/M2

**Assessment:** Acceptable, consistent pattern

**Priority:** Medium

---

### Violation M7: Max-Width Constraint in Bootstrap/No-Bid Review

**Location:**

- `ui/src/styles/_bootstrap.scss:16` (max-width: 1200px)
- `ui/src/styles/_no-bid-review.scss:16` (max-width: 1200px)

**Impact:** Identical to M1/M2/M6

**Assessment:** Consistent pattern across modules

**Priority:** Medium

---

## Low Priority Violations

### Violation L1: Table Link Underline on Hover

**Location:** `ui/src/styles/_tables.scss:105`

**Code:**

```scss
td a {
  color: $color-accent-primary;
  text-decoration: none;
  transition: color $transition-fast;

  &:hover {
    color: $mocha-sapphire;
    text-decoration: underline; // <-- Underline on hover
  }
}
```

**Issue:** Links in table cells gain underline on hover

**Assessment:**

- This is acceptable for **text links** within table cells
- AGENTS.md prohibition on underlines applies to **button-like links**
- Card footer links (L142-157 in `_cards.scss`) are styled as buttons without underlines
- This pattern distinguishes between:
  - **Button-like action links** (no underline)
  - **Text navigation links** (underline acceptable)

**Priority:** Low (acceptable distinction, but worth documenting)

**Recommendation:** No change needed, but ensure button-like links never underline

---

## Not Applicable (False Positives)

### Width: 100% Declarations

Multiple instances of `width: 100%` found in:

- Form inputs (expected and correct)
- Table containers (expected and correct)
- Navigation elements (expected and correct)

**Assessment:** These are **not violations**. They enable responsive behavior.

### Media Query Breakpoints

All media queries found use `min-width` (mobile-first approach):

- `@media (min-width: 600px)`
- `@media (min-width: 768px)`
- `@media (min-width: 1024px)`

**Assessment:** Correct mobile-first pattern, **no violations**.

### Hover States in Buttons

All button hover states checked follow AGENTS.md patterns:

- Background color changes
- No underlines added
- Proper disabled state handling
- Touch-friendly targets (min-height: 44px)

**Assessment:** **No violations** in button hover behavior.

---

## Recommended Remediation Order (Phase 27J)

### Priority 1: Critical Mobile Violation

1. **C1** — Responsive table horizontal scroll
   - Remove `min-width: 600px`
   - Implement card layout for mobile

### Priority 2: Inline Styles (All High Priority)

Create shared SCSS classes:

1. `.connection-hint` — For H1, H2, H3, H4 (identical pattern)
1. `.monospace-value` — For H6, H14
1. `.placeholder-text` — For H7, H8, H13
1. `.action-buttons` — For H9
1. `.monospace-input` — For H10, H11
1. Add spacing to `.error-banner` — For H5
1. Add spacing to `.warning-message` — For H12
1. Create class for H15 button metadata

### Priority 3: Width Constraint Standardization (Optional)

1. Standardize max-width values:
   - Consider 1200px for all module containers
   - Or 1400px to match `.app-main`
   - Document design decision

---

## Pattern Analysis

### Positive Patterns Found

✅ Consistent use of SCSS modules and design tokens
✅ Mobile-first media queries throughout
✅ Proper use of spacing variables (`$spacing-*`)
✅ Color variables used correctly (except inline styles)
✅ Touch-friendly targets (44px minimum)
✅ No hover-underline violations on buttons
✅ Proper disabled states on interactive elements
✅ Responsive grid layouts

### Anti-Patterns Found

❌ Inline styles scattered across components (14 instances)
❌ Duplicate inline style patterns (connection hints repeated 4 times)
❌ Monospace font applied inline instead of using `$font-family-mono`
❌ Hard-coded color `#888` instead of using `$color-text-muted`
❌ One explicit horizontal scroll enforcement

### Recommended Global Improvements

1. Create **shared utility classes** for common patterns:
   - `.monospace` / `.monospace-value`
   - `.placeholder-text` / `.muted-text`
   - `.connection-hint` / `.help-text`
   - `.action-group` / `.action-buttons`

2. Audit and standardize **max-width** values across modules

3. Remove **all inline styles** systematically

4. Document **when to use text links vs button-like links**

---

## Appendix: Search Commands Used

```bash
# Inline styles search
grep -rn "style={{" ui/src/components/ --include="*.tsx"
grep -rn 'style="' ui/src/components/ --include="*.tsx"

# Width constraints
grep -rn "max-width:\s*[0-9]" ui/src/styles/ --include="*.scss"
grep -rn "width:\s*[0-9]" ui/src/styles/ --include="*.scss"

# Hover states
grep -rn ":hover" ui/src/styles/ --include="*.scss"

# Media queries
grep -rn "@media" ui/src/styles/ --include="*.scss"

# Text decoration
grep -rn "text-decoration" ui/src/styles/ --include="*.scss"
```

---

## Completion Checklist

- [x] All inline styles enumerated with file/line numbers
- [x] All width constraints cataloged
- [x] Mobile review complete
- [x] Violations categorized and prioritized
- [x] No code changes made
- [x] Document structured and readable
- [x] Remediation order recommended
- [x] Pattern analysis included

---

## End of UI Violations Catalog

Phase 27I complete. All violations cataloged and ready for remediation in Phase 27J.
