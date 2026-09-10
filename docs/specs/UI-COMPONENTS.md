# Qliphoth UI Components Specification

**Version:** 0.1.0
**Status:** Draft
**Date:** 2026-02-13

---

## 1. Overview

This specification defines the core UI component library for Qliphoth, providing accessible, composable primitives for building web applications in Sigil.

### 1.1 Design Principles

1. **Accessibility First**: All components meet WCAG 2.1 AA standards
2. **Composable**: Components combine naturally without fighting
3. **Themeable**: Styling via CSS custom properties, no hard-coded colors
4. **Evidential**: Props use Sigil's evidentiality markers where appropriate
5. **Minimal**: Only essential components; apps add domain-specific ones

### 1.2 Component Categories

| Category | Components | Status |
|----------|-----------|--------|
| Actions | Button, IconButton, Link | Planned |
| Forms | Input, Select, Checkbox, Radio, TextArea | Future |
| Layout | Container, Stack, Grid, Card | Future |
| Feedback | Alert, Toast, Modal, Spinner | Future |
| Navigation | Nav, Tabs, Breadcrumb | Future |

---

## 2. Button Component

### 2.1 Purpose

Interactive element for triggering actions or navigation.

### 2.2 Props

```sigil
☉ Σ ButtonProps {
    /// Visual style variant
    variant: ButtonVariant = ButtonVariant·Primary,

    /// Size modifier
    size: ButtonSize = ButtonSize·Medium,

    /// Disabled state (grays out, prevents interaction)
    disabled: bool = false,

    /// Loading state (shows spinner, prevents interaction)
    loading: bool = false,

    /// If provided, renders as anchor (<a>) instead of <button>
    href: Option<String> = None,

    /// Button type for form submission
    button_type: ButtonType = ButtonType·Button,

    /// Click handler (not called when disabled/loading)
    on_click: Option<Callback<MouseEvent>> = None,

    /// Child content
    children: Children,
}
```

### 2.3 Variants

```sigil
☉ ᛈ ButtonVariant {
    /// Emphasized action - solid background
    Primary,
    /// De-emphasized action - outlined
    Secondary,
    /// Subtle action - text only
    Ghost,
    /// Destructive action - red theme
    Danger,
}
```

### 2.4 Sizes

```sigil
☉ ᛈ ButtonSize {
    Small,   // Compact UI, inline actions
    Medium,  // Default
    Large,   // Primary CTAs, hero sections
}
```

### 2.5 Button Types

```sigil
☉ ᛈ ButtonType {
    Button,  // Default, no form submission
    Submit,  // Submits enclosing form
    Reset,   // Resets enclosing form
}
```

### 2.6 Behavior Contracts

1. **Click handling**:
   - When `disabled=true` OR `loading=true`: click events are ignored
   - When `href` is set: renders as `<a>`, click navigates
   - Otherwise: calls `on_click` callback

2. **Accessibility**:
   - Rendered element has `role="button"` (implicit for `<button>`)
   - When disabled: `aria-disabled="true"`, not `disabled` attribute (for focus)
   - When loading: `aria-busy="true"`
   - Keyboard: Space and Enter trigger click

3. **Styling**:
   - Uses CSS custom properties: `--btn-bg`, `--btn-fg`, `--btn-border`, etc.
   - Variants set these properties, theme provides values
   - Focus visible ring for keyboard navigation

### 2.7 Rendered Output

```html
<!-- Button (default) -->
<button
  type="button"
  class="qliphoth-btn qliphoth-btn--primary qliphoth-btn--medium"
>
  Click me
</button>

<!-- Button as link -->
<a
  href="/path"
  class="qliphoth-btn qliphoth-btn--secondary qliphoth-btn--small"
  role="button"
>
  Go somewhere
</a>

<!-- Disabled button -->
<button
  type="button"
  class="qliphoth-btn qliphoth-btn--primary qliphoth-btn--disabled"
  aria-disabled="true"
>
  Can't click
</button>
```

---

## 3. CSS Custom Properties

Components use these theme variables:

```css
/* Button */
--qliphoth-btn-bg-primary: var(--qliphoth-color-accent);
--qliphoth-btn-fg-primary: var(--qliphoth-color-on-accent);
--qliphoth-btn-bg-secondary: transparent;
--qliphoth-btn-fg-secondary: var(--qliphoth-color-accent);
--qliphoth-btn-border-secondary: var(--qliphoth-color-accent);
--qliphoth-btn-bg-ghost: transparent;
--qliphoth-btn-fg-ghost: var(--qliphoth-color-text);

/* Sizing */
--qliphoth-btn-padding-sm: 0.25rem 0.5rem;
--qliphoth-btn-padding-md: 0.5rem 1rem;
--qliphoth-btn-padding-lg: 0.75rem 1.5rem;
--qliphoth-btn-font-sm: 0.75rem;
--qliphoth-btn-font-md: 0.875rem;
--qliphoth-btn-font-lg: 1rem;

/* Focus */
--qliphoth-focus-ring: 0 0 0 2px var(--qliphoth-color-accent);
```

---

## 4. Implementation Notes

### 4.1 File Structure

```
qliphoth/src/ui/
├── mod.sigil           # Re-exports
├── button.sigil        # Button, ButtonVariant, ButtonSize
├── button_tests.sigil  # Specification tests
└── styles.sigil        # CSS generation
```

### 4.2 Dependencies

- `qliphoth::core::VNode` - Virtual DOM
- `qliphoth::hooks::use_callback` - Event handling
- `qliphoth_sys::events::MouseEvent` - Event types

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2026-02-13 | Initial draft - Button component |
