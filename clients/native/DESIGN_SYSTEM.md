# Photo Store Design System

A modern, minimalist dark-themed design system built with SCSS for the Photo Store application.

## Overview

This design system provides a consistent, accessible, and maintainable foundation for building the Photo Store UI. It features a bottle-green primary color, soft dark backgrounds, and clean typography optimized for photo browsing.

## Quick Start

The design system is automatically imported globally via `src/styles.scss`. For component-specific styling:

```scss
@use '../../theme/variables' as *;
@use '../../theme/mixins' as *;
```

## Design Tokens

### Colors

#### Primary Colors
- **Primary**: `#2d5016` (Bottle Green)
- **Primary Light**: `#3d6b1f`
- **Primary Dark**: `#1f3910`
- **Primary Hover**: `#3d6b1f`

#### Background Colors
- **Base**: `#1a1a1a` (Soft dark gray)
- **Elevated**: `#242424` (Cards, panels)
- **Elevated Hover**: `#2a2a2a`
- **Surface**: `#2a2a2a`
- **Overlay**: `rgba(0, 0, 0, 0.8)`

#### Text Colors (WCAG AA Compliant)
- **Primary**: `#e5e5e5`
- **Secondary**: `#a3a3a3`
- **Muted**: `#737373`
- **Disabled**: `#525252`
- **On Primary**: `#ffffff`

#### Semantic Colors
- **Success**: `#22c55e` (Green)
- **Warning**: `#f59e0b` (Amber)
- **Error**: `#ef4444` (Red)
- **Info**: `#3b82f6` (Blue)

#### Border Colors
- **Default**: `#404040`
- **Light**: `#525252`
- **Focus**: `$color-primary` (Bottle green)

### Typography

#### Font Family
```scss
font-family: 'Roboto', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Helvetica Neue', Arial, sans-serif;
```

#### Font Sizes
| Token | Size | px Value |
|-------|------|----------|
| `$font-size-xs` | 0.75rem | 12px |
| `$font-size-sm` | 0.875rem | 14px |
| `$font-size-base` | 1rem | 16px |
| `$font-size-lg` | 1.125rem | 18px |
| `$font-size-xl` | 1.25rem | 20px |
| `$font-size-2xl` | 1.5rem | 24px |
| `$font-size-3xl` | 1.875rem | 30px |
| `$font-size-4xl` | 2.25rem | 36px |

#### Font Weights
- **Normal**: `400`
- **Medium**: `500`
- **Bold**: `700`

#### Line Heights
- **Tight**: `1.25`
- **Normal**: `1.5`
- **Relaxed**: `1.75`

### Spacing Scale (8px base unit)

| Token | Value | px Value |
|-------|-------|----------|
| `$spacing-0` | 0 | 0px |
| `$spacing-1` | 0.25rem | 4px |
| `$spacing-2` | 0.5rem | 8px |
| `$spacing-3` | 0.75rem | 12px |
| `$spacing-4` | 1rem | 16px |
| `$spacing-5` | 1.5rem | 24px |
| `$spacing-6` | 2rem | 32px |
| `$spacing-8` | 3rem | 48px |
| `$spacing-10` | 4rem | 64px |
| `$spacing-12` | 6rem | 96px |

### Border Radius

| Token | Value | px Value |
|-------|-------|----------|
| `$radius-sm` | 0.25rem | 4px |
| `$radius-md` | 0.5rem | 8px |
| `$radius-lg` | 0.75rem | 12px |
| `$radius-xl` | 1rem | 16px |
| `$radius-full` | 9999px | Circular |

### Shadows

```scss
$shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.3);
$shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.4), 0 2px 4px -1px rgba(0, 0, 0, 0.3);
$shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.5), 0 4px 6px -2px rgba(0, 0, 0, 0.3);
$shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 10px 10px -5px rgba(0, 0, 0, 0.2);
$shadow-focus: 0 0 0 2px $color-bg-base, 0 0 0 4px $color-primary;
```

### Transitions

| Token | Value | Use Case |
|-------|-------|----------|
| `$transition-fast` | 150ms ease | Quick interactions |
| `$transition-base` | 250ms ease | Standard interactions |
| `$transition-medium` | 300ms ease | Smooth animations |
| `$transition-slow` | 400ms ease | Emphasized motion |

### Responsive Breakpoints

| Breakpoint | Value |
|------------|-------|
| Mobile | `768px` (max-width) |
| Desktop | `769px` (min-width) |

#### Container Widths
- **Constrained**: `1200px` max-width (forms, content)
- **Full-width**: `100%` (gallery, images)
- **Container Padding**: `16px` (1rem)

## Layout Patterns

### Full-Width Layout
Used for: Gallery grid, image displays
```scss
@include container-full();
```

### Constrained Layout
Used for: Forms, login, settings
```scss
@include container-constrained(); // 1200px max-width
@include container-constrained(32rem); // Custom max-width
```

### Hybrid Approach
The app uses both:
- **Full-width**: Gallery with image grid
- **Constrained**: Login/intro forms centered

## Component Library

### Available Components

#### Buttons
- Primary, Secondary, Ghost variants
- Danger, Success, Warning states
- Small, Medium, Large sizes
- Icon buttons
- Full-width buttons

#### Forms
- Text inputs, Textareas, Select dropdowns
- Checkboxes & Radio buttons
- Validation states (error, success)
- Input groups with icons
- Labels and hints

#### Dialogs
- Standard dialogs
- Full-screen dialogs
- Alert/Confirm dialogs
- Animated entrances

### Utility Classes

Basic utilities available in `_base.scss`:
- Display: `.flex`, `.grid`, `.block`, etc.
- Flexbox: `.flex-col`, `.items-center`, `.justify-between`
- Spacing: `.gap-2`, `.m-4`, `.p-6`, etc.
- Text: `.text-center`, `.text-primary`, `.font-bold`
- Visibility: `.hidden`, `.sr-only`

## Accessibility

### WCAG AA Compliance
- Minimum contrast ratio: **4.5:1**
- Text on dark backgrounds meets AA standards
- Semantic color usage for success/error states

### Focus Indicators
- Visible focus rings: **2px solid bottle-green**
- Focus offset: **2px**
- Box shadow for enhanced visibility
- All interactive elements have focus states

### Keyboard Navigation
- All buttons and inputs are keyboard accessible
- Focus management in dialogs
- Proper tab order throughout

## Best Practices

### Use Design Tokens
❌ **Don't:**
```scss
color: #e5e5e5;
margin: 16px;
```

✅ **Do:**
```scss
color: $color-text-primary;
margin: $spacing-4;
```

### Use Mixins for Common Patterns
❌ **Don't:**
```scss
button {
  padding: 8px 16px;
  border-radius: 8px;
  background: #2d5016;
  // ... many more lines
}
```

✅ **Do:**
```scss
button {
  @include button-primary();
}
```

### Responsive Design
Use the responsive mixins:
```scss
.component {
  padding: $spacing-4;

  @include mobile {
    padding: $spacing-2;
  }

  @include desktop {
    padding: $spacing-6;
  }
}
```

### Modern SCSS Syntax
Use `@use` instead of `@import`:
```scss
@use '../../theme/variables' as *;
@use '../../theme/mixins' as *;
```

## File Structure

```
src/theme/
├── _variables.scss    # Design tokens (colors, spacing, typography)
├── _mixins.scss       # Reusable SCSS mixins
├── _typography.scss   # Font imports and text styles
├── _base.scss         # CSS reset and utility classes
├── _buttons.scss      # Button component styles
├── _forms.scss        # Form control styles
├── _dialog.scss       # Modal/dialog styles
└── index.scss         # Main entry point
```

## Browser Support

- Modern evergreen browsers (Chrome, Firefox, Safari, Edge)
- CSS Grid and Flexbox support required
- CSS Custom Properties not used (SCSS variables instead)
- No IE11 support

## Contributing

When adding new components:
1. Use existing design tokens from `_variables.scss`
2. Create mixins for reusable patterns in `_mixins.scss`
3. Follow the established naming conventions
4. Ensure WCAG AA compliance (4.5:1 contrast)
5. Add both mobile and desktop responsive styles
6. Include focus states for interactive elements
7. Test keyboard navigation

## Resources

- [Roboto Font](https://fonts.google.com/specimen/Roboto)
- [WCAG Contrast Checker](https://webaim.org/resources/contrastchecker/)
- [Sass Documentation](https://sass-lang.com/documentation/)
