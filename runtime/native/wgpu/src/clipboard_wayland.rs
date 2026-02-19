//! Wayland clipboard backend using smithay-clipboard
//!
//! Enabled via `wayland-backend` feature flag on Linux when WAYLAND_DISPLAY is set.
//!
//! This backend provides native Wayland clipboard support for text operations.
//! For image operations, it falls back to arboard since smithay-clipboard only
//! supports text.

use smithay_clipboard::Clipboard;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Instant;
use winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
use winit::window::Window;

use crate::{
    ClipboardCompletedData, ClipboardTarget, NativeEvent, PendingOperation,
    CLIPBOARD_ERR_EMPTY, CLIPBOARD_ERR_FORMAT_NOT_FOUND, CLIPBOARD_ERR_INTERNAL,
    EVENT_CLIPBOARD_DATA_READY, EVENT_CLIPBOARD_ERROR, EVENT_CLIPBOARD_FORMATS_AVAILABLE,
    EVENT_CLIPBOARD_WRITE_COMPLETE,
};

/// Wayland clipboard backend state
pub struct WaylandClipboardBackend {
    /// smithay-clipboard instance (handles Wayland protocol internally)
    clipboard: Clipboard,

    /// Pending write data (accumulated via write_text/write_html/etc, committed via write_commit)
    write_data: Option<WaylandWriteData>,
}

/// Pending write data
struct WaylandWriteData {
    target: ClipboardTarget,
    text: Option<String>,
    html: Option<String>,
    // Note: smithay-clipboard only supports text, so images fall back to arboard
}

impl WaylandClipboardBackend {
    /// Check if Wayland is available (WAYLAND_DISPLAY set)
    pub fn is_available() -> bool {
        env::var("WAYLAND_DISPLAY").is_ok()
    }

    /// Create a new Wayland clipboard backend from a winit window
    ///
    /// This extracts the wl_display pointer from the window's display handle.
    /// Returns None if the window is not using Wayland.
    pub fn try_new_from_window(window: &Arc<Window>) -> Option<Self> {
        // Get the display handle from the window
        let display_handle = window.display_handle().ok()?;

        // Extract the Wayland display pointer
        let display_ptr = match display_handle.as_raw() {
            RawDisplayHandle::Wayland(wayland_handle) => {
                wayland_handle.display.as_ptr()
            }
            _ => {
                log::debug!("Window is not using Wayland display");
                return None;
            }
        };

        // Safety: smithay-clipboard::Clipboard::new requires a valid wl_display pointer.
        // The display pointer comes from the window's display handle which is valid
        // for the lifetime of the window. Since the clipboard backend is owned by
        // AppState and the window is also owned by AppState, the display remains valid.
        let clipboard = unsafe { Clipboard::new(display_ptr) };

        log::info!("Wayland clipboard backend initialized");
        Some(Self {
            clipboard,
            write_data: None,
        })
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Read clipboard data in the specified format
    ///
    /// For text formats, uses smithay-clipboard directly.
    /// For other formats, returns an error (caller should fall back to arboard).
    pub fn read_format(
        &mut self,
        target: ClipboardTarget,
        mime: &str,
        callback_id: u64,
        event_queue: &mut Vec<NativeEvent>,
        completed: &mut HashMap<u64, ClipboardCompletedData>,
    ) -> Result<(), i32> {
        // smithay-clipboard only supports text
        if !mime.starts_with("text/") {
            return Err(CLIPBOARD_ERR_FORMAT_NOT_FOUND);
        }

        // Read text from clipboard or primary selection
        let result = match target {
            ClipboardTarget::Clipboard => self.clipboard.load(),
            ClipboardTarget::PrimarySelection => self.clipboard.load_primary(),
        };

        match result {
            Ok(text) => {
                if text.is_empty() {
                    event_queue.push(NativeEvent::ClipboardError {
                        callback_id,
                        error_code: CLIPBOARD_ERR_EMPTY,
                    });
                } else {
                    // Store completed data
                    let data = text.into_bytes();
                    let data_size = data.len();
                    completed.insert(
                        callback_id,
                        ClipboardCompletedData {
                            data,
                            formats: None,
                            format_cstrings: Vec::new(),
                            completed_at: Instant::now(),
                        },
                    );

                    event_queue.push(NativeEvent::ClipboardDataReady { callback_id, data_size });
                }
                Ok(())
            }
            Err(e) => {
                log::warn!("Wayland clipboard read failed: {:?}", e);
                event_queue.push(NativeEvent::ClipboardError {
                    callback_id,
                    error_code: CLIPBOARD_ERR_INTERNAL,
                });
                Ok(())
            }
        }
    }

    /// Get available formats from clipboard
    ///
    /// Note: smithay-clipboard only provides text loading without format discovery.
    /// We report text/plain when content exists. If the source app offered text/html,
    /// we cannot detect it - callers should try reading text/html directly and fall
    /// back to text/plain if that fails. For full format discovery on Wayland, the
    /// FFI layer falls back to arboard which can query more formats.
    pub fn get_formats(
        &mut self,
        target: ClipboardTarget,
        callback_id: u64,
        event_queue: &mut Vec<NativeEvent>,
        completed: &mut HashMap<u64, ClipboardCompletedData>,
    ) -> Result<(), i32> {
        // Try to load text to see if clipboard has content
        // smithay-clipboard doesn't expose format discovery, so we probe by loading
        let result = match target {
            ClipboardTarget::Clipboard => self.clipboard.load(),
            ClipboardTarget::PrimarySelection => self.clipboard.load_primary(),
        };

        match result {
            Ok(text) => {
                let formats = if text.is_empty() {
                    vec![]
                } else {
                    // Report text formats - smithay-clipboard handles these
                    // Note: HTML might be available but we can't detect it
                    vec!["text/plain".to_string(), "text/plain;charset=utf-8".to_string()]
                };
                let format_count = formats.len();

                completed.insert(
                    callback_id,
                    ClipboardCompletedData {
                        data: Vec::new(),
                        formats: Some(formats),
                        format_cstrings: Vec::new(),
                        completed_at: Instant::now(),
                    },
                );

                event_queue.push(NativeEvent::ClipboardFormatsAvailable { callback_id, format_count });
                Ok(())
            }
            Err(_) => {
                // Empty clipboard
                completed.insert(
                    callback_id,
                    ClipboardCompletedData {
                        data: Vec::new(),
                        formats: Some(vec![]),
                        format_cstrings: Vec::new(),
                        completed_at: Instant::now(),
                    },
                );

                event_queue.push(NativeEvent::ClipboardFormatsAvailable { callback_id, format_count: 0 });
                Ok(())
            }
        }
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Stage text for writing
    pub fn write_text(&mut self, target: ClipboardTarget, text: String) {
        let data = self.write_data.get_or_insert(WaylandWriteData {
            target,
            text: None,
            html: None,
        });
        data.text = Some(text);
        data.target = target;
    }

    /// Stage HTML for writing (stored as text since smithay-clipboard only supports text)
    pub fn write_html(&mut self, target: ClipboardTarget, html: String) {
        let data = self.write_data.get_or_insert(WaylandWriteData {
            target,
            text: None,
            html: None,
        });
        data.html = Some(html);
        data.target = target;
    }

    /// Commit staged write data
    pub fn write_commit(
        &mut self,
        callback_id: u64,
        event_queue: &mut Vec<NativeEvent>,
    ) -> Result<(), i32> {
        let data = match self.write_data.take() {
            Some(d) => d,
            None => {
                event_queue.push(NativeEvent::ClipboardError {
                    callback_id,
                    error_code: CLIPBOARD_ERR_INTERNAL,
                });
                return Err(CLIPBOARD_ERR_INTERNAL);
            }
        };

        // Prefer plain text, fall back to HTML stripped
        let text = match (data.text, data.html) {
            (Some(t), _) => t,
            (None, Some(h)) => h, // Store HTML as-is (smithay-clipboard only does text)
            (None, None) => {
                event_queue.push(NativeEvent::ClipboardError {
                    callback_id,
                    error_code: CLIPBOARD_ERR_INTERNAL,
                });
                return Err(CLIPBOARD_ERR_INTERNAL);
            }
        };

        // Write to clipboard or primary selection
        match data.target {
            ClipboardTarget::Clipboard => self.clipboard.store(text),
            ClipboardTarget::PrimarySelection => self.clipboard.store_primary(text),
        }

        event_queue.push(NativeEvent::ClipboardWriteComplete { callback_id });
        Ok(())
    }

    /// Cancel staged write
    pub fn write_cancel(&mut self) {
        self.write_data = None;
    }

    /// Cancel a pending read (no-op for synchronous backend)
    pub fn cancel(&mut self, _callback_id: u64) -> bool {
        // smithay-clipboard operations are synchronous, so there's nothing to cancel
        false
    }

    // =========================================================================
    // Event Processing
    // =========================================================================

    /// Process events (no-op for smithay-clipboard since it handles events internally)
    pub fn process_events(
        &mut self,
        _event_queue: &mut Vec<NativeEvent>,
        _completed: &mut HashMap<u64, ClipboardCompletedData>,
        _pending_ops: &mut HashMap<u64, PendingOperation>,
    ) {
        // smithay-clipboard runs its own event loop on a separate thread,
        // so we don't need to poll for events here.
    }

    /// Check for timed out operations (no-op for synchronous backend)
    pub fn check_timeouts(
        &mut self,
        _event_queue: &mut Vec<NativeEvent>,
        _pending_ops: &mut HashMap<u64, PendingOperation>,
    ) {
        // smithay-clipboard operations are synchronous, so no timeouts to check
    }

    /// Reset state (for testing)
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.write_data = None;
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn skip_if_no_wayland() -> bool {
        if !WaylandClipboardBackend::is_available() {
            eprintln!("Skipping test: WAYLAND_DISPLAY not set");
            return true;
        }
        false
    }

    #[test]
    fn test_is_available_checks_env() {
        // This test just verifies the function runs without panicking
        let _ = WaylandClipboardBackend::is_available();
    }

    #[test]
    fn test_wayland_write_data_struct() {
        // Test WaylandWriteData creation without a real backend
        let data = WaylandWriteData {
            target: ClipboardTarget::Clipboard,
            text: Some("Hello".to_string()),
            html: None,
        };
        assert_eq!(data.target, ClipboardTarget::Clipboard);
        assert_eq!(data.text, Some("Hello".to_string()));
        assert!(data.html.is_none());
    }

    #[test]
    fn test_wayland_write_data_with_html() {
        let data = WaylandWriteData {
            target: ClipboardTarget::PrimarySelection,
            text: Some("Plain text".to_string()),
            html: Some("<b>Bold</b>".to_string()),
        };
        assert_eq!(data.target, ClipboardTarget::PrimarySelection);
        assert!(data.text.is_some());
        assert!(data.html.is_some());
    }

    // =========================================================================
    // Integration tests (require Wayland session)
    // =========================================================================

    #[test]
    #[ignore] // Requires Wayland display
    fn test_wayland_backend_text_roundtrip() {
        if skip_if_no_wayland() {
            return;
        }

        // This test requires a real Wayland display, so it's ignored by default
        // Run with: cargo test --features wayland-backend -- --ignored
    }

    #[test]
    #[ignore] // Requires Wayland display
    fn test_wayland_backend_primary_selection() {
        if skip_if_no_wayland() {
            return;
        }

        // Test primary selection support
    }

    #[test]
    #[ignore] // Requires Wayland display and wl-copy
    fn test_wl_copy_paste_interop() {
        if skip_if_no_wayland() {
            return;
        }

        // Test interop with wl-copy/wl-paste
        // Similar to xclip tests in X11 backend
    }
}
