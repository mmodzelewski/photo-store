# Design System Usage Examples

Practical examples for using the Photo Store Design System components and patterns.

## Table of Contents

- [Buttons](#buttons)
- [Forms](#forms)
- [Dialogs](#dialogs)
- [Page Containers](#page-containers)
- [Cards](#cards)
- [Images & Thumbnails](#images--thumbnails)
- [Typography](#typography)
- [Layout Utilities](#layout-utilities)

---

## Buttons

### Basic Buttons

```html
<!-- Primary Button (Main actions) -->
<button class="btn btn-primary">Save Changes</button>

<!-- Secondary Button (Alternative actions) -->
<button class="btn btn-secondary">Cancel</button>

<!-- Ghost Button (Subtle actions) -->
<button class="btn btn-ghost">View Details</button>
```

### Button Sizes

```html
<!-- Small -->
<button class="btn btn-primary btn-sm">Small</button>

<!-- Medium (default) -->
<button class="btn btn-primary btn-md">Medium</button>

<!-- Large -->
<button class="btn btn-primary btn-lg">Large</button>
```

### Button States

```html
<!-- Disabled -->
<button class="btn btn-primary" disabled>Disabled</button>

<!-- Semantic States -->
<button class="btn btn-success">Upload Complete</button>
<button class="btn btn-warning">Proceed with Caution</button>
<button class="btn btn-danger">Delete Photo</button>
```

### Full Width & Icon Buttons

```html
<!-- Full Width Button -->
<button class="btn btn-primary btn-full">Continue</button>

<!-- Icon Button -->
<button class="btn btn-secondary btn-icon">
  <svg><!-- icon --></svg>
</button>
```

### Button Groups

```html
<!-- Spaced Button Group -->
<div class="btn-group">
  <button class="btn btn-secondary">Previous</button>
  <button class="btn btn-primary">Next</button>
</div>

<!-- Attached Button Group -->
<div class="btn-group btn-group-attached">
  <button class="btn btn-secondary">Day</button>
  <button class="btn btn-secondary">Week</button>
  <button class="btn btn-primary">Month</button>
</div>

<!-- Full Width Group -->
<div class="btn-group btn-group-full">
  <button class="btn btn-secondary">Cancel</button>
  <button class="btn btn-primary">Confirm</button>
</div>
```

### SCSS Implementation

```scss
// Using mixins in component styles
.custom-button {
  @include button-primary();

  // Add custom overrides
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

// Or use the classes directly in HTML
```

---

## Forms

### Basic Form Structure

```html
<form class="form">
  <!-- Form Group with Label -->
  <div class="form-group">
    <label for="name" class="form-label">Name</label>
    <input type="text" id="name" class="form-input" placeholder="Enter your name">
  </div>

  <!-- Required Field -->
  <div class="form-group">
    <label for="email" class="form-label required">Email</label>
    <input type="email" id="email" class="form-input" placeholder="you@example.com">
    <small class="form-hint">We'll never share your email.</small>
  </div>

  <!-- Form Actions -->
  <div class="form-actions form-actions-end">
    <button type="button" class="btn btn-secondary">Cancel</button>
    <button type="submit" class="btn btn-primary">Submit</button>
  </div>
</form>
```

### Input Variations

```html
<!-- Text Input -->
<input type="text" class="form-input" placeholder="Username">

<!-- Password Input -->
<input type="password" class="form-input" placeholder="Password">

<!-- Textarea -->
<textarea class="form-textarea" placeholder="Description" rows="4"></textarea>

<!-- Select Dropdown -->
<select class="form-select">
  <option>Choose an option</option>
  <option>Option 1</option>
  <option>Option 2</option>
</select>
```

### Input Sizes

```html
<!-- Small -->
<input type="text" class="form-input form-input-sm" placeholder="Small">

<!-- Medium (default) -->
<input type="text" class="form-input" placeholder="Medium">

<!-- Large -->
<input type="text" class="form-input form-input-lg" placeholder="Large">
```

### Validation States

```html
<!-- Error State -->
<div class="form-group">
  <label class="form-label">Username</label>
  <input type="text" class="form-input is-error" value="ab">
  <small class="form-error">Username must be at least 3 characters</small>
</div>

<!-- Success State -->
<div class="form-group">
  <label class="form-label">Email</label>
  <input type="email" class="form-input is-success" value="user@example.com">
  <small class="form-success">Email is available!</small>
</div>
```

### Checkboxes & Radio Buttons

```html
<!-- Checkbox -->
<div class="form-check">
  <input type="checkbox" id="terms" class="form-checkbox">
  <label for="terms">I agree to the terms and conditions</label>
</div>

<!-- Radio Buttons -->
<div class="form-group">
  <label class="form-label">Choose a size</label>
  <div class="form-check">
    <input type="radio" id="small" name="size" class="form-radio">
    <label for="small">Small</label>
  </div>
  <div class="form-check">
    <input type="radio" id="medium" name="size" class="form-radio" checked>
    <label for="medium">Medium</label>
  </div>
  <div class="form-check">
    <input type="radio" id="large" name="size" class="form-radio">
    <label for="large">Large</label>
  </div>
</div>
```

### Input with Icons

```html
<div class="input-group has-icon-left">
  <span class="input-icon icon-left">🔍</span>
  <input type="text" class="form-input" placeholder="Search photos...">
</div>

<div class="input-group has-icon-right">
  <input type="text" class="form-input" placeholder="Enter URL">
  <span class="input-icon icon-right">🔗</span>
</div>
```

### File Upload

```html
<input type="file" class="form-file" accept="image/*">
```

### SCSS Implementation

```scss
// Custom styled input in component
input[type="password"] {
  @include input-base();

  // Custom styles
  letter-spacing: 0.1em;
}
```

---

## Dialogs

### Basic Dialog

```html
<dialog id="myDialog">
  <div class="dialog-content">
    <h3>Dialog Title</h3>
    <p>This is the dialog content.</p>
    <button onclick="myDialog.close()">Close</button>
  </div>
</dialog>
```

**TypeScript/JavaScript:**
```typescript
const dialog = document.getElementById('myDialog') as HTMLDialogElement;
dialog.showModal();
```

### Standard Dialog with Header and Footer

```html
<dialog id="confirmDialog">
  <div class="dialog-content">
    <div class="dialog-header">
      <h3 class="dialog-title">Confirm Action</h3>
      <button class="dialog-close" onclick="confirmDialog.close()">✕</button>
    </div>

    <div class="dialog-body">
      <p>Are you sure you want to delete this photo? This action cannot be undone.</p>
    </div>

    <div class="dialog-footer">
      <button class="btn btn-secondary" onclick="confirmDialog.close()">Cancel</button>
      <button class="btn btn-danger">Delete</button>
    </div>
  </div>
</dialog>
```

### Dialog Sizes

```html
<!-- Small Dialog -->
<dialog>
  <div class="dialog-content dialog-sm">
    <h3>Small Dialog</h3>
    <p>Quick confirmation</p>
  </div>
</dialog>

<!-- Medium Dialog (default) -->
<dialog>
  <div class="dialog-content dialog-md">
    <h3>Medium Dialog</h3>
  </div>
</dialog>

<!-- Large Dialog -->
<dialog>
  <div class="dialog-content dialog-lg">
    <h3>Large Dialog</h3>
    <p>Lots of content here...</p>
  </div>
</dialog>

<!-- Full Screen Dialog -->
<dialog>
  <div class="dialog-content dialog-full">
    <!-- Full screen content -->
  </div>
</dialog>
```

### Alert Dialog

```html
<dialog>
  <div class="dialog-content dialog-alert">
    <div class="dialog-header">
      <h3 class="dialog-title">Error</h3>
    </div>
    <div class="dialog-body">
      <p>An error occurred while uploading your photo.</p>
    </div>
    <div class="dialog-footer">
      <button class="btn btn-primary">OK</button>
    </div>
  </div>
</dialog>
```

### Loading/Progress Dialog

```html
<dialog>
  <div class="dialog-content loading">
    <div class="loading-text">Processing images...</div>
    <p>45 / 100</p>
  </div>
</dialog>
```

### SCSS Implementation

```scss
// Component-specific dialog styling
dialog {
  border: none;
  padding: 0;
  background: transparent;
  max-width: none;
  max-height: none;

  &::backdrop {
    background-color: $color-bg-overlay;
    backdrop-filter: blur(2px);
  }

  .dialog-content {
    background-color: $color-bg-elevated;
    border-radius: $radius-lg;
    box-shadow: $shadow-xl;
    max-width: 28rem;
    padding: $spacing-6;

    // Add custom styles
  }
}
```

---

## Page Containers

### Constrained Container (Forms, Settings)

```html
<!-- HTML Structure -->
<div class="page-wrapper">
  <div class="container">
    <h1>Settings</h1>
    <form class="form">
      <!-- Form content -->
    </form>
  </div>
</div>
```

**SCSS:**
```scss
:host {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: $spacing-4;
  background-color: $color-bg-base;
}

.container {
  @include card();
  width: 100%;
  max-width: 32rem;
  padding: $spacing-8;

  @include mobile {
    padding: $spacing-6;
  }
}
```

### Full-Width Container (Gallery)

```html
<div class="gallery-wrapper">
  <div class="gallery-header">
    <h2>Gallery</h2>
    <button class="btn btn-primary">Sync Images</button>
  </div>

  <div class="gallery-grid">
    <!-- Grid items -->
  </div>
</div>
```

**SCSS:**
```scss
:host {
  display: block;
  width: 100%;
  min-height: 100vh;
  padding: $spacing-6 $spacing-4;
}

.gallery-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: $spacing-6;
  max-width: 1920px;
  margin-left: auto;
  margin-right: auto;
}

.gallery-grid {
  display: grid;
  gap: $spacing-4;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  max-width: 1920px;
  margin: 0 auto;

  @include mobile {
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: $spacing-3;
  }
}
```

### Centered Content Page

```html
<div class="centered-page">
  <div class="content-card">
    <h1>Welcome</h1>
    <p>Get started by selecting a directory.</p>
    <button class="btn btn-primary btn-full">Select Directory</button>
  </div>
</div>
```

**SCSS:**
```scss
.centered-page {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: $spacing-4;
  background-color: $color-bg-base;
}

.content-card {
  @include card();
  width: 100%;
  max-width: 32rem;
  padding: $spacing-8;
  text-align: center;
}
```

---

## Cards

### Basic Card

```html
<div class="card">
  <h3>Card Title</h3>
  <p>Card content goes here.</p>
</div>
```

**SCSS:**
```scss
.card {
  @include card();
}
```

### Interactive Card

```html
<button class="photo-card">
  <img src="photo.jpg" alt="Photo">
  <div class="card-info">
    <h4>Photo Title</h4>
    <p class="text-muted">Date taken</p>
  </div>
</button>
```

**SCSS:**
```scss
.photo-card {
  @include card-interactive();

  .card-info {
    padding: $spacing-3;
  }
}
```

---

## Images & Thumbnails

### Image Thumbnail with Hover Effect

```html
<button class="image-thumbnail">
  <img src="thumbnail.jpg" alt="Photo thumbnail">
</button>
```

**SCSS:**
```scss
.image-thumbnail {
  @include card-interactive();
  width: 100%;
  padding: 0;
  overflow: hidden;
  aspect-ratio: 1 / 1;

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform $transition-medium;
  }

  &:hover img {
    transform: scale(1.05);
  }
}
```

### Responsive Image Grid

```html
<div class="image-grid">
  <div class="image-item">
    <img src="photo1.jpg" alt="Photo 1">
  </div>
  <div class="image-item">
    <img src="photo2.jpg" alt="Photo 2">
  </div>
  <!-- More items -->
</div>
```

**SCSS:**
```scss
.image-grid {
  display: grid;
  gap: $spacing-4;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));

  @include mobile {
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: $spacing-3;
  }
}

.image-item {
  @include aspect-ratio(1, 1);

  img {
    object-fit: cover;
    border-radius: $radius-md;
  }
}
```

### Image with Loading Placeholder

```html
<div class="image-wrapper">
  <img src="photo.jpg" alt="Photo" loading="lazy">
</div>
```

**SCSS:**
```scss
.image-wrapper {
  position: relative;
  background-color: $color-bg-elevated;
  border-radius: $radius-md;
  overflow: hidden;

  // Loading state
  &:empty::before {
    content: '';
    display: block;
    width: 3rem;
    height: 3rem;
    margin: auto;
    border: 3px solid $color-border;
    border-top-color: $color-primary;
    border-radius: $radius-full;
    animation: spinner 0.8s linear infinite;
  }
}

@keyframes spinner {
  to { transform: rotate(360deg); }
}
```

---

## Typography

### Headings

```html
<h1>Main Page Title</h1>
<h2>Section Heading</h2>
<h3>Subsection Heading</h3>
<h4>Card Title</h4>
<h5>Small Heading</h5>
<h6>Tiny Heading</h6>
```

### Text Styles

```html
<!-- Primary Text -->
<p class="text-primary">This is primary text</p>

<!-- Secondary Text -->
<p class="text-secondary">This is secondary text</p>

<!-- Muted Text -->
<p class="text-muted">This is muted text</p>

<!-- Semantic Colors -->
<p class="text-success">Success message</p>
<p class="text-warning">Warning message</p>
<p class="text-error">Error message</p>
```

### Font Sizes

```html
<p class="text-xs">Extra small text</p>
<p class="text-sm">Small text</p>
<p class="text-base">Base text</p>
<p class="text-lg">Large text</p>
<p class="text-xl">Extra large text</p>
<p class="text-2xl">2XL text</p>
<p class="text-3xl">3XL text</p>
<p class="text-4xl">4XL text</p>
```

### Font Weights

```html
<p class="font-normal">Normal weight</p>
<p class="font-medium">Medium weight</p>
<p class="font-bold">Bold weight</p>
```

### Text Alignment

```html
<p class="text-left">Left aligned</p>
<p class="text-center">Center aligned</p>
<p class="text-right">Right aligned</p>
```

---

## Layout Utilities

### Flexbox

```html
<!-- Basic Flex Container -->
<div class="flex gap-4">
  <div>Item 1</div>
  <div>Item 2</div>
</div>

<!-- Flex Column -->
<div class="flex flex-col gap-4">
  <div>Item 1</div>
  <div>Item 2</div>
</div>

<!-- Centered Content -->
<div class="flex items-center justify-center" style="height: 100vh;">
  <div>Centered Content</div>
</div>

<!-- Space Between -->
<div class="flex items-center justify-between">
  <div>Left</div>
  <div>Right</div>
</div>
```

### Grid

```html
<div class="grid gap-4" style="grid-template-columns: repeat(3, 1fr);">
  <div>Column 1</div>
  <div>Column 2</div>
  <div>Column 3</div>
</div>
```

### Spacing

```html
<!-- Margin -->
<div class="m-4">All sides margin</div>
<div class="mt-4">Top margin</div>
<div class="mb-4">Bottom margin</div>
<div class="ml-auto">Left auto margin (push right)</div>

<!-- Padding -->
<div class="p-4">All sides padding</div>
<div class="pt-6">Top padding</div>
<div class="pb-6">Bottom padding</div>
```

### Display

```html
<div class="hidden">Hidden element</div>
<div class="block">Block element</div>
<div class="flex">Flex container</div>
<div class="inline-flex">Inline flex</div>
<div class="grid">Grid container</div>
```

---

## Complete Page Examples

### Login Page

```html
<div class="login-page">
  <div class="login-container">
    <h1 class="login-title">Photo Store</h1>
    <p class="text-secondary mb-6">Sign in to your account</p>

    <form class="form">
      <div class="form-group">
        <label for="username" class="form-label">Username</label>
        <input type="text" id="username" class="form-input" placeholder="Enter username">
      </div>

      <div class="form-group">
        <label for="password" class="form-label">Password</label>
        <input type="password" id="password" class="form-input" placeholder="Enter password">
      </div>

      <button type="submit" class="btn btn-primary btn-full">Login</button>
    </form>
  </div>
</div>
```

### Gallery Page

```html
<div class="gallery-page">
  <div class="gallery-header">
    <h2>My Photos</h2>
    <button class="btn btn-primary">Sync Images</button>
  </div>

  <div class="gallery-grid">
    <button class="image-thumbnail">
      <img src="photo1.jpg" alt="Photo 1">
    </button>
    <button class="image-thumbnail">
      <img src="photo2.jpg" alt="Photo 2">
    </button>
    <!-- More thumbnails -->
  </div>
</div>
```

---

## Tips & Best Practices

### 1. Always Use Design Tokens
```scss
// ❌ Don't
.button { background: #2d5016; margin: 16px; }

// ✅ Do
.button { background: $color-primary; margin: $spacing-4; }
```

### 2. Use Mixins for Consistency
```scss
// ❌ Don't repeat styles
.button1 { padding: 8px 16px; border-radius: 8px; /* ... */ }
.button2 { padding: 8px 16px; border-radius: 8px; /* ... */ }

// ✅ Use mixins
.button1 { @include button-primary(); }
.button2 { @include button-secondary(); }
```

### 3. Mobile-First Responsive Design
```scss
.component {
  // Base (mobile) styles
  padding: $spacing-2;

  // Desktop enhancements
  @include desktop {
    padding: $spacing-6;
  }
}
```

### 4. Accessible Focus States
Always ensure interactive elements have visible focus states:
```scss
button {
  @include focus-visible(); // Automatically adds focus ring
}
```

### 5. Semantic HTML
Use appropriate HTML elements for better accessibility:
```html
<!-- ✅ Good -->
<button class="btn btn-primary">Click me</button>

<!-- ❌ Avoid -->
<div class="btn btn-primary" onclick="...">Click me</div>
```
