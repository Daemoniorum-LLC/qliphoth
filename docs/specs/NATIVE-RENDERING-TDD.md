# Native Rendering Backend - Agent-TDD Roadmap

**Spec:** NATIVE-RENDERING-SPEC.md v0.3.0
**Date:** 2025-02-17
**Status:** ✅ Phases 10-12 GREEN (141/141 tests passing)
**Reviewed:** 2025-02-17 - Added stronger assertions, more tests
**Updated:** 2026-02-20 - Phases 10-12 implemented and GREEN: monospace font, text span API, GPU mode API

---

## Philosophy

Tests are crystallized understanding. Each test answers: "How do we know this is correct?"

We test **compliance** (observable behavior), not **conformance** (implementation details).

---

## Phase 1: Window Management

### Specification Tests

```sigil
/// A window can be created with title and dimensions
fn spec_create_window_returns_nonzero_handle() {
    ≔ handle = native_create_window("Test", 800, 600);
    assert(handle > 0, "Window handle must be positive");
}

/// Created window has requested dimensions (or OS-adjusted)
fn spec_window_size_is_reasonable() {
    ≔ handle = native_create_window("Test", 800, 600);
    ≔ (w, h) = native_window_size(handle);
    assert(w >= 100 ∧ w <= 10000, "Width in reasonable range");
    assert(h >= 100 ∧ h <= 10000, "Height in reasonable range");
    native_destroy_window(handle);
}

/// Destroyed window handle becomes invalid (no crash on reuse)
fn spec_destroy_window_invalidates_handle() {
    ≔ handle = native_create_window("Test", 800, 600);
    native_destroy_window(handle);
    // Subsequent operations should be no-op, not crash
    ≔ (w, h) = native_window_size(handle);
    assert(w == 0 ∧ h == 0, "Invalid handle returns zero size");
}
```

**Criteria:** All 3 tests pass.

---

## Phase 2: Element Creation

### Specification Tests

```sigil
/// Elements can be created with various tags
fn spec_create_element_div() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ elem = native_create_element(win, "div");
    assert(elem > 0, "Element handle must be positive");
    native_destroy_window(win);
}

/// Text nodes can be created
fn spec_create_text_node() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ text = native_create_text(win, "Hello, World!");
    assert(text > 0, "Text handle must be positive");
    native_destroy_window(win);
}

/// Destroyed element handle is safe to reuse operations on
fn spec_destroy_element_safe() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ elem = native_create_element(win, "div");
    native_destroy_element(elem);
    // Should not crash
    native_set_attribute(elem, "class", "test");
    native_destroy_window(win);
}
```

**Criteria:** All 3 tests pass.

---

## Phase 3: Element Tree

### Specification Tests

```sigil
/// Child can be appended to parent
fn spec_append_child_succeeds() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    ≔ child = native_create_element(win, "span");

    assert(native_get_child_count(parent) == 0, "Initially no children");
    native_append_child(parent, child);
    assert(native_get_child_count(parent) == 1, "Should have one child");
    assert(native_get_child_at(parent, 0) == child, "Child should be retrievable");

    native_destroy_window(win);
}

/// Child can be removed from parent
fn spec_remove_child_succeeds() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    ≔ child = native_create_element(win, "span");

    native_append_child(parent, child);
    assert(native_get_child_count(parent) == 1, "Should have one child");

    native_remove_child(parent, child);
    assert(native_get_child_count(parent) == 0, "Should have no children after removal");

    native_destroy_window(win);
}

/// Multiple children maintain order
fn spec_children_maintain_order() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    ≔ child1 = native_create_element(win, "span");
    ≔ child2 = native_create_element(win, "span");
    ≔ child3 = native_create_element(win, "span");

    native_append_child(parent, child1);
    native_append_child(parent, child2);
    native_append_child(parent, child3);

    assert(native_get_child_count(parent) == 3, "Should have three children");
    assert(native_get_child_at(parent, 0) == child1, "First child is child1");
    assert(native_get_child_at(parent, 1) == child2, "Second child is child2");
    assert(native_get_child_at(parent, 2) == child3, "Third child is child3");

    native_destroy_window(win);
}

/// insert_before places child at correct position
fn spec_insert_before_correct_position() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    ≔ child1 = native_create_element(win, "span");
    ≔ child2 = native_create_element(win, "span");
    ≔ child3 = native_create_element(win, "span");

    native_append_child(parent, child1);
    native_append_child(parent, child3);
    native_insert_before(parent, child2, child3);  // Insert child2 before child3

    assert(native_get_child_count(parent) == 3, "Should have three children");
    assert(native_get_child_at(parent, 0) == child1, "First is child1");
    assert(native_get_child_at(parent, 1) == child2, "Second is child2 (inserted)");
    assert(native_get_child_at(parent, 2) == child3, "Third is child3");

    native_destroy_window(win);
}
```

**Criteria:** All 4 tests pass.

---

## Phase 4: Flexbox Layout

### Specification Tests

```sigil
/// Flex row distributes children horizontally
fn spec_flex_row_horizontal() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "display", "flex");
    native_set_style(parent, "flex-direction", "row");
    native_set_style(parent, "width", "300px");
    native_set_style(parent, "height", "100px");

    ≔ child1 = native_create_element(win, "div");
    native_set_style(child1, "width", "100px");
    native_set_style(child1, "height", "100px");

    ≔ child2 = native_create_element(win, "div");
    native_set_style(child2, "width", "100px");
    native_set_style(child2, "height", "100px");

    native_append_child(parent, child1);
    native_append_child(parent, child2);

    ≔ layout1 = native_get_layout(child1);
    ≔ layout2 = native_get_layout(child2);

    assert(layout1.x < layout2.x, "Child2 should be right of Child1");
    assert(layout1.y == layout2.y, "Children should have same Y");
    native_destroy_window(win);
}

/// Flex column distributes children vertically
fn spec_flex_column_vertical() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "display", "flex");
    native_set_style(parent, "flex-direction", "column");
    native_set_style(parent, "width", "100px");
    native_set_style(parent, "height", "300px");

    ≔ child1 = native_create_element(win, "div");
    native_set_style(child1, "width", "100px");
    native_set_style(child1, "height", "100px");

    ≔ child2 = native_create_element(win, "div");
    native_set_style(child2, "width", "100px");
    native_set_style(child2, "height", "100px");

    native_append_child(parent, child1);
    native_append_child(parent, child2);

    ≔ layout1 = native_get_layout(child1);
    ≔ layout2 = native_get_layout(child2);

    assert(layout1.y < layout2.y, "Child2 should be below Child1");
    assert(layout1.x == layout2.x, "Children should have same X");
    native_destroy_window(win);
}

/// justify-content: center centers children
fn spec_justify_content_center() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "display", "flex");
    native_set_style(parent, "flex-direction", "row");
    native_set_style(parent, "justify-content", "center");
    native_set_style(parent, "width", "300px");
    native_set_style(parent, "height", "100px");

    ≔ child = native_create_element(win, "div");
    native_set_style(child, "width", "100px");
    native_set_style(child, "height", "100px");

    native_append_child(parent, child);

    ≔ layout = native_get_layout(child);
    ≔ expected_x = (300 - 100) / 2;  // 100

    assert(layout.x ≈ expected_x, "Child should be centered");
    native_destroy_window(win);
}

/// align-items: center centers on cross axis
fn spec_align_items_center() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "display", "flex");
    native_set_style(parent, "flex-direction", "row");
    native_set_style(parent, "align-items", "center");
    native_set_style(parent, "width", "300px");
    native_set_style(parent, "height", "100px");

    ≔ child = native_create_element(win, "div");
    native_set_style(child, "width", "100px");
    native_set_style(child, "height", "50px");

    native_append_child(parent, child);

    ≔ layout = native_get_layout(child);
    ≔ expected_y = (100 - 50) / 2;  // 25

    assert(layout.y ≈ expected_y, "Child should be vertically centered");
    native_destroy_window(win);
}

/// justify-content: space-between distributes children
fn spec_justify_content_space_between() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "display", "flex");
    native_set_style(parent, "flex-direction", "row");
    native_set_style(parent, "justify-content", "space-between");
    native_set_style(parent, "width", "300px");
    native_set_style(parent, "height", "100px");

    ≔ child1 = native_create_element(win, "div");
    native_set_style(child1, "width", "50px");
    native_set_style(child1, "height", "50px");

    ≔ child2 = native_create_element(win, "div");
    native_set_style(child2, "width", "50px");
    native_set_style(child2, "height", "50px");

    native_append_child(parent, child1);
    native_append_child(parent, child2);
    native_set_root(win, parent);
    native_compute_layout(win);

    ≔ layout1 = native_get_layout(child1);
    ≔ layout2 = native_get_layout(child2);

    assert(layout1.x == 0, "First child at start");
    assert(layout2.x == 250, "Second child at end (300 - 50)");
    native_destroy_window(win);
}

/// gap property adds spacing between children
fn spec_gap_adds_spacing() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "display", "flex");
    native_set_style(parent, "flex-direction", "row");
    native_set_style(parent, "gap", "20px");
    native_set_style(parent, "width", "300px");

    ≔ child1 = native_create_element(win, "div");
    native_set_style(child1, "width", "50px");
    native_set_style(child1, "height", "50px");

    ≔ child2 = native_create_element(win, "div");
    native_set_style(child2, "width", "50px");
    native_set_style(child2, "height", "50px");

    native_append_child(parent, child1);
    native_append_child(parent, child2);
    native_set_root(win, parent);
    native_compute_layout(win);

    ≔ layout1 = native_get_layout(child1);
    ≔ layout2 = native_get_layout(child2);

    assert(layout2.x == 70, "Second child after gap (50 + 20)");
    native_destroy_window(win);
}

/// padding adds internal spacing
fn spec_padding_adds_internal_spacing() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "display", "flex");
    native_set_style(parent, "padding", "10px");
    native_set_style(parent, "width", "100px");
    native_set_style(parent, "height", "100px");

    ≔ child = native_create_element(win, "div");
    native_set_style(child, "width", "50px");
    native_set_style(child, "height", "50px");

    native_append_child(parent, child);
    native_set_root(win, parent);
    native_compute_layout(win);

    ≔ layout = native_get_layout(child);

    assert(layout.x == 10, "Child offset by left padding");
    assert(layout.y == 10, "Child offset by top padding");
    native_destroy_window(win);
}

/// nested flex containers layout correctly
fn spec_nested_flex_layout() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ outer = native_create_element(win, "div");
    native_set_style(outer, "display", "flex");
    native_set_style(outer, "flex-direction", "row");
    native_set_style(outer, "width", "200px");
    native_set_style(outer, "height", "100px");

    ≔ inner = native_create_element(win, "div");
    native_set_style(inner, "display", "flex");
    native_set_style(inner, "flex-direction", "column");
    native_set_style(inner, "width", "100px");

    ≔ child1 = native_create_element(win, "div");
    native_set_style(child1, "width", "50px");
    native_set_style(child1, "height", "30px");

    ≔ child2 = native_create_element(win, "div");
    native_set_style(child2, "width", "50px");
    native_set_style(child2, "height", "30px");

    native_append_child(inner, child1);
    native_append_child(inner, child2);
    native_append_child(outer, inner);
    native_set_root(win, outer);
    native_compute_layout(win);

    ≔ layout1 = native_get_layout(child1);
    ≔ layout2 = native_get_layout(child2);

    // Children should be stacked vertically within inner
    assert(layout1.y == 0, "First child at top of inner");
    assert(layout2.y == 30, "Second child below first");
    assert(layout1.x == layout2.x, "Same X position in column");
    native_destroy_window(win);
}
```

**Criteria:** All 8 tests pass.

---

## Phase 5: Rendering Basics

### Specification Tests

```sigil
/// Background color is applied (visual verification)
fn spec_background_color_renders() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "200px");
    native_set_style(elem, "height", "200px");
    native_set_style(elem, "background-color", "#ff0000");
    native_set_root(win, elem);

    // Render one frame
    native_request_animation_frame(0);
    native_poll_events();

    // Visual verification: should see red square
    // Automated: pixel sampling at center
    ≔ pixel = native_sample_pixel(win, 200, 150);
    assert(pixel.r > 200 ∧ pixel.g < 50 ∧ pixel.b < 50, "Should be red");

    native_destroy_window(win);
}

/// Text content renders (visual verification)
fn spec_text_renders() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "200px");
    native_set_style(elem, "height", "50px");
    native_set_style(elem, "color", "#000000");
    native_set_style(elem, "font-size", "16px");
    native_set_text_content(elem, "Hello");
    native_set_root(win, elem);

    native_request_animation_frame(0);
    native_poll_events();

    // Text rendering verification is complex
    // Basic check: some pixels should be dark (text)
    ≔ has_dark_pixel = native_has_pixels_matching(win, |p| p.r < 50 ∧ p.g < 50 ∧ p.b < 50);
    assert(has_dark_pixel, "Text should render some dark pixels");

    native_destroy_window(win);
}

/// Border radius creates rounded corners
fn spec_border_radius_renders() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "100px");
    native_set_style(elem, "height", "100px");
    native_set_style(elem, "background-color", "#0000ff");
    native_set_style(elem, "border-radius", "50px");  // Circle
    native_set_root(win, elem);

    native_request_animation_frame(0);
    native_poll_events();

    // Corner should be transparent (not blue)
    ≔ corner = native_sample_pixel(win, 5, 5);
    assert(corner.b < 50, "Corner should not be blue (rounded)");

    // Center should be blue
    ≔ center = native_sample_pixel(win, 50, 50);
    assert(center.b > 200, "Center should be blue");

    native_destroy_window(win);
}
```

**Criteria:** All 3 tests pass (may require visual verification helpers).

---

## Phase 6: Event Handling

### Specification Tests

```sigil
/// Click event is dispatched to target element
fn spec_click_dispatches_to_target() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "100px");
    native_set_style(elem, "height", "100px");
    native_set_root(win, elem);

    ≔ Δ click_received = False;
    ≔ callback_id = 42u64;
    native_add_event_listener(elem, EVENT_CLICK, callback_id);

    // Simulate click at center of element
    native_simulate_click(win, 50, 50);

    // Poll for event
    ≔ event = native_poll_event();
    assert(event.type == EVENT_CLICK, "Should receive click event");
    assert(event.callback_id == callback_id, "Callback ID should match");

    native_destroy_window(win);
}

/// KeyDown event includes correct key and modifiers
fn spec_keydown_has_key_and_modifiers() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ elem = native_create_element(win, "input");
    native_set_root(win, elem);
    native_focus(elem);

    ≔ callback_id = 43u64;
    native_add_event_listener(elem, EVENT_KEYDOWN, callback_id);

    // Simulate Ctrl+A
    native_simulate_key(win, KEY_A, MODIFIER_CTRL);

    ≔ event = native_poll_event();
    assert(event.type == EVENT_KEYDOWN, "Should receive keydown");
    assert(event.key == KEY_A, "Key should be A");
    assert(event.modifiers.ctrl == True, "Ctrl should be pressed");

    native_destroy_window(win);
}

/// Event listener can be removed
fn spec_remove_event_listener() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "100px");
    native_set_style(elem, "height", "100px");
    native_set_root(win, elem);

    ≔ callback_id = 44u64;
    native_add_event_listener(elem, EVENT_CLICK, callback_id);
    native_remove_event_listener(elem, EVENT_CLICK, callback_id);

    native_simulate_click(win, 50, 50);

    ≔ event = native_poll_event();
    assert(event.type == -1, "Should not receive event after removal");

    native_destroy_window(win);
}

/// Focus event is dispatched when element is focused
fn spec_focus_event_dispatched() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ input = native_create_element(win, "input");
    native_set_root(win, input);

    ≔ callback_id = 50u64;
    native_add_event_listener(input, EVENT_FOCUS, callback_id);

    native_focus(input);

    ≔ event = native_poll_event();
    assert(event.event_type == EVENT_FOCUS, "Should receive focus event");
    assert(event.callback_id == callback_id, "Callback ID should match");
    assert(native_get_focused(win) == input, "Element should be focused");

    native_destroy_window(win);
}

/// Blur event is dispatched when element loses focus
fn spec_blur_event_dispatched() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ input1 = native_create_element(win, "input");
    ≔ input2 = native_create_element(win, "input");
    ≔ container = native_create_element(win, "div");
    native_append_child(container, input1);
    native_append_child(container, input2);
    native_set_root(win, container);

    ≔ blur_callback = 51u64;
    native_add_event_listener(input1, EVENT_BLUR, blur_callback);

    native_focus(input1);
    native_poll_event();  // Consume focus event
    native_focus(input2);  // Focus second input, blurring first

    ≔ event = native_poll_event();
    assert(event.event_type == EVENT_BLUR, "Should receive blur event");
    assert(event.callback_id == blur_callback, "Blur callback should fire");

    native_destroy_window(win);
}

/// MouseMove event reports correct coordinates
fn spec_mouse_move_coordinates() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "200px");
    native_set_style(elem, "height", "200px");
    native_set_root(win, elem);

    ≔ callback_id = 52u64;
    native_add_event_listener(elem, EVENT_MOUSEMOVE, callback_id);

    // Simulate mouse move to (75, 125)
    native_simulate_mouse_move(win, 75.0, 125.0);

    ≔ event = native_poll_event();
    assert(event.event_type == EVENT_MOUSEMOVE, "Should receive mouse move");
    assert(event.x ≈ 75.0, "X coordinate should match");
    assert(event.y ≈ 125.0, "Y coordinate should match");

    native_destroy_window(win);
}

/// Scroll event includes delta values
fn spec_scroll_event_delta() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ scrollable = native_create_element(win, "div");
    native_set_style(scrollable, "width", "200px");
    native_set_style(scrollable, "height", "200px");
    native_set_style(scrollable, "overflow", "scroll");
    native_set_root(win, scrollable);

    ≔ callback_id = 53u64;
    native_add_event_listener(scrollable, EVENT_SCROLL, callback_id);

    // Simulate scroll (delta_y = -100 for scroll down)
    native_simulate_scroll(win, 0.0, -100.0);

    ≔ event = native_poll_event();
    assert(event.event_type == EVENT_SCROLL, "Should receive scroll event");
    assert(event.delta_y ≈ -100.0, "Delta Y should match");

    native_destroy_window(win);
}

/// Event bubbles from child to parent
fn spec_event_bubbling() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ parent = native_create_element(win, "div");
    native_set_style(parent, "width", "200px");
    native_set_style(parent, "height", "200px");

    ≔ child = native_create_element(win, "div");
    native_set_style(child, "width", "100px");
    native_set_style(child, "height", "100px");

    native_append_child(parent, child);
    native_set_root(win, parent);

    ≔ parent_callback = 54u64;
    ≔ child_callback = 55u64;
    native_add_event_listener(parent, EVENT_CLICK, parent_callback);
    native_add_event_listener(child, EVENT_CLICK, child_callback);

    // Click on child (within its bounds)
    native_simulate_click(win, 50, 50);

    // Should receive child event first
    ≔ event1 = native_poll_event();
    assert(event1.callback_id == child_callback, "Child callback first");

    // Then parent event (bubbling)
    ≔ event2 = native_poll_event();
    assert(event2.callback_id == parent_callback, "Parent callback second");

    native_destroy_window(win);
}
```

**Criteria:** All 9 tests pass.

---

## Phase 7: Timing

### Specification Tests

```sigil
/// Animation frame callback is invoked
fn spec_animation_frame_invoked() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ callback_id = 45u64;

    native_request_animation_frame(callback_id);

    // Wait for vsync (up to 20ms)
    ≔ event = native_poll_event_timeout(20);
    assert(event.type == EVENT_ANIMATION_FRAME, "Should receive frame");
    assert(event.callback_id == callback_id, "Callback ID should match");

    native_destroy_window(win);
}

/// Timeout fires after specified delay
fn spec_timeout_fires() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ callback_id = 46u64;
    ≔ start = native_now_ms();

    native_set_timeout(callback_id, 50);  // 50ms delay

    // Wait for timeout
    ≔ event = native_poll_event_timeout(100);
    ≔ elapsed = native_now_ms() - start;

    assert(event.type == EVENT_TIMEOUT, "Should receive timeout");
    assert(elapsed >= 50, "Should wait at least 50ms");
    assert(elapsed < 100, "Should not wait too long");

    native_destroy_window(win);
}

/// Cleared timeout does not fire
fn spec_clear_timeout_prevents_fire() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ callback_id = 47u64;

    ≔ timer_id = native_set_timeout(callback_id, 50);
    native_clear_timeout(timer_id);

    // Wait past the timeout
    ≔ event = native_poll_event_timeout(100);
    assert(event.type == -1, "Cleared timeout should not fire");

    native_destroy_window(win);
}
```

**Criteria:** All 3 tests pass.

---

## Phase 8: Integration Test

### Counter App

```sigil
/// Complete counter app works
fn integration_counter_app() {
    ≔ win = native_create_window("Counter", 400, 200);

    // Build UI
    ≔ container = native_create_element(win, "div");
    native_set_style(container, "display", "flex");
    native_set_style(container, "flex-direction", "column");
    native_set_style(container, "align-items", "center");
    native_set_style(container, "padding", "20px");

    ≔ count_text = native_create_element(win, "div");
    native_set_style(count_text, "font-size", "24px");
    native_set_text_content(count_text, "Count: 0");

    ≔ button = native_create_element(win, "button");
    native_set_style(button, "padding", "10px 20px");
    native_set_text_content(button, "Increment");

    native_append_child(container, count_text);
    native_append_child(container, button);
    native_set_root(win, container);

    // Add click listener
    ≔ Δ count = 0;
    ≔ callback_id = 100u64;
    native_add_event_listener(button, EVENT_CLICK, callback_id);

    // Render initial frame
    native_request_animation_frame(0);
    native_poll_events();

    // Simulate click
    ≔ button_layout = native_get_layout(button);
    native_simulate_click(win, button_layout.x + 20, button_layout.y + 10);

    // Process event
    ≔ event = native_poll_event();
    assert(event.type == EVENT_CLICK, "Should receive click");

    // Update state
    count = count + 1;
    native_set_text_content(count_text, "Count: " ++ count.to_string());

    // Render update
    native_request_animation_frame(0);
    native_poll_events();

    // Verify (visual or text content check)
    ≔ content = native_get_text_content(count_text);
    assert(content == "Count: 1", "Counter should increment");

    native_destroy_window(win);
}
```

**Criteria:** Counter app compiles, runs, and increments on click.

---

## Phase 9: Clipboard API

### Specification Tests

Reference: [CLIPBOARD-SPEC.md](./CLIPBOARD-SPEC.md) v0.2.0

```sigil
/// API version returns expected value
fn spec_clipboard_api_version() {
    ≔ version = native_clipboard_api_version();
    assert(version == 0x000200, "Should be v0.2.0");
}

/// Capabilities include read and write
fn spec_clipboard_capabilities() {
    ≔ caps = native_clipboard_capabilities();
    assert(caps & CLIPBOARD_CAP_READ != 0, "Should have read");
    assert(caps & CLIPBOARD_CAP_WRITE != 0, "Should have write");
}

/// Write handle is non-zero
fn spec_write_begin_returns_handle() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    assert(handle > 0, "Handle should be positive");
}

/// Adding format to valid handle succeeds
fn spec_write_add_format_valid() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    ≔ result = native_clipboard_write_add_format(handle, "text/plain", data, len);
    assert(result == 1, "Should succeed");
}

/// Adding format to invalid handle fails
fn spec_write_add_format_invalid() {
    ≔ result = native_clipboard_write_add_format(99999, "text/plain", data, len);
    assert(result == 0, "Should fail");
}

/// Commit with invalid handle fires error event
fn spec_write_commit_invalid_handle() {
    native_clipboard_write_commit(99999, callback_id);
    ≔ event = native_poll_event();
    assert(event.type == EVENT_CLIPBOARD_ERROR, "Should fire error");
    assert(event.button == CLIPBOARD_ERR_INVALID_HANDLE, "Should be invalid handle error");
}

/// Cancel removes pending handle
fn spec_write_cancel_removes_handle() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    native_clipboard_write_cancel(handle);
    ≔ result = native_clipboard_write_add_format(handle, "text/plain", data, len);
    assert(result == 0, "Cancelled handle should be invalid");
}

/// Release removes completed data
fn spec_release_removes_data() {
    // Insert test data
    ≔ size = native_clipboard_get_data_size(callback_id);
    assert(size > 0, "Data should exist");
    native_clipboard_release(callback_id);
    ≔ size_after = native_clipboard_get_data_size(callback_id);
    assert(size_after == 0, "Data should be gone");
}

/// Get data retrieves stored data
fn spec_get_data_retrieves() {
    // Insert test data via completed storage
    ≔ len = native_clipboard_get_data(callback_id, buf, max);
    assert(len == expected_len, "Length should match");
    assert(buf == expected_data, "Data should match");
}

/// Get data truncates to buffer size
fn spec_get_data_truncates() {
    ≔ len = native_clipboard_get_data(callback_id, small_buf, 10);
    assert(len == 10, "Should truncate");
}

/// Get formats data returns format list
fn spec_get_formats_data() {
    // Insert formats via completed storage
    ≔ count = native_clipboard_get_formats_data(callback_id, out, max);
    assert(count == expected_count, "Count should match");
}

/// Handle overflow protection
fn spec_write_handle_overflow() {
    // Set next_handle to 0
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    assert(handle == 0, "Should return 0 on overflow");
}

// =========================================================================
// Phase 2 Clipboard Tests: HTML and File List Support
// =========================================================================

/// Capabilities include HTML and FILES
fn spec_capabilities_includes_html_files() {
    ≔ caps = native_clipboard_capabilities();
    assert(caps & CLIPBOARD_CAP_HTML != 0, "Should have HTML capability");
    assert(caps & CLIPBOARD_CAP_FILES != 0, "Should have FILES capability");
}

/// Write HTML format stores correctly
fn spec_write_html_format() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    ≔ result = native_clipboard_write_add_format(handle, "text/html", html, len);
    assert(result == 1, "Should succeed");
}

/// Write file list format stores correctly
fn spec_write_file_list_format() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    ≔ result = native_clipboard_write_add_format(handle, "text/uri-list", uris, len);
    assert(result == 1, "Should succeed");
}

/// Read unsupported format returns error
fn spec_read_unsupported_format() {
    native_clipboard_read_format(CLIPBOARD_TARGET, "application/x-unsupported", callback_id);
    ≔ event = native_poll_event();
    assert(event.type == EVENT_CLIPBOARD_ERROR, "Should fire error");
    assert(event.button == CLIPBOARD_ERR_FORMAT_NOT_FOUND, "Should be format not found");
}

/// HTML only (no plain fallback) stores correctly
fn spec_write_html_only() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    native_clipboard_write_add_format(handle, "text/html", html, len);
    // Verify only 1 format stored
    assert(builder.formats.len() == 1, "Should have only HTML");
}

/// File list with RFC 2483 comments parses correctly
fn spec_file_list_with_comments() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    ≔ uri_list = "# Comment\nfile:///path\n";
    ≔ result = native_clipboard_write_add_format(handle, "text/uri-list", uri_list, len);
    assert(result == 1, "Should succeed with comments");
}
```

**Criteria:** All 20 clipboard tests pass (14 Phase 1 + 6 Phase 2).

### Phase 3 Clipboard Tests: Image Support

```sigil
/// Capabilities include IMAGES
fn spec_capabilities_includes_images() {
    ≔ caps = native_clipboard_capabilities();
    assert(caps & CLIPBOARD_CAP_IMAGES != 0, "Should have IMAGES capability");
}

/// Write image/png format stores correctly
fn spec_write_image_png_format() {
    ≔ handle = native_clipboard_write_begin(CLIPBOARD_TARGET);
    ≔ result = native_clipboard_write_add_format(handle, "image/png", png_data, len);
    assert(result == 1, "Should succeed");
}

/// PNG encode/decode roundtrip preserves data
fn spec_encode_decode_png_roundtrip() {
    ≔ png = encode_rgba_to_png(rgba_data, width, height);
    ≔ (decoded, w, h) = decode_png_to_rgba(png);
    assert(decoded == rgba_data, "Should roundtrip");
}

/// Invalid PNG data returns error
fn spec_decode_png_invalid_data() {
    ≔ result = decode_png_to_rgba("not a png");
    assert(result.is_err(), "Should fail on invalid PNG");
}

/// Dimension mismatch returns error
fn spec_encode_rgba_dimension_mismatch() {
    ≔ result = encode_rgba_to_png(4_bytes, 2, 2); // 1 pixel for 2x2
    assert(result.is_err(), "Should fail on mismatch");
}
```

**Criteria:** All 25 clipboard tests pass (14 Phase 1 + 6 Phase 2 + 5 Phase 3).

**Implementation Notes (2026-02-17):**
- Async event-based API per CLIPBOARD-SPEC.md v0.2.0
- arboard crate provides cross-platform clipboard access
- Phase 1: text/plain and text/plain;charset=utf-8 MIME types
- Phase 2: text/html, text/uri-list MIME types
- Phase 3: image/png MIME type with PNG encode/decode
- Per-callback CString storage prevents use-after-free
- Timeout processing: 30s data lifetime, 60s write handle timeout
- Return values: write_add_* returns 1/success, 0/failure
- Primary selection logged as warning (not yet supported)
- Callback ID collision detection with warning log
- Handle overflow protection returns 0
- HTML write uses arboard set().html() with optional plain text fallback
- File list uses arboard set().file_list() and get().file_list()
- Format detection probes clipboard for text, HTML, file list, and images
- PNG encoding via `image` crate (0.25) with minimal features
- Reads image from clipboard via arboard, encodes to PNG for MIME output
- Decodes PNG input to RGBA pixels for arboard clipboard write

---

## Test Summary

| Phase | Tests | Status |
|-------|-------|--------|
| 1. Window Management | 3 | ✅ 3/3 Passing |
| 2. Element Creation | 3 | ✅ 3/3 Passing |
| 3. Element Tree | 4 | ✅ 4/4 Passing |
| 4. Flexbox Layout | 8 | ✅ 8/8 Passing |
| 5. Rendering Basics | 4 | ✅ 4/4 Passing |
| 6. Event Handling | 6 | ✅ 6/6 Passing |
| 7. Timing | 5 | ✅ 5/5 Passing |
| 8. Integration | 1 | ✅ 1/1 Passing |
| 9. Clipboard API (Phase 1+2+3) | 25 | ✅ 25/25 Passing |
| **Total** | **59** | **78/78 passing** |

*Note: Total includes additional edge case tests beyond spec requirements.*

### Implementation Notes (2026-02-17)

**Rust Unit Tests:** 78 tests in `lib.rs` cover complete FFI functionality:

**Phase 1-3: Core Infrastructure**
- Window creation/destruction with proper handle management
- Element creation, destruction, text content storage
- Parent-child relationships (append, remove, insert_before)

**Phase 4: Flexbox Layout (Full)**
- Row/column layouts
- justify-content: center, space-between
- align-items: center
- gap spacing
- padding offsets
- Nested flex containers

**Phase 5: Software Rendering**
- Background color rendering to framebuffer
- Pixel sampling at coordinates
- Pixel range matching (has_pixels_matching)
- Nested element z-ordering

**Phase 6: Event System**
- Click, focus, blur events
- Event bubbling (child → parent)
- Listener registration/removal
- Hit testing for click targets

**Phase 7: Timing**
- `native_now_ms()` monotonic time
- `native_set_timeout()` / `native_clear_timeout()`
- `native_request_animation_frame()` / `native_cancel_animation_frame()`

**Phase 8: Integration**
- Counter app end-to-end test demonstrating full workflow

**Technical Details:**
- `native_poll_event()` uses FIFO ordering
- Tests use `#[serial]` attribute to prevent global state races
- `reset_state()` properly resets taffy layout tree and timers
- Software renderer writes to Vec<Pixel> framebuffer

---

## Next Steps

1. ~~**Implement Phase 1** (Window Management) in wgpu backend~~ ✅
2. ~~Run tests, fix until passing~~ ✅
3. ~~**Implement Phases 2-4, 6-7 core**~~ ✅
4. ~~**Implement remaining Flexbox tests** (justify-content, align-items variants)~~ ✅
5. ~~**Implement software rendering pipeline** for Phase 5~~ ✅
6. ~~**Implement timer callbacks** (setTimeout, requestAnimationFrame)~~ ✅
7. ~~**Integration test** with counter app~~ ✅

---

## Phase 10: Monospace Font Support

**Spec ref:** NATIVE-RENDERING-SPEC.md §3.10
**Status:** 🔴 RED — implementation not yet present

### Key property under test

Two equal-length strings of narrow vs. wide ASCII characters must produce elements of
equal computed width when `font-family: monospace` and `width: auto` are set. This is
the behavioral compliance test for the monospace advance-width invariant.

### Specification Tests

```sigil
/// font-family: monospace is accepted without crash
fn spec_font_family_monospace_accepted() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ elem = native_create_element(win, "div");

    // Should store the property without error
    native_set_style(elem, "font-family", "monospace");

    native_destroy_window(win);
}

/// Monospace font produces equal advance widths (compliance test)
///
/// Verified via layout: "iiiiiiii" and "WWWWWWWW" with font-family:monospace
/// and width:auto must produce elements of equal computed width.
fn spec_monospace_equal_advance_widths() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ container = native_create_element(win, "div");
    native_set_style(container, "display", "flex");
    native_set_style(container, "flex-direction", "column");

    ≔ narrow = native_create_element(win, "div");  // all 'i'
    native_set_style(narrow, "font-family", "monospace");
    native_set_style(narrow, "font-size", "16px");
    native_set_text_content(narrow, "iiiiiiii");

    ≔ wide = native_create_element(win, "div");    // all 'W'
    native_set_style(wide, "font-family", "monospace");
    native_set_style(wide, "font-size", "16px");
    native_set_text_content(wide, "WWWWWWWW");

    native_append_child(container, narrow);
    native_append_child(container, wide);
    native_set_root(win, container);
    native_compute_layout(win);

    ≔ layout_narrow = native_get_layout(narrow);
    ≔ layout_wide   = native_get_layout(wide);

    assert((layout_narrow.width - layout_wide.width).abs() < 1.0,
        "Monospace: equal char count must produce equal element width");

    native_destroy_window(win);
}

/// Sans-serif produces unequal advance widths (inverse test)
///
/// Proves spec_monospace_equal_advance_widths would fail without the font:
/// in proportional fonts, 'i' is narrower than 'W'.
fn spec_sans_serif_unequal_advance_widths() {
    ≔ win = native_create_window("Test", 800, 600);
    ≔ container = native_create_element(win, "div");
    native_set_style(container, "display", "flex");
    native_set_style(container, "flex-direction", "column");

    ≔ narrow = native_create_element(win, "div");
    native_set_style(narrow, "font-family", "sans-serif");
    native_set_style(narrow, "font-size", "16px");
    native_set_text_content(narrow, "iiiiiiii");

    ≔ wide = native_create_element(win, "div");
    native_set_style(wide, "font-family", "sans-serif");
    native_set_style(wide, "font-size", "16px");
    native_set_text_content(wide, "WWWWWWWW");

    native_append_child(container, narrow);
    native_append_child(container, wide);
    native_set_root(win, container);
    native_compute_layout(win);

    ≔ layout_narrow = native_get_layout(narrow);
    ≔ layout_wide   = native_get_layout(wide);

    assert(layout_narrow.width < layout_wide.width,
        "Sans-serif: 'iiiiiiii' must be narrower than 'WWWWWWWW'");

    native_destroy_window(win);
}

/// Monospace text renders glyphs to framebuffer
fn spec_monospace_text_renders() {
    ≔ win = native_create_window("Test", 400, 100);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "400px");
    native_set_style(elem, "height", "100px");
    native_set_style(elem, "font-family", "monospace");
    native_set_style(elem, "font-size", "16px");
    native_set_style(elem, "color", "#000000");
    native_set_text_content(elem, "fn main() {}");
    native_set_root(win, elem);

    native_request_animation_frame(0);
    native_poll_events();

    ≔ has_dark = native_has_pixels_matching(win, |p| p.r < 50 ∧ p.g < 50 ∧ p.b < 50);
    assert(has_dark, "Monospace text must render dark pixels");

    native_destroy_window(win);
}
```

**Criteria:** All 4 tests pass.

---

## Phase 11: Text Span API

**Spec ref:** NATIVE-RENDERING-SPEC.md §3.11
**Status:** 🔴 RED — `native_set_text_spans` does not exist yet

`TextSpan` is a C-compatible struct `{ start: u32, end: u32, r: u8, g: u8, b: u8, a: u8 }`.
`native_set_text_spans(elem, spans, count)` sets per-run color overrides on an element.

### Specification Tests

```sigil
/// Span covering full text range colors text in span color
///
/// Element color is black; span [0, 5) is red.
/// Rendered text must have red pixels.
fn spec_text_span_full_range_colors_text() {
    ≔ win = native_create_window("Test", 400, 100);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "400px");
    native_set_style(elem, "height", "100px");
    native_set_style(elem, "font-size", "24px");
    native_set_style(elem, "color", "#000000");  // element default: black
    native_set_text_content(elem, "Hello");       // 5 bytes
    native_set_root(win, elem);

    // Span: bytes [0, 5) → red (covers all of "Hello")
    native_set_text_spans(elem, [TextSpan { start: 0, end: 5, r: 255, g: 0, b: 0, a: 255 }], 1);

    native_request_animation_frame(0);
    native_poll_events();

    ≔ has_red = native_has_pixels_matching(win, |p| p.r > 200 ∧ p.g < 50 ∧ p.b < 50);
    assert(has_red, "Span must render text in red");

    native_destroy_window(win);
}

/// Calling set_text_spans with count=0 clears spans; element color is restored
fn spec_text_span_cleared_by_zero_count() {
    ≔ win = native_create_window("Test", 400, 100);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "400px");
    native_set_style(elem, "height", "100px");
    native_set_style(elem, "font-size", "24px");
    native_set_style(elem, "color", "#000000");
    native_set_text_content(elem, "Hi");
    native_set_root(win, elem);

    // Set red span, then immediately clear it
    native_set_text_spans(elem, [TextSpan { start: 0, end: 2, r: 255, g: 0, b: 0, a: 255 }], 1);
    native_set_text_spans(elem, [], 0);  // clear

    native_request_animation_frame(0);
    native_poll_events();

    // No red pixels: cleared spans must not affect rendering
    ≔ has_red = native_has_pixels_matching(win, |p| p.r > 200 ∧ p.g < 50 ∧ p.b < 50);
    assert(¬has_red, "Cleared spans must not produce red pixels");

    native_destroy_window(win);
}

/// Later span in array wins when two spans cover the same byte range
fn spec_text_span_later_wins_on_overlap() {
    ≔ win = native_create_window("Test", 400, 100);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "400px");
    native_set_style(elem, "height", "100px");
    native_set_style(elem, "font-size", "24px");
    native_set_text_content(elem, "X");
    native_set_root(win, elem);

    // Span 0: blue. Span 1: red. Same range [0,1). Red must win.
    native_set_text_spans(elem, [
        TextSpan { start: 0, end: 1, r: 0,   g: 0, b: 255, a: 255 },  // blue
        TextSpan { start: 0, end: 1, r: 255, g: 0, b: 0,   a: 255 },  // red — wins
    ], 2);

    native_request_animation_frame(0);
    native_poll_events();

    ≔ has_red  = native_has_pixels_matching(win, |p| p.r > 200 ∧ p.g < 50 ∧ p.b < 50);
    ≔ has_blue = native_has_pixels_matching(win, |p| p.r < 50 ∧ p.g < 50 ∧ p.b > 200);
    assert(has_red,   "Later span (red) must win");
    assert(¬has_blue, "Earlier span (blue) must be overridden");

    native_destroy_window(win);
}

/// Stale spans (end > new text length) do not cause a crash
///
/// Callers may set spans then replace text with shorter content.
/// Spans are clamped to new text length at render time; no panic.
fn spec_text_span_stale_range_clamped() {
    ≔ win = native_create_window("Test", 400, 100);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "400px");
    native_set_style(elem, "height", "100px");
    native_set_text_content(elem, "LongText");  // 8 bytes

    native_set_text_spans(elem, [TextSpan { start: 0, end: 8, r: 255, g: 0, b: 0, a: 255 }], 1);

    // Replace with shorter content — span.end (8) > new len (2)
    native_set_text_content(elem, "Hi");
    native_set_root(win, elem);

    // Must render without crash; spans clamped to [0, 2)
    native_request_animation_frame(0);
    native_poll_events();

    native_destroy_window(win);
}

/// A zero-length span [n, n) has no visual effect
fn spec_text_span_zero_length_has_no_effect() {
    ≔ win = native_create_window("Test", 400, 100);
    ≔ elem = native_create_element(win, "div");
    native_set_style(elem, "width", "400px");
    native_set_style(elem, "height", "100px");
    native_set_style(elem, "font-size", "24px");
    native_set_style(elem, "color", "#000000");
    native_set_text_content(elem, "A");
    native_set_root(win, elem);

    // Zero-length red span at byte 0 — must not color anything
    native_set_text_spans(elem, [TextSpan { start: 0, end: 0, r: 255, g: 0, b: 0, a: 255 }], 1);

    native_request_animation_frame(0);
    native_poll_events();

    ≔ has_red = native_has_pixels_matching(win, |p| p.r > 200 ∧ p.g < 50 ∧ p.b < 50);
    assert(¬has_red, "Zero-length span must have no visual effect");

    native_destroy_window(win);
}
```

**Criteria:** All 5 tests pass.

---

## Phase 12: GPU Rendering Activation

**Spec ref:** NATIVE-RENDERING-SPEC.md §3.12
**Status:** 🔴 RED — `native_set_render_mode` / `native_get_render_mode` do not exist yet

`native_set_render_mode(window, mode)` returns 0 on success, -1 if GPU unavailable.
`native_get_render_mode(window)` returns `RENDER_MODE_SOFTWARE` (0) or `RENDER_MODE_GPU` (1).

The GPU equivalence test requires physical GPU or a software Vulkan adapter and should be
skipped in environments without GPU capability.

### Specification Tests

```sigil
/// Default render mode is Software
fn spec_default_render_mode_is_software() {
    ≔ win = native_create_window("Test", 400, 300);
    assert(native_get_render_mode(win) == RENDER_MODE_SOFTWARE,
        "Default must be Software");
    native_destroy_window(win);
}

/// Setting Software mode always succeeds
fn spec_set_render_mode_software_always_succeeds() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ result = native_set_render_mode(win, RENDER_MODE_SOFTWARE);
    assert(result == 0, "Software mode must always succeed");
    assert(native_get_render_mode(win) == RENDER_MODE_SOFTWARE);
    native_destroy_window(win);
}

/// Setting GPU mode: on success mode is GPU; on failure mode stays Software
///
/// In CI without GPU, returns -1 and mode remains Software.
/// With GPU present, returns 0 and mode is GPU.
fn spec_set_render_mode_gpu_correct_on_success_or_failure() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ result = native_set_render_mode(win, RENDER_MODE_GPU);
    IF result == 0:
        assert(native_get_render_mode(win) == RENDER_MODE_GPU,
            "On success: mode must be GPU");
    ELSE:
        assert(result == -1, "Failure result must be -1");
        assert(native_get_render_mode(win) == RENDER_MODE_SOFTWARE,
            "On failure: mode must remain Software");
    native_destroy_window(win);
}

/// GPU render produces pixel-equivalent output to software render (GPU required)
///
/// Visual equivalence invariant: |software_pixel - gpu_pixel| ≤ 1 per channel.
/// Skipped in environments without GPU capability.
fn spec_gpu_render_pixel_equivalent_to_software() {
    // Render scene with software mode
    ≔ win_sw = native_create_window("SWTest", 200, 200);
    ≔ elem_sw = native_create_element(win_sw, "div");
    native_set_style(elem_sw, "width", "200px");
    native_set_style(elem_sw, "height", "200px");
    native_set_style(elem_sw, "background-color", "#3355aa");
    native_set_root(win_sw, elem_sw);
    native_request_animation_frame(0);
    native_poll_events();
    ≔ sw_pixel = native_sample_pixel(win_sw, 100, 100);

    // Render same scene with GPU mode
    ≔ win_gpu = native_create_window("GPUTest", 200, 200);
    ≔ result = native_set_render_mode(win_gpu, RENDER_MODE_GPU);
    assert(result == 0, "GPU must be available for this test");
    ≔ elem_gpu = native_create_element(win_gpu, "div");
    native_set_style(elem_gpu, "width", "200px");
    native_set_style(elem_gpu, "height", "200px");
    native_set_style(elem_gpu, "background-color", "#3355aa");
    native_set_root(win_gpu, elem_gpu);
    native_request_animation_frame(0);
    native_poll_events();
    ≔ gpu_pixel = native_sample_pixel(win_gpu, 100, 100);

    // Equivalence within ε=1 per channel
    assert((sw_pixel.r - gpu_pixel.r).abs() ≤ 1, "R channel within ε");
    assert((sw_pixel.g - gpu_pixel.g).abs() ≤ 1, "G channel within ε");
    assert((sw_pixel.b - gpu_pixel.b).abs() ≤ 1, "B channel within ε");

    native_destroy_window(win_sw);
    native_destroy_window(win_gpu);
}

/// Can return from GPU mode to Software mode
fn spec_render_mode_gpu_to_software_round_trip() {
    ≔ win = native_create_window("Test", 400, 300);
    ≔ _ = native_set_render_mode(win, RENDER_MODE_GPU);  // may fail in CI
    ≔ result = native_set_render_mode(win, RENDER_MODE_SOFTWARE);
    assert(result == 0, "Return to Software must always succeed");
    assert(native_get_render_mode(win) == RENDER_MODE_SOFTWARE);
    native_destroy_window(win);
}
```

**Criteria:** All 5 tests pass (GPU equivalence test may be skipped without GPU hardware).

---

## Test Summary

| Phase | Tests | Status |
|-------|-------|--------|
| 1. Window Management | 3 | ✅ Passing |
| 2. Element Creation | 3 | ✅ Passing |
| 3. Element Tree | 4 | ✅ Passing |
| 4. Flexbox Layout | 8 | ✅ Passing |
| 5. Rendering Basics | 4 | ✅ Passing |
| 6. Event Handling | 9 | ✅ Passing |
| 7. Timing | 3 | ✅ Passing |
| 8. Integration | 1 | ✅ Passing |
| 9. Clipboard API | 25 | ✅ Passing |
| **10. Monospace Font** | **4** | **🔴 RED** |
| **11. Text Span API** | **5** | **🔴 RED** |
| **12. GPU Rendering** | **5** | **🔴 RED** |
| **Total spec tests** | **74** | **59 passing / 14 RED** |

*Note: Rust implementation in `lib.rs` has 128 passing tests (more granular than spec tests above).*

---

## Build Phase Notes

Sigil pseudocode above is translated to Rust and added to the `#[cfg(test)] mod tests` block
in `runtime/native/wgpu/src/lib.rs`. Rust implementations must:

- Use the `cstr()` helper for string arguments: `cstr("value").as_ptr()`
- Use output pointers for `native_get_layout` and `native_sample_pixel`
- Call `reset_state()` at the start of each test
- Use `#[serial]` attribute on every test
- Gate GPU hardware tests with `#[cfg_attr(not(feature = "gpu_tests"), ignore)]`

### Step 1: Monospace Font

1. Add `NotoSansMono-Regular.ttf` to `runtime/native/wgpu/assets/fonts/`
2. `include_bytes!` it alongside Noto Sans Regular/Bold
3. Load it in `TextSystem::new()`
4. Add `font_family: Family` field to `StyleProperties` (default `Family::SansSerif`)
5. Add `"font-family"` case to CSS parser → `"monospace"` maps to `Family::Monospace`
6. Pass `element.styles.font_family` into `Attrs::new().family(...)` in `render_text()`
7. Confirm Phase 10 tests GREEN

### Step 2: Text Span API

1. Define `#[repr(C)] pub struct TextSpan { start: u32, end: u32, r: u8, g: u8, b: u8, a: u8 }`
2. Add `text_spans: Vec<TextSpan>` to `Element`
3. Implement `native_set_text_spans(elem, spans, count)`: clone spans into element; count=0 clears
4. Extend `render_text()` to accept `&[TextSpan]`; use cosmic-text `set_rich_text()` with per-run `Attrs::new().color(span_color)` split at span boundaries; clamp spans at text length
5. Thread spans through `TextRenderCommand` → `render_text_with_spans()`
6. Confirm Phase 11 tests GREEN

### Step 3: GPU Activation

1. Implement `native_set_render_mode(window, mode) -> i32` and `native_get_render_mode(window) -> i32`
2. `set_render_mode(GPU)`: call `initialize_gpu()` if needed; return 0/-1
3. Branch `native_render()` on `render_mode`: Software → existing path; GPU → `collect_gpu_instances()` + upload + submit
4. In GPU mode, `native_sample_pixel()` must do a synchronous GPU→CPU readback (wgpu buffer copy + map) before reading pixels
5. Confirm Phase 12 tests GREEN (non-GPU tests first; GPU equivalence with feature flag)

---

## Baseline

```bash
cd /home/lilith/development/projects/qliphoth/runtime/native/wgpu
cargo test -- --test-threads=1
# Expected: 128 passed, 0 failed
```

When tests reveal spec gaps, **STOP and update NATIVE-RENDERING-SPEC.md**.

**Spec ref:** NATIVE-RENDERING-SPEC.md §3.10
**Status:** 🔴 RED — implementation not yet present

### Key property under test

Two equal-length strings composed entirely of narrow characters ('i') or wide characters
('W') must produce elements of equal computed width when `font-family: monospace` and
`width: auto` are set. This is the compliance test for the monospace invariant.

### Specification Tests

```rust
/// font-family: monospace property is stored and applied
#[test]
#[serial]
fn spec_font_family_monospace_stored() {
    // Arrange
    let win = native_create_window(c"Test".as_ptr(), 800, 600);
    let elem = native_create_element(win, c"div".as_ptr());

    // Act
    native_set_style(elem, c"font-family".as_ptr(), c"monospace".as_ptr());

    // Assert: element accepts the property without crash
    // (Internal storage verified via rendering behavior in subsequent tests)
    native_destroy_window(win);
}

/// Monospace font produces equal advance widths (compliance test)
///
/// In monospace: advance_width('i') == advance_width('W')
/// Verified: render "iiiiiiii" and "WWWWWWWW" with width:auto,
/// computed layout widths must be equal.
#[test]
#[serial]
fn spec_monospace_equal_advance_widths() {
    let win = native_create_window(c"Test".as_ptr(), 800, 600);
    let container = native_create_element(win, c"div".as_ptr());
    native_set_style(container, c"display".as_ptr(), c"flex".as_ptr());
    native_set_style(container, c"flex-direction".as_ptr(), c"column".as_ptr());

    // "iiiiiiii" — narrow characters
    let narrow = native_create_element(win, c"div".as_ptr());
    native_set_style(narrow, c"font-family".as_ptr(), c"monospace".as_ptr());
    native_set_style(narrow, c"font-size".as_ptr(), c"16px".as_ptr());
    native_set_text_content(narrow, c"iiiiiiii".as_ptr());

    // "WWWWWWWW" — wide characters (same count)
    let wide = native_create_element(win, c"div".as_ptr());
    native_set_style(wide, c"font-family".as_ptr(), c"monospace".as_ptr());
    native_set_style(wide, c"font-size".as_ptr(), c"16px".as_ptr());
    native_set_text_content(wide, c"WWWWWWWW".as_ptr());

    native_append_child(container, narrow);
    native_append_child(container, wide);
    native_set_root(win, container);
    native_compute_layout(win);

    let layout_narrow = native_get_layout(narrow);
    let layout_wide = native_get_layout(wide);

    // In monospace: both must have the same width
    let diff = (layout_narrow.width - layout_wide.width).abs();
    assert!(diff < 1.0, "Monospace widths must be equal: narrow={} wide={}", layout_narrow.width, layout_wide.width);

    native_destroy_window(win);
}

/// Sans-serif produces different advance widths (inverse test)
///
/// Proves that the monospace compliance test is meaningful:
/// with sans-serif, 'i' is narrower than 'W'.
#[test]
#[serial]
fn spec_sans_serif_unequal_advance_widths() {
    let win = native_create_window(c"Test".as_ptr(), 800, 600);
    let container = native_create_element(win, c"div".as_ptr());
    native_set_style(container, c"display".as_ptr(), c"flex".as_ptr());
    native_set_style(container, c"flex-direction".as_ptr(), c"column".as_ptr());

    let narrow = native_create_element(win, c"div".as_ptr());
    native_set_style(narrow, c"font-family".as_ptr(), c"sans-serif".as_ptr());
    native_set_style(narrow, c"font-size".as_ptr(), c"16px".as_ptr());
    native_set_text_content(narrow, c"iiiiiiii".as_ptr());

    let wide = native_create_element(win, c"div".as_ptr());
    native_set_style(wide, c"font-family".as_ptr(), c"sans-serif".as_ptr());
    native_set_style(wide, c"font-size".as_ptr(), c"16px".as_ptr());
    native_set_text_content(wide, c"WWWWWWWW".as_ptr());

    native_append_child(container, narrow);
    native_append_child(container, wide);
    native_set_root(win, container);
    native_compute_layout(win);

    let layout_narrow = native_get_layout(narrow);
    let layout_wide = native_get_layout(wide);

    // In proportional: 'i' string is narrower than 'W' string
    assert!(layout_narrow.width < layout_wide.width,
        "Sans-serif 'iiiiiiii' must be narrower than 'WWWWWWWW'");

    native_destroy_window(win);
}

/// Monospace text renders to framebuffer (pixel presence check)
#[test]
#[serial]
fn spec_monospace_text_renders() {
    let win = native_create_window(c"Test".as_ptr(), 400, 100);
    let elem = native_create_element(win, c"div".as_ptr());
    native_set_style(elem, c"width".as_ptr(), c"400px".as_ptr());
    native_set_style(elem, c"height".as_ptr(), c"100px".as_ptr());
    native_set_style(elem, c"font-family".as_ptr(), c"monospace".as_ptr());
    native_set_style(elem, c"font-size".as_ptr(), c"16px".as_ptr());
    native_set_style(elem, c"color".as_ptr(), c"#000000".as_ptr());
    native_set_text_content(elem, c"fn main() {}".as_ptr());
    native_set_root(win, elem);

    native_request_animation_frame(0);
    native_poll_events();

    let has_dark = native_has_pixels_matching(win, 0, 50, 0, 50, 0, 50);
    assert!(has_dark != 0, "Monospace text must render dark pixels");

    native_destroy_window(win);
}
```

**Criteria:** All 4 tests pass.

---

## Phase 11: Text Span API

**Spec ref:** NATIVE-RENDERING-SPEC.md §3.11
**Status:** 🔴 RED — `native_set_text_spans` FFI does not exist yet

### Specification Tests

```rust
/// Span with full text range applies span color
///
/// Set "Hello" with a red span covering all 5 bytes.
/// Red pixels must appear where text renders.
#[test]
#[serial]
fn spec_text_span_full_range_colors_text() {
    let win = native_create_window(c"Test".as_ptr(), 400, 100);
    let elem = native_create_element(win, c"div".as_ptr());
    native_set_style(elem, c"width".as_ptr(), c"400px".as_ptr());
    native_set_style(elem, c"height".as_ptr(), c"100px".as_ptr());
    native_set_style(elem, c"font-size".as_ptr(), c"24px".as_ptr());
    // Element-level color is black
    native_set_style(elem, c"color".as_ptr(), c"#000000".as_ptr());
    native_set_text_content(elem, c"Hello".as_ptr());
    native_set_root(win, elem);

    // Span: bytes [0, 5) → red
    let spans = [TextSpan { start: 0, end: 5, r: 255, g: 0, b: 0, a: 255 }];
    native_set_text_spans(elem, spans.as_ptr(), 1);

    native_request_animation_frame(0);
    native_poll_events();

    // Must have red pixels (text rendered in red)
    let has_red = native_has_pixels_matching(win, 200, 255, 0, 50, 0, 50);
    assert!(has_red != 0, "Text span must produce red pixels");

    // Must NOT have black text pixels (span overrides element color)
    let has_black = native_has_pixels_matching(win, 0, 30, 0, 30, 0, 30);
    assert!(has_black == 0, "No black text when full span is red");

    native_destroy_window(win);
}

/// Zero spans restores element-level color
#[test]
#[serial]
fn spec_text_span_cleared_by_empty() {
    let win = native_create_window(c"Test".as_ptr(), 400, 100);
    let elem = native_create_element(win, c"div".as_ptr());
    native_set_style(elem, c"width".as_ptr(), c"400px".as_ptr());
    native_set_style(elem, c"height".as_ptr(), c"100px".as_ptr());
    native_set_style(elem, c"font-size".as_ptr(), c"24px".as_ptr());
    native_set_style(elem, c"color".as_ptr(), c"#000000".as_ptr());
    native_set_text_content(elem, c"Hi".as_ptr());
    native_set_root(win, elem);

    // Set red span
    let spans = [TextSpan { start: 0, end: 2, r: 255, g: 0, b: 0, a: 255 }];
    native_set_text_spans(elem, spans.as_ptr(), 1);

    // Clear spans by passing count = 0
    native_set_text_spans(elem, std::ptr::null(), 0);

    native_request_animation_frame(0);
    native_poll_events();

    // Text should render in default black (no red)
    let has_red = native_has_pixels_matching(win, 200, 255, 0, 50, 0, 50);
    assert!(has_red == 0, "Cleared spans must not render red");

    native_destroy_window(win);
}

/// Later span in array wins on overlap
#[test]
#[serial]
fn spec_text_span_later_span_wins_overlap() {
    // Two spans covering the same byte range: first blue, second red.
    // Result must be red (later wins).
    let win = native_create_window(c"Test".as_ptr(), 400, 100);
    let elem = native_create_element(win, c"div".as_ptr());
    native_set_style(elem, c"width".as_ptr(), c"400px".as_ptr());
    native_set_style(elem, c"height".as_ptr(), c"100px".as_ptr());
    native_set_style(elem, c"font-size".as_ptr(), c"24px".as_ptr());
    native_set_text_content(elem, c"X".as_ptr());
    native_set_root(win, elem);

    let spans = [
        TextSpan { start: 0, end: 1, r: 0, g: 0, b: 255, a: 255 },  // blue
        TextSpan { start: 0, end: 1, r: 255, g: 0, b: 0, a: 255 },   // red — wins
    ];
    native_set_text_spans(elem, spans.as_ptr(), 2);

    native_request_animation_frame(0);
    native_poll_events();

    let has_red = native_has_pixels_matching(win, 200, 255, 0, 50, 0, 50);
    assert!(has_red != 0, "Later span (red) must win over earlier span (blue)");

    let has_blue = native_has_pixels_matching(win, 0, 50, 0, 50, 200, 255);
    assert!(has_blue == 0, "Earlier span (blue) must be overridden");

    native_destroy_window(win);
}

/// set_text_content does not crash when spans are set with incompatible ranges
///
/// Spans with end > len(new_text) are clamped at render time; no panic.
#[test]
#[serial]
fn spec_text_span_stale_range_no_crash() {
    let win = native_create_window(c"Test".as_ptr(), 400, 100);
    let elem = native_create_element(win, c"div".as_ptr());
    native_set_style(elem, c"width".as_ptr(), c"400px".as_ptr());
    native_set_style(elem, c"height".as_ptr(), c"100px".as_ptr());
    native_set_text_content(elem, c"LongText".as_ptr());  // 8 bytes

    let spans = [TextSpan { start: 0, end: 8, r: 255, g: 0, b: 0, a: 255 }];
    native_set_text_spans(elem, spans.as_ptr(), 1);

    // Replace with shorter text — span end (8) > new len (2)
    native_set_text_content(elem, c"Hi".as_ptr());
    native_set_root(win, elem);

    // Must not panic / crash during render
    native_request_animation_frame(0);
    native_poll_events();

    native_destroy_window(win);
}

/// Zero-length span has no visual effect
#[test]
#[serial]
fn spec_text_span_zero_length_no_effect() {
    let win = native_create_window(c"Test".as_ptr(), 400, 100);
    let elem = native_create_element(win, c"div".as_ptr());
    native_set_style(elem, c"width".as_ptr(), c"400px".as_ptr());
    native_set_style(elem, c"height".as_ptr(), c"100px".as_ptr());
    native_set_style(elem, c"font-size".as_ptr(), c"24px".as_ptr());
    native_set_style(elem, c"color".as_ptr(), c"#000000".as_ptr());
    native_set_text_content(elem, c"A".as_ptr());
    native_set_root(win, elem);

    // Zero-length span at byte 0: no effect
    let spans = [TextSpan { start: 0, end: 0, r: 255, g: 0, b: 0, a: 255 }];
    native_set_text_spans(elem, spans.as_ptr(), 1);

    native_request_animation_frame(0);
    native_poll_events();

    // Text must render in black (element color), not red (zero-length span)
    let has_red = native_has_pixels_matching(win, 200, 255, 0, 50, 0, 50);
    assert!(has_red == 0, "Zero-length span must have no visual effect");

    native_destroy_window(win);
}
```

**Criteria:** All 5 tests pass.

---

## Phase 12: GPU Rendering Activation

**Spec ref:** NATIVE-RENDERING-SPEC.md §3.12
**Status:** 🔴 RED — `native_set_render_mode` / `native_get_render_mode` FFI do not exist yet

**Test feature gate:** `#[cfg_attr(not(feature = "gpu_tests"), ignore)]` on tests requiring
actual GPU submission. Mode-setting and fallback tests run in all environments.

### Specification Tests

```rust
/// Default render mode is Software
#[test]
#[serial]
fn spec_default_render_mode_is_software() {
    let win = native_create_window(c"Test".as_ptr(), 400, 300);
    let mode = native_get_render_mode(win);
    assert_eq!(mode, RENDER_MODE_SOFTWARE, "Default must be Software");
    native_destroy_window(win);
}

/// set_render_mode Software always succeeds
#[test]
#[serial]
fn spec_set_render_mode_software_succeeds() {
    let win = native_create_window(c"Test".as_ptr(), 400, 300);
    let result = native_set_render_mode(win, RENDER_MODE_SOFTWARE);
    assert_eq!(result, 0, "Software mode must always succeed");
    assert_eq!(native_get_render_mode(win), RENDER_MODE_SOFTWARE);
    native_destroy_window(win);
}

/// set_render_mode GPU → get_render_mode reports GPU
///
/// NOTE: May return -1 (GPU unavailable) in CI without GPU.
/// On return 0: mode must be GPU. On return -1: mode must remain Software.
#[test]
#[serial]
fn spec_set_render_mode_gpu_updates_mode() {
    let win = native_create_window(c"Test".as_ptr(), 400, 300);
    let result = native_set_render_mode(win, RENDER_MODE_GPU);
    if result == 0 {
        assert_eq!(native_get_render_mode(win), RENDER_MODE_GPU,
            "On success: mode must be GPU");
    } else {
        assert_eq!(result, -1, "Failure must return -1");
        assert_eq!(native_get_render_mode(win), RENDER_MODE_SOFTWARE,
            "On failure: mode must remain Software");
    }
    native_destroy_window(win);
}

/// GPU mode render produces same background color as software mode
///
/// Visual equivalence invariant: |pixel_gpu - pixel_software| ≤ 1 per channel
/// Requires GPU hardware or software Vulkan adapter.
#[test]
#[serial]
#[cfg_attr(not(feature = "gpu_tests"), ignore)]
fn spec_gpu_render_equivalent_to_software() {
    // Render with software mode
    let win_sw = native_create_window(c"SwTest".as_ptr(), 200, 200);
    let elem_sw = native_create_element(win_sw, c"div".as_ptr());
    native_set_style(elem_sw, c"width".as_ptr(), c"200px".as_ptr());
    native_set_style(elem_sw, c"height".as_ptr(), c"200px".as_ptr());
    native_set_style(elem_sw, c"background-color".as_ptr(), c"#3355aa".as_ptr());
    native_set_root(win_sw, elem_sw);
    native_request_animation_frame(0);
    native_poll_events();
    let sw_pixel = native_sample_pixel(win_sw, 100, 100);

    // Render same scene with GPU mode
    let win_gpu = native_create_window(c"GpuTest".as_ptr(), 200, 200);
    let result = native_set_render_mode(win_gpu, RENDER_MODE_GPU);
    assert_eq!(result, 0, "GPU mode must be available for this test");
    let elem_gpu = native_create_element(win_gpu, c"div".as_ptr());
    native_set_style(elem_gpu, c"width".as_ptr(), c"200px".as_ptr());
    native_set_style(elem_gpu, c"height".as_ptr(), c"200px".as_ptr());
    native_set_style(elem_gpu, c"background-color".as_ptr(), c"#3355aa".as_ptr());
    native_set_root(win_gpu, elem_gpu);
    native_request_animation_frame(0);
    native_poll_events();
    let gpu_pixel = native_sample_pixel(win_gpu, 100, 100);

    // Visual equivalence: within ε=1 per channel
    assert!((sw_pixel.r as i32 - gpu_pixel.r as i32).abs() <= 1, "R channel mismatch");
    assert!((sw_pixel.g as i32 - gpu_pixel.g as i32).abs() <= 1, "G channel mismatch");
    assert!((sw_pixel.b as i32 - gpu_pixel.b as i32).abs() <= 1, "B channel mismatch");

    native_destroy_window(win_sw);
    native_destroy_window(win_gpu);
}

/// Switching back from GPU to Software mode works
#[test]
#[serial]
fn spec_render_mode_round_trip() {
    let win = native_create_window(c"Test".as_ptr(), 400, 300);

    // Start Software → try GPU → back to Software
    assert_eq!(native_get_render_mode(win), RENDER_MODE_SOFTWARE);
    let _ = native_set_render_mode(win, RENDER_MODE_GPU);
    let result = native_set_render_mode(win, RENDER_MODE_SOFTWARE);
    assert_eq!(result, 0, "Return to Software must always succeed");
    assert_eq!(native_get_render_mode(win), RENDER_MODE_SOFTWARE);

    native_destroy_window(win);
}
```

**Criteria:** All 5 tests pass (GPU equivalence test may be skipped in CI without `gpu_tests` feature).

---

## Test Summary (Updated)

| Phase | Tests | Status |
|-------|-------|--------|
| 1. Window Management | 3 | ✅ 3/3 Passing |
| 2. Element Creation | 3 | ✅ 3/3 Passing |
| 3. Element Tree | 4 | ✅ 4/4 Passing |
| 4. Flexbox Layout | 8 | ✅ 8/8 Passing |
| 5. Rendering Basics | 4 | ✅ 4/4 Passing |
| 6. Event Handling | 6 | ✅ 6/6 Passing |
| 7. Timing | 5 | ✅ 5/5 Passing |
| 8. Integration | 1 | ✅ 1/1 Passing |
| 9. Clipboard API | 25 | ✅ 25/25 Passing |
| **10. Monospace Font** | **4** | **🔴 0/4 Failing** |
| **11. Text Span API** | **5** | **🔴 0/5 Failing** |
| **12. GPU Rendering** | **5** | **🔴 0/5 Failing** |
| **Total** | **142** | **128/142 (14 new tests: 0 passing; 1 gpu_tests-gated)** |

---

## Implementation Plan (GREEN Phase)

Once tests are written and confirmed RED, implement in this order:

### Step 1: Monospace Font (Phase 10)

1. Download `NotoSansMono-Regular.ttf` and add to `runtime/native/wgpu/assets/fonts/`
2. Add `static NOTO_SANS_MONO: &[u8] = include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf");`
3. In `TextSystem::new()`: `font_system.db_mut().load_font_data(NOTO_SANS_MONO.to_vec());`
4. Add `font_family: FontFamily` field to `StyleProperties` (default `SansSerif`)
5. In CSS parser (line ~2080): add `"font-family"` case → parse `"monospace"` → `Family::Monospace`
6. In `render_text()` (line ~725): pass `element.styles.font_family` into `Attrs::new().family(...)`
7. Run Phase 10 tests → GREEN

### Step 2: Text Span API (Phase 11)

1. Define `#[repr(C)] pub struct TextSpan { start: u32, end: u32, r: u8, g: u8, b: u8, a: u8 }`
2. Add `text_spans: Vec<TextSpan>` field to `Element` struct (line ~343)
3. Implement `native_set_text_spans(elem, spans, count)`:
   - Lock STATE, find element, clone spans into `element.text_spans`
   - count == 0 → clear the vec
4. Extend `TextSystem::render_text()` to accept `&[TextSpan]` and use cosmic-text per-run `Attrs` with color
5. Pass `element.text_spans.as_slice()` through render pipeline (TextRenderCommand → render_text)
6. Update `draw_glyph_to_framebuffer` to accept per-glyph color (already has `TextGlyph.color`, just needs span routing)
7. Run Phase 11 tests → GREEN

### Step 3: GPU Activation (Phase 12)

1. Add `native_set_render_mode(window, mode) -> i32` and `native_get_render_mode(window) -> i32`
2. `set_render_mode(GPU)`: call `initialize_gpu(window)` if `gpu_state.is_none()`, set `render_mode = Gpu`, return 0/-1
3. Modify `native_render(window)` to branch on `render_mode`:
   - Software: existing `render_to_framebuffer()` path
   - GPU: `collect_gpu_instances()` + upload + submit (extract from event loop's RedrawRequested handler)
4. `native_sample_pixel` must read from GPU framebuffer in GPU mode (readback via wgpu buffer copy)
5. Run Phase 12 tests → GREEN (non-gpu_tests tests first, then GPU equivalence with feature flag)

---

## Baseline Verification

Before adding new tests to `lib.rs`, run:

```bash
cd /home/lilith/development/projects/qliphoth/runtime/native/wgpu
cargo test -- --test-threads=1
```

All 78 existing tests must pass. Only then add Phase 10-12 tests and confirm RED.

When tests reveal spec gaps, **STOP and update NATIVE-RENDERING-SPEC.md v0.3.x**.
