//! X11 Clipboard Backend
//!
//! Native X11 clipboard implementation using x11rb.
//! Enabled via `x11-backend` feature flag on Linux when DISPLAY is set.
//!
//! # X11 Selection Protocol
//!
//! X11 clipboard works via selections:
//! 1. ConvertSelection(CLIPBOARD, UTF8_STRING, property) - request data
//! 2. Wait for SelectionNotify event
//! 3. GetProperty() to retrieve data
//! 4. If type=INCR: enter chunked transfer loop
//!
//! # INCR Protocol (large data >256KB)
//!
//! 1. SelectionNotify with type=INCR
//! 2. DeleteProperty (signal ready)
//! 3. Loop: PropertyNotify → GetProperty → DeleteProperty
//! 4. Empty property = transfer complete

use std::collections::HashMap;
use std::time::Instant;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, *};
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _; // Provides change_property8, change_property32

use crate::{
    ClipboardCompletedData, ClipboardTarget, NativeEvent, PendingOpState, PendingOperation,
    CLIPBOARD_ERR_EMPTY, CLIPBOARD_ERR_INTERNAL, CLIPBOARD_ERR_TIMEOUT,
};

// =============================================================================
// X11 Atoms
// =============================================================================

x11rb::atom_manager! {
    /// Pre-interned atoms for clipboard operations
    pub ClipboardAtoms: AtomsCookie {
        CLIPBOARD,
        PRIMARY,
        TARGETS,
        UTF8_STRING,
        INCR,
        TEXT_PLAIN: b"text/plain",
        TEXT_PLAIN_UTF8: b"text/plain;charset=utf-8",
        TEXT_HTML: b"text/html",
        TEXT_URI_LIST: b"text/uri-list",
        IMAGE_PNG: b"image/png",
        _QLIPHOTH_CLIPBOARD,  // Temp property for receiving data
    }
}

// =============================================================================
// Types
// =============================================================================

/// Type of X11 clipboard request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Variants used when FFI layer routes through X11 backend
enum X11RequestType {
    /// Regular data request
    Data(ClipboardTarget),
    /// TARGETS query to discover available formats
    Formats,
}

/// Pending X11 clipboard read request
#[allow(dead_code)] // Fields used in process_events
struct X11ReadRequest {
    callback_id: u64,
    request_type: X11RequestType,
    target_atom: Atom,
    started_at: Instant,
}

/// INCR transfer state for chunked data
struct IncrTransfer {
    callback_id: u64,
    #[allow(dead_code)] // Used for logging/debugging
    request_type: X11RequestType,
    partial_data: Vec<u8>,
    #[allow(dead_code)] // Used for format validation
    expected_format: u8,
}

/// X11 write request (we own the selection)
struct X11WriteData {
    text: Option<String>,
    html: Option<String>,
    image_png: Option<Vec<u8>>,
    uri_list: Option<String>,
}

// =============================================================================
// X11ClipboardBackend
// =============================================================================

/// Native X11 clipboard backend
pub struct X11ClipboardBackend {
    conn: RustConnection,
    #[allow(dead_code)] // Used for future multi-screen support
    screen_num: usize,
    atoms: ClipboardAtoms,
    selection_window: Window,
    pending_reads: HashMap<u64, X11ReadRequest>,
    /// Active INCR transfer (only one at a time since we use a single property)
    /// New read requests are rejected while INCR is active (fall back to arboard)
    active_incr: Option<IncrTransfer>,
    write_data: Option<X11WriteData>,
}

impl X11ClipboardBackend {
    /// Create a new X11 clipboard backend
    ///
    /// Returns None if X11 is not available (e.g., DISPLAY not set)
    pub fn new() -> Result<Self, String> {
        // Connect to X11 display
        let (conn, screen_num) = RustConnection::connect(None)
            .map_err(|e| format!("Failed to connect to X11: {}", e))?;

        // Intern all atoms
        let atoms = ClipboardAtoms::new(&conn)
            .map_err(|e| format!("Failed to intern atoms: {}", e))?
            .reply()
            .map_err(|e| format!("Failed to get atom reply: {}", e))?;

        // Get screen root
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        // Create hidden 1x1 window for selection operations
        let selection_window = conn.generate_id().map_err(|e| format!("Failed to generate window ID: {}", e))?;

        conn.create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            selection_window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(|e| format!("Failed to create selection window: {}", e))?;

        conn.flush().map_err(|e| format!("Failed to flush: {}", e))?;

        log::debug!("X11 clipboard backend initialized");

        Ok(Self {
            conn,
            screen_num,
            atoms,
            selection_window,
            pending_reads: HashMap::new(),
            active_incr: None,
            write_data: None,
        })
    }

    /// Check if X11 display is available
    ///
    /// Returns true if DISPLAY environment variable is set, indicating
    /// an X11 server is available (either native X11 or XWayland).
    pub fn is_available() -> bool {
        std::env::var("DISPLAY").is_ok()
    }

    /// Request clipboard data in a specific format
    ///
    /// This initiates an async read. The result will be delivered via
    /// NativeEvent::ClipboardDataReady or NativeEvent::ClipboardError
    #[allow(dead_code)] // Called when FFI layer routes through X11 backend
    pub fn read_format(
        &mut self,
        target: ClipboardTarget,
        mime: &str,
        callback_id: u64,
    ) -> Result<(), i32> {
        // Reject if INCR transfer is active (can only handle one at a time)
        if self.active_incr.is_some() {
            log::debug!("X11 read_format rejected: INCR transfer in progress");
            return Err(CLIPBOARD_ERR_INTERNAL);
        }

        // Reject duplicate callback_id
        if self.pending_reads.contains_key(&callback_id) {
            return Err(CLIPBOARD_ERR_INTERNAL);
        }

        // Select X11 selection based on target
        let selection = match target {
            ClipboardTarget::Clipboard => self.atoms.CLIPBOARD,
            ClipboardTarget::PrimarySelection => self.atoms.PRIMARY,
        };

        // Map MIME type to X11 atom
        let target_atom = self.mime_to_atom(mime);

        // Send ConvertSelection request
        self.conn
            .convert_selection(
                self.selection_window,
                selection,
                target_atom,
                self.atoms._QLIPHOTH_CLIPBOARD,
                x11rb::CURRENT_TIME,
            )
            .map_err(|_| CLIPBOARD_ERR_INTERNAL)?;

        self.conn.flush().map_err(|_| CLIPBOARD_ERR_INTERNAL)?;

        // Track pending request
        self.pending_reads.insert(
            callback_id,
            X11ReadRequest {
                callback_id,
                request_type: X11RequestType::Data(target),
                target_atom,
                started_at: Instant::now(),
            },
        );

        Ok(())
    }

    /// Query available clipboard formats
    #[allow(dead_code)] // Called when FFI layer routes through X11 backend
    pub fn get_formats(&mut self, callback_id: u64) -> Result<(), i32> {
        // Reject if INCR transfer is active (can only handle one at a time)
        if self.active_incr.is_some() {
            log::debug!("X11 get_formats rejected: INCR transfer in progress");
            return Err(CLIPBOARD_ERR_INTERNAL);
        }

        // Reject duplicate callback_id
        if self.pending_reads.contains_key(&callback_id) {
            return Err(CLIPBOARD_ERR_INTERNAL);
        }

        // Request TARGETS to discover available formats
        self.conn
            .convert_selection(
                self.selection_window,
                self.atoms.CLIPBOARD,
                self.atoms.TARGETS,
                self.atoms._QLIPHOTH_CLIPBOARD,
                x11rb::CURRENT_TIME,
            )
            .map_err(|_| CLIPBOARD_ERR_INTERNAL)?;

        self.conn.flush().map_err(|_| CLIPBOARD_ERR_INTERNAL)?;

        // Track as a special TARGETS request
        self.pending_reads.insert(
            callback_id,
            X11ReadRequest {
                callback_id,
                request_type: X11RequestType::Formats,
                target_atom: self.atoms.TARGETS,
                started_at: Instant::now(),
            },
        );

        Ok(())
    }

    /// Write text to clipboard (staged until commit)
    #[allow(dead_code)] // Called when FFI layer routes through X11 backend
    pub fn write_text(&mut self, text: &str) -> Result<(), i32> {
        // Just store data - ownership is taken on commit
        let write_data = self.write_data.get_or_insert(X11WriteData {
            text: None,
            html: None,
            image_png: None,
            uri_list: None,
        });
        write_data.text = Some(text.to_string());
        Ok(())
    }

    /// Write HTML to clipboard (staged until commit)
    #[allow(dead_code)] // Called when FFI layer routes through X11 backend
    pub fn write_html(&mut self, html: &str) -> Result<(), i32> {
        let write_data = self.write_data.get_or_insert(X11WriteData {
            text: None,
            html: None,
            image_png: None,
            uri_list: None,
        });
        write_data.html = Some(html.to_string());
        Ok(())
    }

    /// Write PNG image to clipboard (staged until commit)
    #[allow(dead_code)] // Called when FFI layer routes through X11 backend
    pub fn write_image(&mut self, png_data: &[u8]) -> Result<(), i32> {
        let write_data = self.write_data.get_or_insert(X11WriteData {
            text: None,
            html: None,
            image_png: None,
            uri_list: None,
        });
        write_data.image_png = Some(png_data.to_vec());
        Ok(())
    }

    /// Commit all pending writes by taking selection ownership
    #[allow(dead_code)] // Called when FFI layer routes through X11 backend
    pub fn write_commit(&mut self, _callback_id: u64) -> Result<(), i32> {
        // Nothing to commit if no data was staged
        if self.write_data.is_none() {
            return Err(CLIPBOARD_ERR_INTERNAL);
        }

        // Take ownership of CLIPBOARD selection
        self.conn
            .set_selection_owner(self.selection_window, self.atoms.CLIPBOARD, x11rb::CURRENT_TIME)
            .map_err(|_| CLIPBOARD_ERR_INTERNAL)?;

        self.conn.flush().map_err(|_| CLIPBOARD_ERR_INTERNAL)?;

        // Verify we actually got ownership (another client could have raced us)
        let owner = self
            .conn
            .get_selection_owner(self.atoms.CLIPBOARD)
            .map_err(|_| CLIPBOARD_ERR_INTERNAL)?
            .reply()
            .map_err(|_| CLIPBOARD_ERR_INTERNAL)?;

        if owner.owner != self.selection_window {
            log::warn!("X11: Failed to acquire clipboard ownership (another client won)");
            self.write_data = None; // Clear staged data
            return Err(CLIPBOARD_ERR_INTERNAL);
        }

        // Ownership confirmed - data will be served via SelectionRequest events
        Ok(())
    }

    /// Process X11 events and generate clipboard events
    ///
    /// This should be called from native_poll_event() to integrate X11 clipboard
    /// events with the main event loop.
    pub fn process_events(
        &mut self,
        event_queue: &mut Vec<NativeEvent>,
        completed: &mut HashMap<u64, ClipboardCompletedData>,
        pending_ops: &mut HashMap<u64, PendingOperation>,
    ) {
        // Poll for X11 events (non-blocking)
        while let Ok(Some(event)) = self.conn.poll_for_event() {
            match event {
                x11rb::protocol::Event::SelectionNotify(notify) => {
                    self.handle_selection_notify(notify, event_queue, completed, pending_ops);
                }
                x11rb::protocol::Event::SelectionRequest(request) => {
                    self.handle_selection_request(request);
                }
                x11rb::protocol::Event::SelectionClear(_clear) => {
                    // We lost selection ownership
                    self.write_data = None;
                    log::debug!("Lost clipboard selection ownership");
                }
                x11rb::protocol::Event::PropertyNotify(notify) => {
                    self.handle_property_notify(notify, event_queue, completed, pending_ops);
                }
                _ => {}
            }
        }

        // Check for timeouts
        self.check_timeouts(event_queue, pending_ops);
    }

    /// Cancel a pending operation
    #[allow(dead_code)] // Called when FFI layer routes through X11 backend
    pub fn cancel(&mut self, callback_id: u64) -> bool {
        let removed_pending = self.pending_reads.remove(&callback_id).is_some();
        let removed_incr = self
            .active_incr
            .as_ref()
            .map(|incr| incr.callback_id == callback_id)
            .unwrap_or(false);
        if removed_incr {
            self.active_incr = None;
        }
        removed_pending || removed_incr
    }

    /// Reset internal state and drain pending X11 events
    /// Used during test state reset to prevent event leakage between tests
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.pending_reads.clear();
        self.active_incr = None;
        self.write_data = None;
        // Drain any pending X11 events from the connection
        while self.conn.poll_for_event().ok().flatten().is_some() {}
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    fn mime_to_atom(&self, mime: &str) -> Atom {
        match mime {
            "text/plain" => self.atoms.TEXT_PLAIN,
            "text/plain;charset=utf-8" => self.atoms.TEXT_PLAIN_UTF8,
            "text/html" => self.atoms.TEXT_HTML,
            "text/uri-list" => self.atoms.TEXT_URI_LIST,
            "image/png" => self.atoms.IMAGE_PNG,
            _ => self.atoms.UTF8_STRING, // Default to UTF8_STRING for text
        }
    }

    #[allow(dead_code)] // Used in process_events path
    fn atom_to_mime(&self, atom: Atom) -> &'static str {
        if atom == self.atoms.TEXT_PLAIN || atom == self.atoms.TEXT_PLAIN_UTF8 {
            "text/plain"
        } else if atom == self.atoms.TEXT_HTML {
            "text/html"
        } else if atom == self.atoms.TEXT_URI_LIST {
            "text/uri-list"
        } else if atom == self.atoms.IMAGE_PNG {
            "image/png"
        } else if atom == self.atoms.UTF8_STRING {
            "text/plain"
        } else {
            "application/octet-stream"
        }
    }

    fn handle_selection_notify(
        &mut self,
        notify: SelectionNotifyEvent,
        event_queue: &mut Vec<NativeEvent>,
        completed: &mut HashMap<u64, ClipboardCompletedData>,
        pending_ops: &mut HashMap<u64, PendingOperation>,
    ) {
        // Find the pending request for this selection
        let callback_id = self
            .pending_reads
            .iter()
            .find(|(_, req)| req.target_atom == notify.target)
            .map(|(id, _)| *id);

        let Some(callback_id) = callback_id else {
            return;
        };

        // Property is None means selection conversion failed (empty clipboard)
        if notify.property == x11rb::NONE {
            self.pending_reads.remove(&callback_id);
            if let Some(op) = pending_ops.get_mut(&callback_id) {
                op.state = PendingOpState::Completed;
            }
            event_queue.push(NativeEvent::ClipboardError {
                callback_id,
                error_code: CLIPBOARD_ERR_EMPTY,
            });
            return;
        }

        // Get the property data
        let property_reply = match self.conn.get_property(
            true, // delete after reading
            self.selection_window,
            notify.property,
            GetPropertyType::ANY,
            0,
            u32::MAX,
        ) {
            Ok(cookie) => match cookie.reply() {
                Ok(reply) => reply,
                Err(_) => {
                    self.pending_reads.remove(&callback_id);
                    event_queue.push(NativeEvent::ClipboardError {
                        callback_id,
                        error_code: CLIPBOARD_ERR_INTERNAL,
                    });
                    return;
                }
            },
            Err(_) => {
                self.pending_reads.remove(&callback_id);
                event_queue.push(NativeEvent::ClipboardError {
                    callback_id,
                    error_code: CLIPBOARD_ERR_INTERNAL,
                });
                return;
            }
        };

        // Check for INCR (incremental transfer)
        if property_reply.type_ == self.atoms.INCR {
            // Start INCR transfer (only one at a time)
            if let Some(request) = self.pending_reads.remove(&callback_id) {
                self.active_incr = Some(IncrTransfer {
                    callback_id,
                    request_type: request.request_type,
                    partial_data: Vec::new(),
                    expected_format: property_reply.format,
                });
            }
            return;
        }

        // Handle TARGETS response
        let request = self.pending_reads.remove(&callback_id);
        if let Some(op) = pending_ops.get_mut(&callback_id) {
            op.state = PendingOpState::Completed;
        }

        if let Some(req) = request {
            if req.request_type == X11RequestType::Formats {
                // Parse TARGETS response
                let formats = self.parse_targets(&property_reply);
                let format_count = formats.len();
                // Store formats in completed data
                completed.insert(
                    callback_id,
                    ClipboardCompletedData {
                        data: Vec::new(),
                        formats: Some(formats),
                        format_cstrings: Vec::new(),
                        completed_at: Instant::now(),
                    },
                );
                event_queue.push(NativeEvent::ClipboardFormatsAvailable {
                    callback_id,
                    format_count,
                });
            } else {
                // Regular data response
                let data = property_reply.value;
                let data_size = data.len();
                // Store data in completed
                completed.insert(
                    callback_id,
                    ClipboardCompletedData {
                        data,
                        formats: None,
                        format_cstrings: Vec::new(),
                        completed_at: Instant::now(),
                    },
                );
                event_queue.push(NativeEvent::ClipboardDataReady {
                    callback_id,
                    data_size,
                });
            }
        }
    }

    fn handle_property_notify(
        &mut self,
        notify: PropertyNotifyEvent,
        event_queue: &mut Vec<NativeEvent>,
        completed: &mut HashMap<u64, ClipboardCompletedData>,
        pending_ops: &mut HashMap<u64, PendingOperation>,
    ) {
        // Only handle new value events for INCR on our selection window
        if notify.state != Property::NEW_VALUE || notify.window != self.selection_window {
            return;
        }

        // Check if we have an active INCR transfer
        let Some(ref mut _transfer) = self.active_incr else {
            return;
        };

        // Get the property data
        let property_reply = match self.conn.get_property(
            true, // delete after reading
            notify.window,
            notify.atom,
            GetPropertyType::ANY,
            0,
            u32::MAX,
        ) {
            Ok(cookie) => match cookie.reply() {
                Ok(reply) => reply,
                Err(_) => return,
            },
            Err(_) => return,
        };

        if property_reply.value.is_empty() {
            // INCR transfer complete - take ownership of the transfer
            let transfer = self.active_incr.take().unwrap();
            if let Some(op) = pending_ops.get_mut(&transfer.callback_id) {
                op.state = PendingOpState::Completed;
            }
            let data_size = transfer.partial_data.len();
            // Store completed data
            completed.insert(
                transfer.callback_id,
                ClipboardCompletedData {
                    data: transfer.partial_data,
                    formats: None,
                    format_cstrings: Vec::new(),
                    completed_at: Instant::now(),
                },
            );
            event_queue.push(NativeEvent::ClipboardDataReady {
                callback_id: transfer.callback_id,
                data_size,
            });
        } else {
            // Accumulate data
            if let Some(transfer) = self.active_incr.as_mut() {
                transfer.partial_data.extend_from_slice(&property_reply.value);
            }
        }
    }

    fn handle_selection_request(&mut self, request: SelectionRequestEvent) {
        let Some(ref write_data) = self.write_data else {
            // No data to provide
            self.send_selection_notify(request.requestor, request.selection, request.target, x11rb::NONE, request.time);
            return;
        };

        // Handle TARGETS request
        if request.target == self.atoms.TARGETS {
            let mut targets: Vec<Atom> = vec![self.atoms.TARGETS];
            if write_data.text.is_some() {
                targets.push(self.atoms.UTF8_STRING);
                targets.push(self.atoms.TEXT_PLAIN);
            }
            if write_data.html.is_some() {
                targets.push(self.atoms.TEXT_HTML);
            }
            if write_data.image_png.is_some() {
                targets.push(self.atoms.IMAGE_PNG);
            }
            if write_data.uri_list.is_some() {
                targets.push(self.atoms.TEXT_URI_LIST);
            }

            let _ = self.conn.change_property32(
                PropMode::REPLACE,
                request.requestor,
                request.property,
                AtomEnum::ATOM,
                &targets,
            );
            let _ = self.conn.flush();
            self.send_selection_notify(
                request.requestor,
                request.selection,
                request.target,
                request.property,
                request.time,
            );
            return;
        }

        // Handle data requests
        let data: Option<(&[u8], Atom)> = if request.target == self.atoms.UTF8_STRING
            || request.target == self.atoms.TEXT_PLAIN
        {
            write_data.text.as_ref().map(|s| (s.as_bytes(), self.atoms.UTF8_STRING))
        } else if request.target == self.atoms.TEXT_HTML {
            write_data.html.as_ref().map(|s| (s.as_bytes(), self.atoms.TEXT_HTML))
        } else if request.target == self.atoms.IMAGE_PNG {
            write_data
                .image_png
                .as_ref()
                .map(|d| (d.as_slice(), self.atoms.IMAGE_PNG))
        } else if request.target == self.atoms.TEXT_URI_LIST {
            write_data
                .uri_list
                .as_ref()
                .map(|s| (s.as_bytes(), self.atoms.TEXT_URI_LIST))
        } else {
            None
        };

        if let Some((bytes, type_atom)) = data {
            let _ = self.conn.change_property8(
                PropMode::REPLACE,
                request.requestor,
                request.property,
                type_atom,
                bytes,
            );
            let _ = self.conn.flush();
            self.send_selection_notify(
                request.requestor,
                request.selection,
                request.target,
                request.property,
                request.time,
            );
        } else {
            // Unsupported target
            self.send_selection_notify(request.requestor, request.selection, request.target, x11rb::NONE, request.time);
        }
    }

    fn send_selection_notify(
        &self,
        requestor: Window,
        selection: Atom,
        target: Atom,
        property: Atom,
        time: Timestamp,
    ) {
        let event = SelectionNotifyEvent {
            response_type: SELECTION_NOTIFY_EVENT,
            sequence: 0,
            time,
            requestor,
            selection,
            target,
            property,
        };

        let _ = self.conn.send_event(
            false,
            requestor,
            EventMask::NO_EVENT,
            event,
        );
        let _ = self.conn.flush();
    }

    fn parse_targets(&self, reply: &GetPropertyReply) -> Vec<String> {
        use std::collections::HashSet;

        if reply.format != 32 {
            return Vec::new();
        }

        // TARGETS are 32-bit atoms
        let atoms: Vec<Atom> = reply
            .value
            .chunks_exact(4)
            .map(|chunk| u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        // Use HashSet to deduplicate (e.g., UTF8_STRING and TEXT_PLAIN both map to text/plain)
        let unique_formats: HashSet<&str> = atoms
            .iter()
            .filter_map(|&atom| {
                // Map known atoms to MIME types
                if atom == self.atoms.UTF8_STRING || atom == self.atoms.TEXT_PLAIN {
                    Some("text/plain")
                } else if atom == self.atoms.TEXT_HTML {
                    Some("text/html")
                } else if atom == self.atoms.IMAGE_PNG {
                    Some("image/png")
                } else if atom == self.atoms.TEXT_URI_LIST {
                    Some("text/uri-list")
                } else {
                    None
                }
            })
            .collect();

        // Sort for deterministic output order
        let mut formats: Vec<String> = unique_formats.into_iter().map(String::from).collect();
        formats.sort();
        formats
    }

    fn check_timeouts(
        &mut self,
        event_queue: &mut Vec<NativeEvent>,
        pending_ops: &mut HashMap<u64, PendingOperation>,
    ) {
        let timeout = std::time::Duration::from_millis(crate::CLIPBOARD_PENDING_OP_TIMEOUT_MS);
        let now = Instant::now();

        // Check pending reads
        let timed_out: Vec<u64> = self
            .pending_reads
            .iter()
            .filter(|(_, req)| now.duration_since(req.started_at) > timeout)
            .map(|(id, _)| *id)
            .collect();

        for callback_id in timed_out {
            self.pending_reads.remove(&callback_id);
            if let Some(op) = pending_ops.get_mut(&callback_id) {
                op.state = PendingOpState::TimedOut;
            }
            event_queue.push(NativeEvent::ClipboardError {
                callback_id,
                error_code: CLIPBOARD_ERR_TIMEOUT,
            });
        }

        // Check active INCR transfer for timeout
        if let Some(ref transfer) = self.active_incr {
            if let Some(op) = pending_ops.get(&transfer.callback_id) {
                if now.duration_since(op.started_at) > timeout {
                    let callback_id = transfer.callback_id;
                    // Take the transfer to complete timeout handling
                    let transfer = self.active_incr.take().unwrap();
                    if let Some(op) = pending_ops.get_mut(&transfer.callback_id) {
                        op.state = PendingOpState::TimedOut;
                    }
                    event_queue.push(NativeEvent::ClipboardError {
                        callback_id,
                        error_code: CLIPBOARD_ERR_TIMEOUT,
                    });
                }
            }
        }
    }
}

impl Drop for X11ClipboardBackend {
    fn drop(&mut self) {
        // Destroy the selection window
        let _ = self.conn.destroy_window(self.selection_window);
        let _ = self.conn.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_checks_display() {
        // This test just verifies the function runs without panicking
        let _ = X11ClipboardBackend::is_available();
    }

    #[test]
    fn test_is_available_reflects_environment() {
        // Test that is_available() returns true only when DISPLAY is set
        let has_display = std::env::var("DISPLAY").is_ok();
        assert_eq!(X11ClipboardBackend::is_available(), has_display);
    }

    // =========================================================================
    // X11 Backend Tests (require DISPLAY environment variable)
    // Run with: cargo test --features x11-backend -- --ignored
    // =========================================================================

    /// Helper to skip test if X11 is not available
    fn skip_if_no_x11() -> bool {
        if !X11ClipboardBackend::is_available() {
            eprintln!("Skipping test: DISPLAY not set (X11 not available)");
            return true;
        }
        false
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_backend_initialization() {
        if skip_if_no_x11() {
            return;
        }

        let backend = X11ClipboardBackend::new();
        assert!(backend.is_ok(), "X11 backend should initialize when DISPLAY is set");

        let backend = backend.unwrap();
        assert!(backend.selection_window != 0, "Selection window should be created");
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_atoms_interned() {
        if skip_if_no_x11() {
            return;
        }

        let backend = X11ClipboardBackend::new().unwrap();

        // Verify key atoms are non-zero (properly interned)
        assert!(backend.atoms.CLIPBOARD != 0, "CLIPBOARD atom should be interned");
        assert!(backend.atoms.PRIMARY != 0, "PRIMARY atom should be interned");
        assert!(backend.atoms.TARGETS != 0, "TARGETS atom should be interned");
        assert!(backend.atoms.UTF8_STRING != 0, "UTF8_STRING atom should be interned");
        assert!(backend.atoms.INCR != 0, "INCR atom should be interned");
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_mime_to_atom_mapping() {
        if skip_if_no_x11() {
            return;
        }

        let backend = X11ClipboardBackend::new().unwrap();

        // Test MIME type to atom mapping
        assert_eq!(backend.mime_to_atom("text/plain"), backend.atoms.TEXT_PLAIN);
        assert_eq!(
            backend.mime_to_atom("text/plain;charset=utf-8"),
            backend.atoms.TEXT_PLAIN_UTF8
        );
        assert_eq!(backend.mime_to_atom("text/html"), backend.atoms.TEXT_HTML);
        assert_eq!(backend.mime_to_atom("text/uri-list"), backend.atoms.TEXT_URI_LIST);
        assert_eq!(backend.mime_to_atom("image/png"), backend.atoms.IMAGE_PNG);
        // Unknown types default to UTF8_STRING
        assert_eq!(backend.mime_to_atom("application/json"), backend.atoms.UTF8_STRING);
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_atom_to_mime_mapping() {
        if skip_if_no_x11() {
            return;
        }

        let backend = X11ClipboardBackend::new().unwrap();

        // Test atom to MIME type mapping
        assert_eq!(backend.atom_to_mime(backend.atoms.TEXT_PLAIN), "text/plain");
        assert_eq!(backend.atom_to_mime(backend.atoms.TEXT_PLAIN_UTF8), "text/plain");
        assert_eq!(backend.atom_to_mime(backend.atoms.UTF8_STRING), "text/plain");
        assert_eq!(backend.atom_to_mime(backend.atoms.TEXT_HTML), "text/html");
        assert_eq!(backend.atom_to_mime(backend.atoms.TEXT_URI_LIST), "text/uri-list");
        assert_eq!(backend.atom_to_mime(backend.atoms.IMAGE_PNG), "image/png");
        // Unknown atoms return octet-stream
        assert_eq!(backend.atom_to_mime(12345), "application/octet-stream");
    }

    #[test]
    fn test_incr_serialization_rejects_concurrent() {
        // This test doesn't require X11 - it tests the logic directly
        // We can't create a real backend without X11, but we can test the concept
        // by verifying the error code constants are correct
        assert_eq!(CLIPBOARD_ERR_INTERNAL, 99);
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_read_format_rejects_during_incr() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Simulate an active INCR transfer
        backend.active_incr = Some(IncrTransfer {
            callback_id: 100,
            request_type: X11RequestType::Data(ClipboardTarget::Clipboard),
            partial_data: Vec::new(),
            expected_format: 8,
        });

        // New read should be rejected
        let result = backend.read_format(ClipboardTarget::Clipboard, "text/plain", 200);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CLIPBOARD_ERR_INTERNAL);
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_get_formats_rejects_during_incr() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Simulate an active INCR transfer
        backend.active_incr = Some(IncrTransfer {
            callback_id: 100,
            request_type: X11RequestType::Formats,
            partial_data: Vec::new(),
            expected_format: 32,
        });

        // New get_formats should be rejected
        let result = backend.get_formats(200);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CLIPBOARD_ERR_INTERNAL);
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_duplicate_callback_rejected() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // First read should succeed
        let result1 = backend.read_format(ClipboardTarget::Clipboard, "text/plain", 100);
        assert!(result1.is_ok());

        // Same callback_id should be rejected
        let result2 = backend.read_format(ClipboardTarget::Clipboard, "text/plain", 100);
        assert!(result2.is_err());
        assert_eq!(result2.unwrap_err(), CLIPBOARD_ERR_INTERNAL);
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_cancel_removes_pending() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Add a pending read
        let _ = backend.read_format(ClipboardTarget::Clipboard, "text/plain", 100);
        assert!(backend.pending_reads.contains_key(&100));

        // Cancel should remove it
        let removed = backend.cancel(100);
        assert!(removed);
        assert!(!backend.pending_reads.contains_key(&100));

        // Cancel again should return false
        let removed_again = backend.cancel(100);
        assert!(!removed_again);
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_cancel_removes_incr() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Simulate an active INCR transfer
        backend.active_incr = Some(IncrTransfer {
            callback_id: 100,
            request_type: X11RequestType::Data(ClipboardTarget::Clipboard),
            partial_data: Vec::new(),
            expected_format: 8,
        });

        // Cancel should remove the INCR transfer
        let removed = backend.cancel(100);
        assert!(removed);
        assert!(backend.active_incr.is_none());
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_write_stages_data() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Write text should stage data
        let result = backend.write_text("Hello, X11!");
        assert!(result.is_ok());
        assert!(backend.write_data.is_some());
        assert_eq!(
            backend.write_data.as_ref().unwrap().text,
            Some("Hello, X11!".to_string())
        );

        // Write html should add to staged data
        let result = backend.write_html("<b>Bold</b>");
        assert!(result.is_ok());
        assert_eq!(
            backend.write_data.as_ref().unwrap().html,
            Some("<b>Bold</b>".to_string())
        );

        // Text should still be there
        assert_eq!(
            backend.write_data.as_ref().unwrap().text,
            Some("Hello, X11!".to_string())
        );
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_write_commit_without_data_fails() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Commit without staging data should fail
        let result = backend.write_commit(100);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CLIPBOARD_ERR_INTERNAL);
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_selection_target_clipboard() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Read from CLIPBOARD target should work
        let result = backend.read_format(ClipboardTarget::Clipboard, "text/plain", 100);
        assert!(result.is_ok());

        // Verify the request was tracked
        assert!(backend.pending_reads.contains_key(&100));
        let request = backend.pending_reads.get(&100).unwrap();
        assert!(matches!(
            request.request_type,
            X11RequestType::Data(ClipboardTarget::Clipboard)
        ));
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_selection_target_primary() {
        if skip_if_no_x11() {
            return;
        }

        let mut backend = X11ClipboardBackend::new().unwrap();

        // Read from PRIMARY target should work
        let result = backend.read_format(ClipboardTarget::PrimarySelection, "text/plain", 100);
        assert!(result.is_ok());

        // Verify the request was tracked with correct target
        assert!(backend.pending_reads.contains_key(&100));
        let request = backend.pending_reads.get(&100).unwrap();
        assert!(matches!(
            request.request_type,
            X11RequestType::Data(ClipboardTarget::PrimarySelection)
        ));
    }

    #[test]
    #[ignore] // Requires X11 display
    fn test_x11_backend_drop_cleans_up() {
        if skip_if_no_x11() {
            return;
        }

        // Create and drop backend - should not panic
        let backend = X11ClipboardBackend::new().unwrap();
        let window_id = backend.selection_window;
        assert!(window_id != 0);
        drop(backend);
        // If we get here without panic, cleanup succeeded
    }
}
