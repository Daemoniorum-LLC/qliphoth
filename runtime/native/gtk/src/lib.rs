//! GTK4 Native Backend for Qliphoth
//!
//! This crate provides the native FFI functions that implement the
//! NativePlatform interface defined in Qliphoth's platform/native.sigil.
//!
//! # Architecture
//!
//! ```text
//! Sigil Code → NativePlatform (FFI) → This Crate → GTK4 → Native Window
//! ```
//!
//! # Usage
//!
//! Link this library with your Sigil application when building for native:
//!
//! ```bash
//! sigil build --target native --link qliphoth-native-gtk
//! ```

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};
use glib::MainContext;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::sync::Mutex;

// =============================================================================
// Global State
// =============================================================================

/// Widget registry - maps handles to GTK widgets
static WIDGETS: Lazy<Mutex<HashMap<usize, gtk4::Widget>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Window registry
static WINDOWS: Lazy<Mutex<HashMap<usize, ApplicationWindow>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Next handle ID
static NEXT_HANDLE: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(1));

/// Event queue for polling
static EVENT_QUEUE: Lazy<Mutex<Vec<NativeEvent>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// Callback registry - maps callback IDs to (widget_handle, event_type)
static CALLBACKS: Lazy<Mutex<HashMap<u64, (usize, i32)>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// GTK Application instance
static APP: Lazy<Mutex<Option<Application>>> = Lazy::new(|| Mutex::new(None));

// =============================================================================
// Event Types
// =============================================================================

#[derive(Clone, Debug)]
enum NativeEvent {
    Click { x: i32, y: i32, callback_id: u64 },
    KeyDown { key: u32, modifiers: u32, callback_id: u64 },
    KeyUp { key: u32, modifiers: u32, callback_id: u64 },
    MouseMove { x: i32, y: i32 },
    Resize { width: i32, height: i32 },
    Close,
    Redraw,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn allocate_handle() -> usize {
    let mut handle = NEXT_HANDLE.lock().unwrap();
    let h = *handle;
    *handle += 1;
    h
}

fn c_str_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

/// Map HTML tag to GTK widget type
fn tag_to_widget(tag: &str) -> gtk4::Widget {
    match tag {
        "div" | "section" | "article" | "main" | "header" | "footer" | "nav" => {
            GtkBox::new(Orientation::Vertical, 0).upcast()
        }
        "span" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "label" => {
            Label::new(None).upcast()
        }
        "button" => Button::new().upcast(),
        "input" => gtk4::Entry::new().upcast(),
        "textarea" => gtk4::TextView::new().upcast(),
        "img" => gtk4::Picture::new().upcast(),
        // Default to a box for unknown tags
        _ => GtkBox::new(Orientation::Vertical, 0).upcast(),
    }
}

// =============================================================================
// FFI Functions - Window Management
// =============================================================================

/// Create a new native window
#[no_mangle]
pub extern "C" fn native_create_window(
    title: *const c_char,
    width: c_int,
    height: c_int,
) -> usize {
    let title = c_str_to_string(title);
    let handle = allocate_handle();

    // Initialize GTK if needed
    let mut app_guard = APP.lock().unwrap();
    if app_guard.is_none() {
        let app = Application::builder()
            .application_id("com.qliphoth.native")
            .build();
        *app_guard = Some(app);
    }

    let app = app_guard.as_ref().unwrap().clone();
    drop(app_guard);

    // Create window on the main thread
    let title_clone = title.clone();
    glib::idle_add_local_once(move || {
        let window = ApplicationWindow::builder()
            .application(&app)
            .title(&title_clone)
            .default_width(width)
            .default_height(height)
            .build();

        // Create root container
        let container = GtkBox::new(Orientation::Vertical, 0);
        window.set_child(Some(&container));

        // Store window
        WINDOWS.lock().unwrap().insert(handle, window.clone());

        // Store root container as widget
        let container_handle = allocate_handle();
        WIDGETS.lock().unwrap().insert(container_handle, container.upcast());

        window.present();
    });

    handle
}

/// Destroy a window
#[no_mangle]
pub extern "C" fn native_destroy_window(handle: usize) {
    if let Some(window) = WINDOWS.lock().unwrap().remove(&handle) {
        glib::idle_add_local_once(move || {
            window.close();
        });
    }
}

/// Get window size
#[no_mangle]
pub extern "C" fn native_window_size(
    handle: usize,
    width: *mut c_int,
    height: *mut c_int,
) {
    if let Some(window) = WINDOWS.lock().unwrap().get(&handle) {
        let (w, h) = window.default_size();
        unsafe {
            *width = w;
            *height = h;
        }
    }
}

/// Set window title
#[no_mangle]
pub extern "C" fn native_set_window_title(handle: usize, title: *const c_char) {
    let title = c_str_to_string(title);
    if let Some(window) = WINDOWS.lock().unwrap().get(&handle) {
        let window = window.clone();
        glib::idle_add_local_once(move || {
            window.set_title(Some(&title));
        });
    }
}

// =============================================================================
// FFI Functions - Widget Creation
// =============================================================================

/// Create a widget from an HTML-like tag
#[no_mangle]
pub extern "C" fn native_create_widget(window: usize, tag: *const c_char) -> usize {
    let tag = c_str_to_string(tag);
    let handle = allocate_handle();

    glib::idle_add_local_once(move || {
        let widget = tag_to_widget(&tag);
        WIDGETS.lock().unwrap().insert(handle, widget);
    });

    handle
}

/// Create a text node
#[no_mangle]
pub extern "C" fn native_create_text(_window: usize, content: *const c_char) -> usize {
    let content = c_str_to_string(content);
    let handle = allocate_handle();

    glib::idle_add_local_once(move || {
        let label = Label::new(Some(&content));
        WIDGETS.lock().unwrap().insert(handle, label.upcast());
    });

    handle
}

/// Destroy a widget
#[no_mangle]
pub extern "C" fn native_destroy_widget(handle: usize) {
    if let Some(widget) = WIDGETS.lock().unwrap().remove(&handle) {
        glib::idle_add_local_once(move || {
            widget.unparent();
        });
    }
}

// =============================================================================
// FFI Functions - Widget Tree Manipulation
// =============================================================================

/// Append child widget to parent
#[no_mangle]
pub extern "C" fn native_append_child(parent: usize, child: usize) {
    let widgets = WIDGETS.lock().unwrap();
    if let (Some(parent_widget), Some(child_widget)) = (widgets.get(&parent), widgets.get(&child)) {
        let parent = parent_widget.clone();
        let child = child_widget.clone();
        drop(widgets);

        glib::idle_add_local_once(move || {
            if let Some(container) = parent.downcast_ref::<GtkBox>() {
                container.append(&child);
            }
        });
    }
}

/// Remove child widget from parent
#[no_mangle]
pub extern "C" fn native_remove_child(parent: usize, child: usize) {
    let widgets = WIDGETS.lock().unwrap();
    if let (Some(parent_widget), Some(child_widget)) = (widgets.get(&parent), widgets.get(&child)) {
        let parent = parent_widget.clone();
        let child = child_widget.clone();
        drop(widgets);

        glib::idle_add_local_once(move || {
            if let Some(container) = parent.downcast_ref::<GtkBox>() {
                container.remove(&child);
            }
        });
    }
}

/// Insert child before another widget
#[no_mangle]
pub extern "C" fn native_insert_before(parent: usize, child: usize, before: usize) {
    let widgets = WIDGETS.lock().unwrap();
    if let (Some(parent_widget), Some(child_widget), Some(before_widget)) =
        (widgets.get(&parent), widgets.get(&child), widgets.get(&before))
    {
        let parent = parent_widget.clone();
        let child = child_widget.clone();
        let before = before_widget.clone();
        drop(widgets);

        glib::idle_add_local_once(move || {
            if let Some(container) = parent.downcast_ref::<GtkBox>() {
                container.insert_child_after(&child, Some(&before));
            }
        });
    }
}

// =============================================================================
// FFI Functions - Widget Attributes
// =============================================================================

/// Set widget attribute
#[no_mangle]
pub extern "C" fn native_set_attribute(
    widget: usize,
    name: *const c_char,
    value: *const c_char,
) {
    let name = c_str_to_string(name);
    let value = c_str_to_string(value);

    if let Some(widget) = WIDGETS.lock().unwrap().get(&widget).cloned() {
        glib::idle_add_local_once(move || {
            match name.as_str() {
                "title" | "label" => {
                    if let Some(button) = widget.downcast_ref::<Button>() {
                        button.set_label(&value);
                    } else if let Some(label) = widget.downcast_ref::<Label>() {
                        label.set_label(&value);
                    }
                }
                "placeholder" => {
                    if let Some(entry) = widget.downcast_ref::<gtk4::Entry>() {
                        entry.set_placeholder_text(Some(&value));
                    }
                }
                "value" => {
                    if let Some(entry) = widget.downcast_ref::<gtk4::Entry>() {
                        entry.set_text(&value);
                    }
                }
                "visible" => {
                    widget.set_visible(value == "true" || value == "1");
                }
                "sensitive" | "enabled" => {
                    widget.set_sensitive(value == "true" || value == "1");
                }
                _ => {
                    // Unknown attribute - could add CSS class handling here
                }
            }
        });
    }
}

/// Remove widget attribute
#[no_mangle]
pub extern "C" fn native_remove_attribute(widget: usize, name: *const c_char) {
    let name = c_str_to_string(name);

    if let Some(widget) = WIDGETS.lock().unwrap().get(&widget).cloned() {
        glib::idle_add_local_once(move || {
            match name.as_str() {
                "placeholder" => {
                    if let Some(entry) = widget.downcast_ref::<gtk4::Entry>() {
                        entry.set_placeholder_text(None);
                    }
                }
                _ => {}
            }
        });
    }
}

/// Set text content of a widget
#[no_mangle]
pub extern "C" fn native_set_text_content(widget: usize, content: *const c_char) {
    let content = c_str_to_string(content);

    if let Some(widget) = WIDGETS.lock().unwrap().get(&widget).cloned() {
        glib::idle_add_local_once(move || {
            if let Some(label) = widget.downcast_ref::<Label>() {
                label.set_label(&content);
            } else if let Some(button) = widget.downcast_ref::<Button>() {
                button.set_label(&content);
            }
        });
    }
}

/// Set style property (CSS-like)
#[no_mangle]
pub extern "C" fn native_set_style(
    widget: usize,
    property: *const c_char,
    value: *const c_char,
) {
    let property = c_str_to_string(property);
    let value = c_str_to_string(value);

    if let Some(widget) = WIDGETS.lock().unwrap().get(&widget).cloned() {
        glib::idle_add_local_once(move || {
            // GTK uses CSS for styling
            // For now, just apply some common properties via widget methods
            match property.as_str() {
                "margin" => {
                    if let Ok(margin) = value.parse::<i32>() {
                        widget.set_margin_top(margin);
                        widget.set_margin_bottom(margin);
                        widget.set_margin_start(margin);
                        widget.set_margin_end(margin);
                    }
                }
                "width" => {
                    if let Ok(width) = value.parse::<i32>() {
                        widget.set_width_request(width);
                    }
                }
                "height" => {
                    if let Ok(height) = value.parse::<i32>() {
                        widget.set_height_request(height);
                    }
                }
                _ => {
                    // Could add GTK CSS class handling here
                }
            }
        });
    }
}

// =============================================================================
// FFI Functions - Event Handling
// =============================================================================

/// Add event listener
#[no_mangle]
pub extern "C" fn native_add_event_listener(
    widget: usize,
    event_type: c_int,
    callback_id: u64,
) {
    CALLBACKS.lock().unwrap().insert(callback_id, (widget, event_type));

    if let Some(widget) = WIDGETS.lock().unwrap().get(&widget).cloned() {
        glib::idle_add_local_once(move || {
            match event_type {
                0 => {
                    // Click
                    if let Some(button) = widget.downcast_ref::<Button>() {
                        button.connect_clicked(move |_| {
                            EVENT_QUEUE.lock().unwrap().push(NativeEvent::Click {
                                x: 0,
                                y: 0,
                                callback_id,
                            });
                        });
                    }
                }
                10 => {
                    // KeyDown - would need keyboard controller
                }
                _ => {}
            }
        });
    }
}

/// Remove event listener
#[no_mangle]
pub extern "C" fn native_remove_event_listener(
    _widget: usize,
    _event_type: c_int,
    callback_id: u64,
) {
    CALLBACKS.lock().unwrap().remove(&callback_id);
    // Note: GTK doesn't easily support removing signal handlers by ID
    // Would need to store signal handler IDs for proper cleanup
}

// =============================================================================
// FFI Functions - Event Loop
// =============================================================================

/// Poll for events (non-blocking)
/// Returns event type code, -1 if no events
#[no_mangle]
pub extern "C" fn native_poll_events() -> c_int {
    // Process pending GTK events
    while MainContext::default().iteration(false) {}

    // Check our event queue
    if let Some(event) = EVENT_QUEUE.lock().unwrap().pop() {
        match event {
            NativeEvent::Click { .. } => 0,
            NativeEvent::KeyDown { .. } => 1,
            NativeEvent::Resize { .. } => 2,
            NativeEvent::Close => 3,
            NativeEvent::Redraw => 4,
            _ => -1,
        }
    } else {
        -1
    }
}

/// Get event data (for the last polled event)
#[no_mangle]
pub extern "C" fn native_get_event_data(_out_data: *mut u8, _max_len: usize) -> usize {
    // TODO: Serialize event data
    0
}

/// Run the main event loop (blocking)
#[no_mangle]
pub extern "C" fn native_run_event_loop() {
    if let Some(app) = APP.lock().unwrap().as_ref() {
        app.run();
    }
}

/// Request window redraw
#[no_mangle]
pub extern "C" fn native_request_redraw(handle: usize) {
    if let Some(window) = WINDOWS.lock().unwrap().get(&handle) {
        let window = window.clone();
        glib::idle_add_local_once(move || {
            window.queue_draw();
        });
    }
}

// =============================================================================
// FFI Functions - Timing
// =============================================================================

/// Set a timeout
#[no_mangle]
pub extern "C" fn native_set_timeout(callback_id: u64, delay_ms: u64) -> u64 {
    let source_id = glib::timeout_add_local_once(
        std::time::Duration::from_millis(delay_ms),
        move || {
            // Would dispatch message to Sigil runtime here
            println!("Timeout fired: callback_id={}", callback_id);
        },
    );
    source_id.as_raw() as u64
}

/// Clear a timeout
#[no_mangle]
pub extern "C" fn native_clear_timeout(timer_id: u64) {
    // GTK source IDs can't be cancelled once scheduled without storing them
    let _ = timer_id;
}

/// Request animation frame
#[no_mangle]
pub extern "C" fn native_request_animation_frame(callback_id: u64) -> u64 {
    // Use idle_add for frame-like behavior
    let source_id = glib::idle_add_local_once(move || {
        println!("Animation frame: callback_id={}", callback_id);
    });
    source_id.as_raw() as u64
}

/// Cancel animation frame
#[no_mangle]
pub extern "C" fn native_cancel_animation_frame(frame_id: u64) {
    let _ = frame_id;
}

// =============================================================================
// FFI Functions - Clipboard
// =============================================================================

/// Read from clipboard
#[no_mangle]
pub extern "C" fn native_clipboard_read(_out_buf: *mut c_char, _max_len: usize) -> usize {
    // TODO: Implement clipboard access
    0
}

/// Write to clipboard
#[no_mangle]
pub extern "C" fn native_clipboard_write(_content: *const c_char) {
    // TODO: Implement clipboard access
}
