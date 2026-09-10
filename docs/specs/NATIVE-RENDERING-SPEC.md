# Qliphoth Native Rendering Backend Specification

**Version:** 0.2.0
**Date:** 2025-02-17
**Status:** Draft (Reviewed)
**SDD Phase:** Spec
**Parent Spec:** None

---

## Executive Summary

This specification defines the Native Rendering Backend for Qliphoth, enabling Sigil applications to run as native desktop applications without a browser runtime. The primary use case is Wraith IDE.

### Current State Analysis

| Component | Status | Notes |
|-----------|--------|-------|
| `RenderTarget::Native` | ✅ Defined | Enum variant exists in platform/mod.sigil:28-33 |
| `Platform::Native` | ✅ Defined | Added to enum, match arms updated |
| `NativePlatform` | ⚠️ Interface only | FFI declarations in platform/native.sigil |
| Native Runtime | ❌ Not implemented | Needs actual native graphics library |
| Layout Engine | ❌ Not implemented | Need taffy or similar for flexbox |
| Text Rendering | ❌ Not implemented | Need glyphon/cosmic-text |

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Sigil Application                            │
│                    (Wraith IDE, etc.)                           │
├─────────────────────────────────────────────────────────────────┤
│                    Qliphoth Framework                            │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐       │
│  │  VNode   │  Actor   │  Signal  │  Router  │  Effect  │       │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘       │
├─────────────────────────────────────────────────────────────────┤
│                      Platform Enum                               │
│      Browser(js)  │  Server(ssr)  │  Native(wgpu)               │
├─────────────────────────────────────────────────────────────────┤
│                  Native Runtime (Rust)                          │
│  ┌──────────┬──────────┬──────────┬──────────┬──────────┐       │
│  │  winit   │   wgpu   │  taffy   │ glyphon  │  Events  │       │
│  │ (window) │  (GPU)   │ (layout) │  (text)  │ (input)  │       │
│  └──────────┴──────────┴──────────┴──────────┴──────────┘       │
├─────────────────────────────────────────────────────────────────┤
│              Operating System / GPU Driver                       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 1. Conceptual Foundation

### 1.1 Design Philosophy

The native backend should:

1. **Preserve Qliphoth semantics** - VNodes, Actors, Signals work identically
2. **Be implementation-agnostic** - Sigil code doesn't know it's running native
3. **Support IDE requirements** - Fast text rendering, smooth scrolling, keyboard focus
4. **Be cross-platform** - Linux, Windows, macOS from same codebase

### 1.2 Why wgpu Over GTK

| Requirement | GTK | wgpu |
|-------------|-----|------|
| Consistent fonts | ❌ Platform fonts | ✅ Custom rendering |
| GPU acceleration | ⚠️ Limited | ✅ Full |
| Custom themes | ⚠️ CSS-like | ✅ Full control |
| Code editor rendering | ❌ Need separate widget | ✅ Direct control |
| Cross-platform consistency | ⚠️ Looks different | ✅ Identical |

**Decision:** Use wgpu for rendering, taffy for layout, glyphon for text.

### 1.3 Scope

**In Scope:**
- Window creation and management
- VNode → Native element mapping
- Flexbox layout (via taffy)
- Text rendering (via glyphon)
- Mouse and keyboard events
- Animation frames and timing

**Out of Scope (Future):**
- Accessibility (screen readers, etc.)
- Native file dialogs (use browser-style)
- System tray integration
- Multiple windows

---

## 2. Type Architecture

### 2.1 Core Types (Sigil Side)

```sigil
/// Opaque handle to native window
☉ Σ NativeWindow {
    handle: usize!
}

/// Opaque handle to native widget/element
☉ Σ NativeWidget {
    handle: usize!
}

/// Native event from the platform
☉ ᛈ NativeEvent {
    Click { x: f32, y: f32, button: MouseButton },
    KeyDown { key: KeyCode, modifiers: Modifiers },
    KeyUp { key: KeyCode, modifiers: Modifiers },
    TextInput { text: String },
    MouseMove { x: f32, y: f32 },
    Scroll { delta_x: f32, delta_y: f32 },
    Resize { width: u32, height: u32 },
    Focus,
    Blur,
    Close,
}

/// Mouse button enumeration
☉ ᛈ MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// Keyboard modifiers
☉ Σ Modifiers {
    shift: bool!
    ctrl: bool!
    alt: bool!
    meta: bool!  // Cmd on macOS, Win on Windows
}

/// Key codes (subset of winit VirtualKeyCode)
☉ ᛈ KeyCode {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // Numbers
    Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9,
    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // Navigation
    Up, Down, Left, Right, Home, End, PageUp, PageDown,
    // Editing
    Enter, Tab, Backspace, Delete, Insert, Escape,
    // Modifiers (as keys themselves)
    Shift, Ctrl, Alt, Meta,
    // Punctuation
    Space, Comma, Period, Semicolon, Quote, Slash, Backslash,
    BracketLeft, BracketRight, Minus, Equals, Grave,
    // Unknown/other
    Unknown(u32),
}

/// Computed layout for an element
☉ Σ Layout {
    x: f32!       // Position relative to parent
    y: f32!
    width: f32!   // Computed dimensions
    height: f32!
}

/// Pixel color (for test verification)
☉ Σ Pixel {
    r: u8!
    g: u8!
    b: u8!
    a: u8!
}

/// Event data structure returned from poll_event
☉ Σ NativeEventData {
    event_type: i32!      // -1 = no event, else EVENT_* constant
    callback_id: u64!     // Which listener to invoke
    // Click/Mouse data
    x: f32!
    y: f32!
    button: i32!          // MouseButton as int
    // Key data
    key: i32!             // KeyCode as int
    modifiers: Modifiers!
    // Text data
    text_ptr: *const i8!  // For TextInput events
    // Resize data
    width: u32!
    height: u32!
    // Scroll data
    delta_x: f32!
    delta_y: f32!
}
```

### 2.3 Event Type Constants

```sigil
// Event type codes (matches Appendix B)
☉ const EVENT_CLICK: i32 = 0;
☉ const EVENT_DBLCLICK: i32 = 1;
☉ const EVENT_MOUSEDOWN: i32 = 2;
☉ const EVENT_MOUSEUP: i32 = 3;
☉ const EVENT_MOUSEMOVE: i32 = 4;
☉ const EVENT_MOUSEENTER: i32 = 5;
☉ const EVENT_MOUSELEAVE: i32 = 6;
☉ const EVENT_KEYDOWN: i32 = 10;
☉ const EVENT_KEYUP: i32 = 11;
☉ const EVENT_TEXTINPUT: i32 = 12;
☉ const EVENT_FOCUS: i32 = 20;
☉ const EVENT_BLUR: i32 = 21;
☉ const EVENT_SCROLL: i32 = 30;
☉ const EVENT_RESIZE: i32 = 40;
☉ const EVENT_CLOSE: i32 = 50;
☉ const EVENT_ANIMATION_FRAME: i32 = 60;
☉ const EVENT_TIMEOUT: i32 = 61;

// Modifier flags
☉ const MODIFIER_NONE: i32 = 0;
☉ const MODIFIER_SHIFT: i32 = 1;
☉ const MODIFIER_CTRL: i32 = 2;
☉ const MODIFIER_ALT: i32 = 4;
☉ const MODIFIER_META: i32 = 8;
```

### 2.2 FFI Interface (Rust Side)

The native runtime exposes these C-compatible functions:

```rust
// Window management
extern "C" fn native_create_window(title: *const c_char, w: i32, h: i32) -> usize;
extern "C" fn native_destroy_window(handle: usize);
extern "C" fn native_window_size(handle: usize, w: *mut i32, h: *mut i32);

// Element creation
extern "C" fn native_create_element(window: usize, tag: *const c_char) -> usize;
extern "C" fn native_create_text(window: usize, content: *const c_char) -> usize;
extern "C" fn native_destroy_element(handle: usize);

// Element tree
extern "C" fn native_append_child(parent: usize, child: usize);
extern "C" fn native_remove_child(parent: usize, child: usize);
extern "C" fn native_insert_before(parent: usize, child: usize, before: usize);

// Attributes and styles
extern "C" fn native_set_attribute(elem: usize, name: *const c_char, value: *const c_char);
extern "C" fn native_set_text_content(elem: usize, content: *const c_char);
extern "C" fn native_set_style(elem: usize, property: *const c_char, value: *const c_char);

// Events
extern "C" fn native_add_event_listener(elem: usize, event_type: i32, callback_id: u64);
extern "C" fn native_remove_event_listener(elem: usize, event_type: i32, callback_id: u64);

// Event loop
extern "C" fn native_poll_event(out_event: *mut NativeEventData) -> i32;
extern "C" fn native_run_event_loop();

// Timing
extern "C" fn native_set_timeout(callback_id: u64, delay_ms: u64) -> u64;
extern "C" fn native_clear_timeout(timer_id: u64);
extern "C" fn native_request_animation_frame(callback_id: u64) -> u64;
extern "C" fn native_cancel_animation_frame(frame_id: u64);
extern "C" fn native_now_ms() -> u64;  // Current timestamp in milliseconds

// Window content
extern "C" fn native_set_root(window: usize, element: usize);  // Set root element
extern "C" fn native_get_root(window: usize) -> usize;         // Get root element

// Layout queries
extern "C" fn native_get_layout(elem: usize, out_layout: *mut Layout);
extern "C" fn native_compute_layout(window: usize);  // Force layout computation

// Content queries
extern "C" fn native_get_text_content(elem: usize, out_buf: *mut c_char, buf_len: usize) -> usize;
extern "C" fn native_get_child_count(elem: usize) -> usize;
extern "C" fn native_get_child_at(elem: usize, index: usize) -> usize;

// Focus management
extern "C" fn native_focus(elem: usize);
extern "C" fn native_blur(elem: usize);
extern "C" fn native_get_focused(window: usize) -> usize;

// Event loop variants
extern "C" fn native_poll_events();  // Process all pending events (non-blocking)
extern "C" fn native_poll_event_timeout(timeout_ms: u64, out_event: *mut NativeEventData) -> i32;

// Test infrastructure (may be compiled out in release)
#[cfg(test)]
extern "C" fn native_simulate_click(window: usize, x: f32, y: f32);
#[cfg(test)]
extern "C" fn native_simulate_key(window: usize, key: i32, modifiers: i32);
#[cfg(test)]
extern "C" fn native_simulate_text_input(window: usize, text: *const c_char);
#[cfg(test)]
extern "C" fn native_simulate_mouse_move(window: usize, x: f32, y: f32);
#[cfg(test)]
extern "C" fn native_simulate_scroll(window: usize, delta_x: f32, delta_y: f32);
#[cfg(test)]
extern "C" fn native_sample_pixel(window: usize, x: i32, y: i32, out_pixel: *mut Pixel);
#[cfg(test)]
extern "C" fn native_has_pixels_matching(window: usize, r_min: u8, r_max: u8,
                                          g_min: u8, g_max: u8, b_min: u8, b_max: u8) -> i32;
```

---

## 3. Behavioral Contracts

### 3.1 Window Lifecycle

**Invariant:** A window handle is valid from `native_create_window` until `native_destroy_window`.

**Contract:**
```
create_window(title, width, height):
    PRE:  width > 0 ∧ height > 0
    POST: handle > 0
    POST: window is visible on screen
    POST: window has specified dimensions (may be adjusted by OS)

destroy_window(handle):
    PRE:  handle was returned by create_window
    POST: window is closed
    POST: handle is invalid (subsequent calls are no-op)
```

### 3.2 Element Tree

**Invariant:** Elements form a tree rooted at the window's root container.

**Contract:**
```
create_element(window, tag):
    PRE:  window handle is valid
    POST: element handle > 0
    POST: element is detached (no parent)

append_child(parent, child):
    PRE:  parent handle is valid
    PRE:  child handle is valid
    PRE:  child has no parent OR child.parent == parent (move within same parent)
    POST: child.parent == parent
    POST: child is last in parent.children

remove_child(parent, child):
    PRE:  parent handle is valid
    PRE:  child.parent == parent
    POST: child has no parent
    POST: child not in parent.children
```

### 3.3 Layout Computation

**Invariant:** Layout is computed on demand, not on every tree modification.

**Contract:**
```
set_style(element, property, value):
    POST: element.styles[property] = value
    POST: layout is marked dirty (not recomputed)

compute_layout(root, available_width, available_height):
    PRE:  root is valid element
    POST: all descendants have computed position (x, y) and size (w, h)
    POST: layout respects flexbox rules per CSS Flexbox spec
```

### 3.4 Event Dispatch

**Invariant:** Events are dispatched to listeners in registration order.

**Contract:**
```
add_event_listener(element, event_type, callback_id):
    POST: callback_id is associated with (element, event_type)
    POST: future events of event_type on element will include callback_id

poll_event():
    IF event queue is empty:
        RETURN -1 (no event)
    ELSE:
        event ← dequeue()
        RETURN event_type_code
        out_event filled with event data

dispatch_event(event):
    target ← hit_test(event.position)  // for positional events
    FOR listener IN target.listeners[event.type]:
        enqueue_callback(listener.callback_id, event)
```

### 3.5 Rendering

**Invariant:** Frame rendering happens on animation frame request.

**Contract:**
```
request_animation_frame(callback_id):
    POST: frame_id > 0
    POST: callback_id will be invoked before next vsync

render_frame():
    compute_layout(root)
    FOR element IN depth_first_traversal(root):
        render_background(element)
        render_border(element)
        render_text(element)  // if text content
        render_children(element)
```

### 3.6 Coordinate System

**Invariant:** All coordinates are in logical pixels, origin top-left.

**Contract:**
```
Coordinate System:
    - Window origin is (0, 0) at top-left corner
    - X increases rightward
    - Y increases downward
    - Layout positions are relative to parent element
    - Event coordinates are relative to window
    - Pixel sampling coordinates are absolute to window
```

### 3.7 Focus Management

**Invariant:** At most one element has focus per window.

**Contract:**
```
focus(element):
    PRE:  element handle is valid
    POST: element receives focus (if focusable)
    POST: previously focused element receives Blur event
    POST: element receives Focus event

blur(element):
    PRE:  element handle is valid
    POST: element loses focus
    POST: element receives Blur event
    POST: window has no focused element

get_focused(window):
    RETURN currently focused element handle, or 0 if none
```

### 3.8 Root Element

**Invariant:** Each window has exactly one root element.

**Contract:**
```
set_root(window, element):
    PRE:  window handle is valid
    PRE:  element handle is valid
    POST: element is the root of window's element tree
    POST: element fills the window (layout computed to window size)

get_root(window):
    PRE:  window handle is valid
    RETURN root element handle, or 0 if not set
```

### 3.9 Event Bubbling

**Invariant:** Events bubble up from target to root, unless stopped.

**Contract:**
```
event_dispatch(event):
    target ← hit_test(event.position)  // deepest element at position
    current ← target
    WHILE current != null AND event.propagation_stopped == false:
        FOR listener IN current.listeners[event.type]:
            invoke(listener, event)
        current ← current.parent
```

**Note:** Phase 1 does NOT implement stopPropagation. All events bubble to root.

---

## 4. Constraints & Invariants

### 4.1 Memory Safety

- All handles are opaque integers; actual objects live in Rust
- Sigil code cannot forge handles or access freed memory
- Double-free is a no-op (handle becomes invalid after first free)

### 4.2 Thread Safety

- All FFI calls must happen from the main thread
- The event loop runs on the main thread
- Background work (if any) must use channels to communicate

### 4.3 Performance Targets

| Operation | Target | Notes |
|-----------|--------|-------|
| Frame render (simple) | < 8ms | 120fps capable |
| Frame render (1000 elements) | < 16ms | 60fps minimum |
| Layout computation | < 2ms | Per frame budget |
| Text shaping | < 1ms | Per paragraph |
| Event dispatch | < 1ms | Including hit testing |

### 4.4 CSS Property Support (Phase 1)

| Property | Support | Notes |
|----------|---------|-------|
| `display` | flex, none | No grid in Phase 1 |
| `flex-direction` | row, column, row-reverse, column-reverse | |
| `justify-content` | flex-start, flex-end, center, space-between, space-around | |
| `align-items` | flex-start, flex-end, center, stretch | |
| `width`, `height` | px, %, auto | |
| `margin`, `padding` | px, % | |
| `gap` | px | |
| `background-color` | hex, named | |
| `color` | hex, named | |
| `font-size` | px | |
| `border-radius` | px | |
| `overflow` | hidden, scroll | visible is hidden |

### 4.5 Default Styles

Elements have these defaults when created:

| Property | Default Value | Notes |
|----------|---------------|-------|
| `display` | flex | All elements are flex containers |
| `flex-direction` | column | Vertical stacking by default |
| `width` | auto | Shrink to content |
| `height` | auto | Shrink to content |
| `background-color` | transparent | |
| `color` | #000000 | Black text |
| `font-size` | 16px | Base text size |
| `margin` | 0 | No margin |
| `padding` | 0 | No padding |
| `gap` | 0 | No gap |
| `border-radius` | 0 | Square corners |

---

## 5. Error Conditions

### 5.1 Recoverable Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| Invalid handle | Using freed element | Return silently (no-op) |
| Invalid parent | Append to non-container | Return silently |
| Style parse error | Invalid CSS value | Use default value |

### 5.2 Fatal Errors

| Error | Cause | Action |
|-------|-------|--------|
| GPU device lost | Driver crash | Attempt recreation, then exit |
| Out of memory | Allocation failed | Exit with error code |
| Window creation failed | No display | Exit with error code |

---

## 6. Integration Points

### 6.1 Sigil Compiler Integration

The compiler must:
1. Link against `qliphoth-native-wgpu` library when `target = "native"`
2. Generate FFI calls for platform methods
3. Bundle font assets with the binary

### 6.2 Qliphoth Framework Integration

`Platform::native()` must:
1. Create NativePlatform instance
2. Call `native_create_window` with app title
3. Return Platform::Native(instance)

VNode diffing must:
1. Use `native_create_element` for new nodes
2. Use `native_set_attribute` for changed attributes
3. Use `native_remove_child` / `native_append_child` for tree changes

---

## 7. Open Questions

### 7.1 Resolved

- **Q:** GTK vs wgpu? **A:** wgpu for consistency and control

### 7.2 Unresolved

- **Q:** How to handle system clipboard? (need platform-specific code)
- **Q:** How to handle DPI scaling? (query from OS, scale layout)
- **Q:** How to bundle fonts? (embed in binary vs load from disk)
- **Q:** How to handle IME for CJK input?

---

## 8. Implementation Phases

### Phase 1: Minimal Viable Native (This Spec)

- [ ] Window creation with wgpu surface
- [ ] Element tree with basic layout (taffy)
- [ ] Rectangle rendering (backgrounds, borders)
- [ ] Text rendering (glyphon)
- [ ] Mouse events (click, move)
- [ ] Keyboard events
- [ ] Animation frames

**Success Criteria:** Render a simple counter app (button + text).

### Phase 2: Wraith Requirements

- [ ] Scrolling containers
- [ ] Text selection
- [ ] Clipboard integration
- [ ] Focus management
- [ ] Cursor styles

**Success Criteria:** Render Wraith IDE with basic functionality.

### Phase 3: Polish

- [ ] Smooth scrolling
- [ ] Animations/transitions
- [ ] DPI awareness
- [ ] IME support
- [ ] Accessibility basics

---

## 9. Test Strategy (Agent-TDD)

### 9.1 Specification Tests (Phase 1)

```sigil
// Window lifecycle
fn spec_create_window_returns_valid_handle()
fn spec_destroy_window_closes_window()
fn spec_window_size_matches_requested()

// Element tree
fn spec_create_element_returns_valid_handle()
fn spec_append_child_adds_to_parent()
fn spec_remove_child_detaches_from_parent()

// Layout
fn spec_flex_row_distributes_horizontally()
fn spec_flex_column_distributes_vertically()
fn spec_justify_content_center_centers_children()

// Events
fn spec_click_dispatches_to_target()
fn spec_keydown_includes_modifiers()

// Rendering
fn spec_background_color_renders_correctly()
fn spec_text_content_renders_readable()
```

### 9.2 Property Tests

```sigil
fn property_layout_children_fit_in_parent<N: Layout>(node: N)
fn property_event_dispatch_respects_order<E: Event>(events: Vec<E>)
fn property_style_changes_mark_dirty<S: Style>(style: S)
```

### 9.3 Integration Tests

```sigil
fn integration_counter_app_increments()
fn integration_list_scrolls_smoothly()
fn integration_text_input_receives_keys()
```

---

## Appendix A: VNode to Native Mapping

| VNode Tag | Native Behavior |
|-----------|-----------------|
| `div` | Flex container (column) |
| `span` | Flex container (row) |
| `button` | Clickable, hover state |
| `input` | Text input, focus |
| `textarea` | Multi-line text input |
| `p`, `h1`-`h6` | Text block |
| `img` | Image rendering |
| `svg` | Vector rendering (future) |

---

## Appendix B: Event Type Codes

| Code | Event Type |
|------|------------|
| 0 | Click |
| 1 | DblClick |
| 2 | MouseDown |
| 3 | MouseUp |
| 4 | MouseMove |
| 5 | MouseEnter |
| 6 | MouseLeave |
| 10 | KeyDown |
| 11 | KeyUp |
| 12 | TextInput |
| 20 | Focus |
| 21 | Blur |
| 30 | Scroll |
| 40 | Resize |
| 50 | Close |

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 0.1.0 | 2025-02-17 | Initial draft |
| 0.2.0 | 2025-02-17 | Added: KeyCode enum, Layout struct, Pixel struct, NativeEventData struct, event constants, modifier flags. Added FFI: set_root, get_root, get_layout, compute_layout, get_text_content, get_child_count, get_child_at, focus, blur, get_focused, poll_events, poll_event_timeout, now_ms, test simulation functions. Added contracts: coordinate system, focus management, root element, event bubbling. Added default styles table. |
