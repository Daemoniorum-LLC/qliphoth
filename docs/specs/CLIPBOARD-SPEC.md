# Qliphoth Clipboard API Specification

**Version:** 0.5.1
**Date:** 2025-02-18
**Status:** Draft (Methodology Compliant)
**SDD Phase:** Spec
**Parent Spec:** NATIVE-RENDERING-SPEC.md
**Compliance:** SPEC-FORMATTING v1.0.0, SDD v1.1.0

---

## Executive Summary

This specification defines a modern, async, MIME-aware clipboard API for Qliphoth's native backend. The API supports multiple content types, works consistently across platforms, and integrates with the existing event loop.

### Design Goals

1. **MIME-aware** - Support multiple content formats, not just plain text
2. **Async-first** - Unified async API that works on all platforms (especially Wayland)
3. **Extensible** - Easy to add new MIME types without API changes
4. **Secure** - No implicit clipboard monitoring; explicit opt-in only

### Non-Goals

The following are explicitly out of scope for this specification:

- **Drag-and-drop** - Related but separate interaction model (future spec)
- **Inter-process streaming** - Large data should be copied, not streamed
- **Clipboard manager APIs** - No history, sync, or manager integration beyond sensitive flag
- **System clipboard monitoring** - No passive observation of other apps' clipboard activity

### Current State

| Component | Status | Notes |
|-----------|--------|-------|
| `native_clipboard_read` | ⚠️ Stub | Returns 0, no implementation |
| `native_clipboard_write` | ⚠️ Stub | No-op, no implementation |
| MIME support | ❌ None | Current API assumes plain text |
| Async support | ❌ None | Current API is synchronous |

---

## 1. Conceptual Foundation

### 1.1 The Modern Clipboard

Modern clipboards are **format-negotiated data transfers**:

```
┌─────────────────┐         ┌─────────────────┐
│  Source App     │         │  Target App     │
│                 │         │                 │
│ "I have data    │         │ "What formats   │
│  available as:  │ ──────► │  do you have?"  │
│  - text/html    │         │                 │
│  - text/plain   │         │ "Give me        │
│  - image/png"   │ ◄────── │  text/html"     │
│                 │         │                 │
│ [sends html]    │ ──────► │ [receives]      │
└─────────────────┘         └─────────────────┘
```

Key concepts:

- **Offer** - Source declares available formats when copying
- **Request** - Target asks for specific format when pasting
- **Lazy evaluation** - Data may not be serialized until requested
- **Format priority** - Target typically requests richest format it supports

### 1.2 Platform Clipboard Models

| Platform | Model | Primary Selection | Notes |
|----------|-------|-------------------|-------|
| **X11** | Owner-based | ✅ Yes | Selection owner provides data on demand; can block if owner unresponsive |
| **Wayland** | Asynchronous | ✅ Yes (`zwp_primary_selection_v1`) | Data offers via file descriptors; supported by wlroots, KWin, Mutter |
| **Windows** | Global store | ❌ No | Data copied to system clipboard; delayed rendering optional |
| **macOS** | Pasteboard | ❌ No | NSPasteboard with multiple representations |

**Note:** Even "synchronous" platforms can block indefinitely (e.g., X11 owner crashes). The async API protects against UI freezes in all cases.

### 1.3 Why Async-First

```
Wayland clipboard read:
1. App requests paste
2. Compositor returns available MIME types (async)
3. App picks format, requests data
4. Compositor connects app to data source via pipe (async)
5. App reads data from pipe (async)
```

Benefits of unified async API:
- Non-blocking UI during large clipboard transfers (images)
- Protection against unresponsive clipboard owners (X11)
- Consistent code paths across platforms
- Natural fit with Qliphoth's event-driven architecture

---

## 2. Type Architecture

This section defines core types using pseudocode notation. Implementers may use different internal representations as long as observable behavior matches.

### 2.1 Clipboard Target ❓

```
ClipboardTarget:
    Clipboard = 0       // Standard clipboard (Ctrl+C/V)
    PrimarySelection = 1 // X11/Wayland highlight-to-copy

    invariant: value ∈ {0, 1}
```

**Note:** Primary selection falls back to Clipboard on unsupported platforms.

### 2.2 Result Codes

```
ClipboardResult:
    OK = 0                  // Success
    ERR_UNAVAILABLE = 1     // Clipboard not available
    ERR_FORMAT_NOT_FOUND = 2 // Requested format not in clipboard
    ERR_ACCESS_DENIED = 3   // Permission denied
    ERR_TIMEOUT = 4         // Operation timed out
    ERR_EMPTY = 5           // Clipboard is empty
    ERR_CANCELLED = 6       // Operation was cancelled
    ERR_INVALID_HANDLE = 7  // Invalid write/request handle
    ERR_INTERNAL = 99       // Internal error
```

### 2.3 Capability Flags

```
ClipboardCapabilities (bitfield):
    CAP_READ = 1 << 0           // ✅ Can read
    CAP_WRITE = 1 << 1          // ✅ Can write
    CAP_PRIMARY = 1 << 2        // ⚠️ Primary selection (X11/Wayland only)
    CAP_IMAGES = 1 << 3         // 🔮 Image formats
    CAP_HTML = 1 << 4           // 🔮 HTML format
    CAP_FILES = 1 << 5          // 🔮 File URI list
    CAP_SENSITIVE = 1 << 6      // ⚠️ Sensitive data flag (platform-dependent)
    CAP_CHANGE_NOTIFY = 1 << 7  // 🔮 Change notifications
```

### 2.4 Operation Handles

```
WriteHandle:
    raw: u64
    invariant: raw ≠ 0 for valid handles

    // Returned by write_begin()
    // Invalidated by commit(), cancel(), or timeout (60s)

CallbackId:
    raw: u64

    // Caller-provided for correlating async responses
    // Same value appears in NativeEventData.callback_id
```

### 2.5 Event Types

```
ClipboardEventType:
    FORMATS_AVAILABLE = 200  // Format list ready
    DATA_READY = 201         // Data retrieved successfully
    WRITE_COMPLETE = 202     // Write committed
    ERROR = 203              // Operation failed

    invariant: value ∈ [200, 299]  // Reserved range
```

### 2.6 Internal State ❓

These types guide implementation but are not part of the public API:

```
PendingOperation:
    op_type: OpType          // GetFormats | ReadFormat
    target: ClipboardTarget
    mime_type: Option<String>
    callback_id: CallbackId
    started_at: Instant

CompletedData:
    data: bytes              // Retrieved clipboard data
    formats: Option<[String]> // For GetFormats responses
    completed_at: Instant

WriteBuilder:
    target: ClipboardTarget
    formats: [(mime: String, data: bytes, sensitive: bool)]
    created_at: Instant
```

---

## 3. MIME Type Support

### 3.1 MIME Type Normalization

All MIME types are normalized before use:

1. **Case-insensitive** - `TEXT/PLAIN` becomes `text/plain`
2. **Whitespace stripped** - `text/plain; charset=utf-8` becomes `text/plain;charset=utf-8`
3. **Parameters preserved** - `text/plain;charset=utf-8` kept distinct from `text/plain`

The implementation performs this normalization; callers need not pre-normalize.

### 3.2 Encoding Handling

**All text data exposed to Sigil is UTF-8.**

The implementation handles platform-specific encodings transparently:

| Platform | Native Encoding | Conversion |
|----------|-----------------|------------|
| Windows | UTF-16LE (`CF_UNICODETEXT`) | Automatic UTF-8 ↔ UTF-16 |
| macOS | UTF-8 (usually) | Pass-through |
| X11 | `UTF8_STRING` preferred | Falls back to `STRING` with Latin-1 conversion |
| Wayland | UTF-8 | Pass-through |

### 3.3 Core MIME Types (Phase 1) ❌

| MIME Type | Description | Priority |
|-----------|-------------|----------|
| `text/plain` | Plain UTF-8 text | P0 - Essential |
| `text/plain;charset=utf-8` | Explicit UTF-8 (alias) | P0 - Alias |

### 3.4 Extended MIME Types (Phase 2) 🔮

| MIME Type | Description | Priority |
|-----------|-------------|----------|
| `text/html` | HTML formatted text | P1 - Common |
| `text/uri-list` | File paths / URLs (newline-separated) | P1 - File operations |
| `image/png` | PNG image data | P2 - Screenshots |
| `image/jpeg` | JPEG image data | P2 - Photos |
| `image/svg+xml` | SVG vector graphics | P2 - Design tools |

### 3.5 Application-Specific Types (Phase 3) 🔮

| MIME Type | Description | Priority |
|-----------|-------------|----------|
| `application/json` | JSON data | P2 - Data exchange |
| `application/x-qliphoth-vnode` | Serialized VNode tree | P3 - Internal |
| Custom types | App-defined formats | P3 - Extensible |

### 3.6 Format Negotiation

When reading, apps should request formats in preference order:

```
// Example: Rich text editor paste
preferred ← ["text/html", "text/plain"]

∀ fmt ∈ preferred:
    if available_formats contains fmt:
        native_clipboard_read_format(target, fmt, callback_id)
        return
```

When writing, apps should offer all applicable formats:

```
// Always include plain text fallback
native_clipboard_write_add_format(handle, "text/html", html_data, html_len)
native_clipboard_write_add_format(handle, "text/plain", plain_data, plain_len)
```

---

## 4. Event System Integration

### 4.1 Extending NativeEventData

Clipboard events use the existing `NativeEventData` structure with clipboard-specific interpretation of fields:

```sigil
// Existing NativeEventData fields repurposed for clipboard events:
☉ Σ NativeEventData {
    event_type: i32!,      // EVENT_CLIPBOARD_* constant
    callback_id: u64!,     // The callback_id passed to the initiating function

    // For clipboard events, these fields have special meaning:
    // x: f32              → (unused, 0.0)
    // y: f32              → (unused, 0.0)
    // button: i32         → error_code (for EVENT_CLIPBOARD_ERROR)
    // key: i32            → format_count (for EVENT_CLIPBOARD_FORMATS_AVAILABLE)
    // width: u32          → data_size low 32 bits (for EVENT_CLIPBOARD_DATA_READY)
    // height: u32         → data_size high 32 bits (for large data)
    // ...
}
```

**Important:** The `callback_id` in the event matches the `callback_id` passed to the initiating function. This allows callers to correlate responses with requests.

### 4.2 Retrieving Clipboard-Specific Data

After receiving a clipboard event, use dedicated retrieval functions:

```sigil
// After EVENT_CLIPBOARD_FORMATS_AVAILABLE:
native_clipboard_get_formats_data(callback_id, out_formats, max_formats) -> usize

// After EVENT_CLIPBOARD_DATA_READY:
native_clipboard_get_data_size(callback_id) -> usize
native_clipboard_get_data(callback_id, out_buf, max_len) -> usize
```

### 4.3 Event Constants

```sigil
// Clipboard event types (range 200-299 reserved for clipboard)
const EVENT_CLIPBOARD_FORMATS_AVAILABLE: i32 = 200;
const EVENT_CLIPBOARD_DATA_READY: i32 = 201;
const EVENT_CLIPBOARD_WRITE_COMPLETE: i32 = 202;
const EVENT_CLIPBOARD_ERROR: i32 = 203;
const EVENT_CLIPBOARD_CHANGED: i32 = 204;  // Fired when clipboard content changes (Phase 5)
```

**Reserved ranges:**
- 0-99: Core events (click, key, mouse, focus, etc.)
- 100-199: Timer/animation events
- 200-299: Clipboard events
- 300-399: (Reserved for future)

---

## 5. FFI API Design ❌

### 5.1 Clipboard Targets

```sigil
/// Clipboard target selection
☉ ᛈ ClipboardTarget {
    /// Standard clipboard (Ctrl+C / Ctrl+V)
    Clipboard = 0,
    /// Primary selection (X11/Wayland: highlight to copy, middle-click to paste)
    /// Falls back to Clipboard on platforms without primary selection
    PrimarySelection = 1,
}
```

### 5.2 Error Codes

```sigil
/// Clipboard error codes (returned in NativeEventData.button for ERROR events)
const CLIPBOARD_OK: i32 = 0;                  // Success (not an error)
const CLIPBOARD_ERR_UNAVAILABLE: i32 = 1;     // Clipboard not available
const CLIPBOARD_ERR_FORMAT_NOT_FOUND: i32 = 2; // Requested format not available
const CLIPBOARD_ERR_ACCESS_DENIED: i32 = 3;   // Permission denied
const CLIPBOARD_ERR_TIMEOUT: i32 = 4;         // Operation timed out
const CLIPBOARD_ERR_EMPTY: i32 = 5;           // Clipboard is empty
const CLIPBOARD_ERR_CANCELLED: i32 = 6;       // Operation was cancelled
const CLIPBOARD_ERR_INVALID_HANDLE: i32 = 7;  // Invalid request/write handle
const CLIPBOARD_ERR_INTERNAL: i32 = 99;       // Internal error
```

### 5.3 FFI Functions

```sigil
⊞ "C" {
    // =========================================================================
    // API Version
    // =========================================================================

    /// Get clipboard API version.
    /// Returns: (major << 16) | (minor << 8) | patch
    /// Current: 0x000200 (0.2.0)
    rite native_clipboard_api_version() -> u32;

    // =========================================================================
    // Capabilities Query
    // =========================================================================

    /// Query clipboard capabilities for the current platform.
    /// Returns: Bitfield of CLIPBOARD_CAP_* flags
    rite native_clipboard_capabilities() -> u32;

    // =========================================================================
    // Reading from Clipboard
    // =========================================================================

    /// Request available formats from clipboard.
    ///
    /// Triggers EVENT_CLIPBOARD_FORMATS_AVAILABLE or EVENT_CLIPBOARD_ERROR.
    /// If clipboard is empty, fires EVENT_CLIPBOARD_FORMATS_AVAILABLE with
    /// format_count=0 (check via NativeEventData.key field).
    ///
    /// # Arguments
    /// - `target`: ClipboardTarget (0 = Clipboard, 1 = PrimarySelection)
    /// - `callback_id`: ID for correlating the async response
    ///
    /// # Returns
    /// 1 on success (request queued), 0 on immediate failure
    rite native_clipboard_get_formats(
        target: i32,
        callback_id: u64,
    ) -> i32;

    /// Get the format list after EVENT_CLIPBOARD_FORMATS_AVAILABLE.
    ///
    /// # Arguments
    /// - `callback_id`: The callback_id from the event
    /// - `out_formats`: Array to fill with null-terminated MIME type strings
    /// - `max_formats`: Maximum number of format pointers to write
    ///
    /// # Returns
    /// Number of formats written. Pointers valid until next clipboard operation
    /// or native_clipboard_release(callback_id).
    rite native_clipboard_get_formats_data(
        callback_id: u64,
        out_formats: *mut *const u8,
        max_formats: usize,
    ) -> usize;

    /// Request clipboard data in specific format.
    ///
    /// Triggers EVENT_CLIPBOARD_DATA_READY or EVENT_CLIPBOARD_ERROR.
    ///
    /// # Arguments
    /// - `target`: ClipboardTarget
    /// - `mime_type`: Null-terminated MIME type string (ASCII)
    /// - `callback_id`: ID for correlating the async response
    ///
    /// # Returns
    /// 1 on success (request queued), 0 on immediate failure
    rite native_clipboard_read_format(
        target: i32,
        mime_type: *const u8,
        callback_id: u64,
    ) -> i32;

    /// Get the total size of clipboard data after EVENT_CLIPBOARD_DATA_READY.
    ///
    /// # Arguments
    /// - `callback_id`: The callback_id from the event
    ///
    /// # Returns
    /// Total data size in bytes, or 0 if callback_id invalid/not ready
    rite native_clipboard_get_data_size(
        callback_id: u64,
    ) -> usize;

    /// Get the data from a completed clipboard read.
    ///
    /// May be called multiple times; data is not consumed.
    /// Data remains available until native_clipboard_release() or timeout.
    ///
    /// # Arguments
    /// - `callback_id`: The callback_id from the event
    /// - `out_buf`: Buffer to write data into
    /// - `max_len`: Maximum bytes to write
    ///
    /// # Returns
    /// Number of bytes written, or 0 if callback_id invalid/not ready
    rite native_clipboard_get_data(
        callback_id: u64,
        out_buf: *mut u8,
        max_len: usize,
    ) -> usize;

    /// Cancel a pending read operation.
    ///
    /// If the operation is still pending, fires EVENT_CLIPBOARD_ERROR with
    /// CLIPBOARD_ERR_CANCELLED. If already complete, releases the data.
    ///
    /// # Arguments
    /// - `callback_id`: The callback_id of the pending operation
    rite native_clipboard_cancel(
        callback_id: u64,
    );

    /// Release resources associated with a completed clipboard operation.
    ///
    /// Should be called after retrieving data to free memory.
    /// Automatically called after DATA_LIFETIME_SECONDS (30s) if not released.
    ///
    /// # Arguments
    /// - `callback_id`: The callback_id of the completed operation
    rite native_clipboard_release(
        callback_id: u64,
    );

    // =========================================================================
    // Writing to Clipboard
    // =========================================================================

    /// Begin a clipboard write operation.
    ///
    /// Returns a write handle for adding formats. The write is committed
    /// when native_clipboard_write_commit is called.
    ///
    /// Write handles expire after WRITE_HANDLE_TIMEOUT_SECONDS (60s) if not
    /// committed or cancelled.
    ///
    /// # Arguments
    /// - `target`: ClipboardTarget
    ///
    /// # Returns
    /// Write handle (non-zero on success, 0 on failure)
    rite native_clipboard_write_begin(
        target: i32,
    ) -> u64;

    /// Add a format to the pending clipboard write.
    ///
    /// Can be called multiple times to offer multiple formats.
    /// Data is copied; caller may free after this returns.
    ///
    /// # Arguments
    /// - `write_handle`: Handle from native_clipboard_write_begin
    /// - `mime_type`: Null-terminated MIME type string (ASCII)
    /// - `data`: Pointer to data bytes
    /// - `data_len`: Length of data in bytes
    ///
    /// # Returns
    /// 1 on success, 0 on failure (invalid handle, out of memory)
    rite native_clipboard_write_add_format(
        write_handle: u64,
        mime_type: *const u8,
        data: *const u8,
        data_len: usize,
    ) -> i32;

    /// Add a sensitive format (excluded from clipboard managers/history).
    ///
    /// Same as write_add_format but marks data as sensitive.
    /// Not all platforms support this; check CLIPBOARD_CAP_SENSITIVE.
    ///
    /// # Arguments
    /// (same as write_add_format)
    rite native_clipboard_write_add_sensitive(
        write_handle: u64,
        mime_type: *const u8,
        data: *const u8,
        data_len: usize,
    ) -> i32;

    /// Commit the clipboard write.
    ///
    /// Triggers EVENT_CLIPBOARD_WRITE_COMPLETE or EVENT_CLIPBOARD_ERROR.
    /// The write handle becomes invalid after this call.
    ///
    /// # Arguments
    /// - `write_handle`: Handle from native_clipboard_write_begin
    /// - `callback_id`: ID for correlating the async response
    ///
    /// # Returns
    /// 1 on success (commit queued), 0 on failure (invalid handle)
    rite native_clipboard_write_commit(
        write_handle: u64,
        callback_id: u64,
    ) -> i32;

    /// Cancel a pending clipboard write.
    ///
    /// The write handle becomes invalid after this call.
    ///
    /// # Arguments
    /// - `write_handle`: Handle from native_clipboard_write_begin
    rite native_clipboard_write_cancel(
        write_handle: u64,
    );

    // =========================================================================
    // Chunked Read (Phase 5)
    // =========================================================================

    /// Read a chunk of clipboard data at a specific offset.
    /// Enables efficient streaming of large clipboard data without copying everything.
    ///
    /// # Arguments
    /// - `callback_id`: The callback_id from the completed read event
    /// - `offset`: Byte offset to start reading from
    /// - `out_buf`: Buffer to write data into
    /// - `max_len`: Maximum bytes to write
    ///
    /// # Returns
    /// Number of bytes written, or 0 if invalid callback_id, offset out of bounds, or null buffer
    rite native_clipboard_read_chunk(
        callback_id: u64,
        offset: usize,
        out_buf: *mut u8,
        max_len: usize,
    ) -> usize;

    // =========================================================================
    // Change Notifications (Phase 5)
    // =========================================================================

    /// Subscribe to clipboard change notifications.
    /// When clipboard content changes, EVENT_CLIPBOARD_CHANGED fires with the callback_id.
    ///
    /// Implementation uses polling (500ms interval) with content hashing.
    ///
    /// # Arguments
    /// - `target`: ClipboardTarget (0 = Clipboard, 1 = PrimarySelection)
    /// - `callback_id`: ID for correlating change events
    ///
    /// # Returns
    /// 1 on success, 0 if already subscribed with this callback_id
    rite native_clipboard_subscribe_changes(
        target: i32,
        callback_id: u64,
    ) -> i32;

    /// Unsubscribe from clipboard change notifications.
    ///
    /// # Arguments
    /// - `callback_id`: The callback_id used when subscribing
    rite native_clipboard_unsubscribe_changes(
        callback_id: u64,
    );

    // =========================================================================
    // Deprecated API (backward compatibility)
    // =========================================================================

    /// DEPRECATED: Use native_clipboard_read_format instead.
    /// Synchronous read, blocks thread, text/plain only.
    rite native_clipboard_read(out_buf: *mut u8, max_len: usize) -> usize;

    /// DEPRECATED: Use native_clipboard_write_* instead.
    /// Synchronous write, blocks thread, text/plain only.
    rite native_clipboard_write(content: *const u8);
}
```

### 5.4 Capability Flags

```sigil
/// Clipboard capability flags
const CLIPBOARD_CAP_READ: u32 = 1 << 0;           // Can read from clipboard
const CLIPBOARD_CAP_WRITE: u32 = 1 << 1;          // Can write to clipboard
const CLIPBOARD_CAP_PRIMARY: u32 = 1 << 2;        // Primary selection supported
const CLIPBOARD_CAP_IMAGES: u32 = 1 << 3;         // Image formats supported
const CLIPBOARD_CAP_HTML: u32 = 1 << 4;           // HTML format supported
const CLIPBOARD_CAP_FILES: u32 = 1 << 5;          // File URI list supported
const CLIPBOARD_CAP_SENSITIVE: u32 = 1 << 6;      // Sensitive data flag supported
const CLIPBOARD_CAP_CHANGE_NOTIFY: u32 = 1 << 7;  // Change notifications (polling-based)
const CLIPBOARD_CAP_SVG: u32 = 1 << 8;            // SVG format supported
const CLIPBOARD_CAP_CUSTOM_FORMATS: u32 = 1 << 9; // Custom application/* formats
const CLIPBOARD_CAP_CHUNKED_READ: u32 = 1 << 10;  // Chunked read API supported
```

### 5.5 Constants

```sigil
/// Data lifetime: completed read data auto-released after this duration
const CLIPBOARD_DATA_LIFETIME_SECONDS: u32 = 30;

/// Write handle timeout: uncommitted writes cancelled after this duration
const CLIPBOARD_WRITE_HANDLE_TIMEOUT_SECONDS: u32 = 60;
```

---

## 6. Data Lifecycle

### 6.1 Read Operation Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────┐
│ 1. App calls native_clipboard_read_format(target, mime, callback_id)    │
│    → Returns 1 (success) or 0 (immediate failure)                       │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 2. Implementation fetches data asynchronously                           │
│    → On success: fires EVENT_CLIPBOARD_DATA_READY                       │
│    → On failure: fires EVENT_CLIPBOARD_ERROR                            │
│    → Event contains callback_id for correlation                         │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 3. App receives event, retrieves data                                   │
│    → native_clipboard_get_data_size(callback_id)                        │
│    → native_clipboard_get_data(callback_id, buf, len)                   │
│    → May call get_data multiple times (data not consumed)               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 4. App releases resources                                               │
│    → native_clipboard_release(callback_id)                              │
│    → Or: auto-released after 30 seconds                                 │
└─────────────────────────────────────────────────────────────────────────┘
```

### 6.2 Write Operation Lifecycle

```
┌─────────────────────────────────────────────────────────────────────────┐
│ 1. App calls native_clipboard_write_begin(target)                       │
│    → Returns write_handle (non-zero) or 0 (failure)                     │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 2. App adds one or more formats                                         │
│    → native_clipboard_write_add_format(handle, mime, data, len)         │
│    → Data is COPIED; caller may free original after return              │
│    → May add multiple formats for rich clipboard support                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 3. App commits or cancels                                               │
│    → native_clipboard_write_commit(handle, callback_id)                 │
│    → Or: native_clipboard_write_cancel(handle)                          │
│    → Or: auto-cancelled after 60 seconds (handle expires)               │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ 4. For commit: async completion                                         │
│    → EVENT_CLIPBOARD_WRITE_COMPLETE on success                          │
│    → EVENT_CLIPBOARD_ERROR on failure                                   │
└─────────────────────────────────────────────────────────────────────────┘
```

### 6.3 Empty Clipboard Handling

When the clipboard is empty:

- `native_clipboard_get_formats()` succeeds and fires `EVENT_CLIPBOARD_FORMATS_AVAILABLE`
  with `format_count = 0` (in `NativeEventData.key` field)
- `native_clipboard_read_format()` fires `EVENT_CLIPBOARD_ERROR` with
  `CLIPBOARD_ERR_EMPTY` (since requested format cannot exist)

---

## 7. Usage Examples

### 7.1 Simple Text Copy

```
copy_text(text: bytes):
    handle ← native_clipboard_write_begin(0)  // Clipboard target
    if handle = 0:
        log_error("Failed to begin clipboard write")
        return

    native_clipboard_write_add_format(handle, "text/plain", text, len(text))

    callback_id ← generate_callback_id()
    native_clipboard_write_commit(handle, callback_id)
    // EVENT_CLIPBOARD_WRITE_COMPLETE fires when done
```

### 7.2 Simple Text Paste

```
// Request paste
request_paste():
    callback_id ← generate_callback_id()
    native_clipboard_read_format(0, "text/plain", callback_id)

// Handle the callback (in event loop)
on_event(event: NativeEventData):
    match event.event_type:
        EVENT_CLIPBOARD_DATA_READY:
            size ← native_clipboard_get_data_size(event.callback_id)
            buf ← allocate(size)
            native_clipboard_get_data(event.callback_id, buf, size)

            // Use the text...
            process_pasted_text(buf)

            // Release resources
            native_clipboard_release(event.callback_id)

        EVENT_CLIPBOARD_ERROR:
            error_code ← event.button  // Error code in button field
            log_error("Clipboard error: " + error_code)
```

### 7.3 Rich Content Copy (HTML + Plain Text)

```
// Copy with multiple formats for maximum compatibility
copy_rich_text(html: bytes, plain: bytes):
    handle ← native_clipboard_write_begin(0)

    // Offer HTML (rich) - recipients that support it will prefer this
    native_clipboard_write_add_format(handle, "text/html", html, len(html))

    // Always offer plain text fallback
    native_clipboard_write_add_format(handle, "text/plain", plain, len(plain))

    native_clipboard_write_commit(handle, generate_callback_id())
```

### 7.4 Format-Aware Paste

```
// State for two-phase paste
pending_paste_callback: Option<u64> ← None

// First, query available formats
smart_paste_begin():
    pending_paste_callback ← Some(generate_callback_id())
    native_clipboard_get_formats(0, pending_paste_callback)

// Handle formats response
on_formats_available(event: NativeEventData):
    format_count ← event.key

    if format_count = 0:
        show_message("Clipboard is empty")
        return

    // Get format list
    formats ← allocate_ptr_array(format_count)
    native_clipboard_get_formats_data(event.callback_id, formats, format_count)

    // Pick best format
    preferred ← ["text/html", "text/plain"]
    ∀ pref ∈ preferred:
        ∀ i ∈ 0..format_count:
            if strcmp(formats[i], pref) = 0:
                // Request this format
                native_clipboard_read_format(0, pref, generate_callback_id())
                native_clipboard_release(event.callback_id)
                return

    native_clipboard_release(event.callback_id)
    show_message("No compatible format")
```

### 7.5 Password Manager (Sensitive Data)

```
// Copy password with sensitive flag
copy_password(password: bytes):
    handle ← native_clipboard_write_begin(0)

    // Mark as sensitive - clipboard managers should not persist this
    native_clipboard_write_add_sensitive(
        handle,
        "text/plain",
        password,
        len(password)
    )

    native_clipboard_write_commit(handle, generate_callback_id())

    // Optionally: set a timer to clear clipboard after N seconds
    set_timeout(clear_clipboard, 30000)
```

---

## 8. Implementation Notes

### 8.1 Recommended Rust Crates

| Crate | Purpose | Notes |
|-------|---------|-------|
| `arboard` | Cross-platform clipboard | Active, supports images, primary selection |
| `wl-clipboard-rs` | Wayland-native | Better async support on Wayland |
| `x11-clipboard` | X11-native | More control for X11 edge cases |

**Recommendation:** Start with `arboard` for simplicity. It handles most cases well and has active maintenance.

### 8.2 Internal State

```rust
struct ClipboardState {
    /// Pending async operations (read requests, format queries)
    pending_ops: HashMap<u64, PendingOperation>,

    /// Completed data awaiting retrieval
    completed: HashMap<u64, CompletedData>,

    /// Pending write builders
    write_handles: HashMap<u64, WriteBuilder>,

    /// Next handle ID
    next_id: u64,
}

struct PendingOperation {
    op_type: OpType,         // GetFormats, ReadFormat
    target: ClipboardTarget,
    mime_type: Option<String>,
    callback_id: u64,
    started_at: Instant,
}

struct CompletedData {
    data: Vec<u8>,
    formats: Option<Vec<String>>,  // For GetFormats responses
    completed_at: Instant,
}

struct WriteBuilder {
    target: ClipboardTarget,
    formats: Vec<(String, Vec<u8>, bool)>,  // (mime, data, is_sensitive)
    created_at: Instant,
}
```

### 8.3 Platform Format Mapping

| MIME Type | Windows | macOS | X11 | Wayland |
|-----------|---------|-------|-----|---------|
| `text/plain` | `CF_UNICODETEXT` | `public.utf8-plain-text` | `UTF8_STRING`, `TEXT` | `text/plain` |
| `text/html` | `CF_HTML` (with header) | `public.html` | `text/html` | `text/html` |
| `text/uri-list` | `CF_HDROP` (converted) | `public.file-url` | `text/uri-list` | `text/uri-list` |
| `image/png` | `CF_PNG` or `CF_DIB` | `public.png` | `image/png` | `image/png` |

### 8.4 Windows CF_HTML Format

Windows HTML clipboard requires a specific header format:

```
Version:0.9
StartHTML:XXXXX
EndHTML:XXXXX
StartFragment:XXXXX
EndFragment:XXXXX
<html><body>
<!--StartFragment-->
ACTUAL HTML CONTENT
<!--EndFragment-->
</body></html>
```

The implementation MUST:
- Add this header when writing `text/html`
- Strip this header when reading `text/html`
- Handle byte offsets correctly (they count from start of entire string)

### 8.5 Timeout Handling

The implementation must track:

1. **Pending operations** - Cancel and fire error event after reasonable timeout (e.g., 10s)
2. **Completed data** - Release after `DATA_LIFETIME_SECONDS` (30s)
3. **Write handles** - Cancel after `WRITE_HANDLE_TIMEOUT_SECONDS` (60s)

Use a background task or check timeouts during event polling.

---

## 9. Implementation Phases

### Phase 1: Basic Text (P0) ✅ COMPLETE

- [x] Add `arboard` dependency
- [x] Implement `native_clipboard_api_version`
- [x] Implement `native_clipboard_capabilities` (basic flags)
- [x] Implement write flow: `begin`, `add_format`, `commit`, `cancel` for `text/plain`
- [x] Implement read flow: `read_format`, `get_data_size`, `get_data` for `text/plain`
- [x] Implement `release` and `cancel`
- [x] Add clipboard event types to event loop
- [x] Implement timeouts for completed data and write handles
- [x] Basic tests

### Phase 2: Format Negotiation (P1) ✅ COMPLETE

- [x] Implement `native_clipboard_get_formats`
- [x] Implement `native_clipboard_get_formats_data`
- [x] Add `text/html` support (via arboard set().html() / get().html())
- [x] Add `text/uri-list` support (via arboard set().file_list() / get().file_list())
- [x] Update capability flags (CAP_HTML, CAP_FILES)

### Phase 3: Binary Content (P2) ✅

- [x] Add `image/png` support (via `image` crate encode/decode + arboard)
- [x] Add `image/jpeg` support (via `image` crate encode/decode + arboard)
- [x] Handle large data transfers efficiently (PNG compression handles this)
- [x] Update capability flags (CAP_IMAGES)

### Phase 4: Platform Polish (P2) ✅

- [x] Primary selection support (X11/Wayland)
- [x] Sensitive data support where available
- [x] Platform-specific format conversions
- [x] Update capability flags

### Phase 5: Advanced Features (P3) ✅

- [x] Clipboard change notifications (polling-based, 500ms interval)
- [x] Custom application formats (`application/x-qliphoth-*`, `application/json`, etc.)
- [x] Performance optimization for large transfers (chunked read API)
- [x] `image/svg+xml` support
- [x] New capabilities: CAP_SVG, CAP_CUSTOM_FORMATS, CAP_CHANGE_NOTIFY, CAP_CHUNKED_READ

### Phase 6: Async Implementation (P3)

For true Wayland/async clipboard support:

- [ ] Replace synchronous arboard calls with async file descriptor reads
- [ ] Implement proper pending operation tracking
- [ ] Fire `CLIPBOARD_ERR_CANCELLED` from `native_clipboard_cancel` for pending ops
- [ ] Add `CLIPBOARD_ERR_TIMEOUT` for long-running operations
- [ ] Consider `wl-clipboard` integration for better Wayland support

### Known Limitations (Current Implementation)

1. **Synchronous operations**: All clipboard operations complete synchronously using arboard.
   True async is deferred to Phase 6.

2. **Primary selection on Wayland**: Requires `zwp_primary_selection_v1` protocol.
   Currently uses arboard's X11/Wayland abstraction.

3. **Cancel behavior**: `native_clipboard_cancel()` silently removes completed data rather
   than firing `CLIPBOARD_ERR_CANCELLED` for pending operations (no pending state exists
   in synchronous implementation).

4. **Binary custom formats**: Custom `application/*` formats containing binary (non-UTF-8)
   data will be stored as lossy UTF-8 text, which corrupts non-text content. This is because
   arboard doesn't support raw MIME type storage. Only text-based formats (JSON, XML, etc.)
   are reliably preserved.

5. **SVG validation**: The `image/svg+xml` format uses heuristic validation (checking for
   `<svg` tags and XML declaration) rather than full XML parsing. Valid SVG with unusual
   formatting may be rejected; non-SVG XML containing `<svg>` elements may be accepted.

6. **Image hash for change detection**: Change notifications use a hash of the first 256
   bytes of image data (plus dimensions) for performance. Two images differing only after
   byte 256 would have the same hash, though this is unlikely in practice due to distinct
   PNG/JPEG headers.

---

## 10. Security Considerations

### 10.1 No Implicit Monitoring

The API does **not** include automatic clipboard change notifications by default. Apps must explicitly request paste operations. This prevents:
- Clipboard snooping by malicious components
- Privacy violations from clipboard history

Future change notification support (if added) will require explicit opt-in.

### 10.2 Sensitive Data

The `native_clipboard_write_add_sensitive` function marks data for exclusion from:
- Clipboard manager history
- Cloud clipboard sync
- Clipboard monitoring tools

Support varies by platform. Check `CLIPBOARD_CAP_SENSITIVE` before relying on this.

### 10.3 Data Lifetime

- Completed data auto-releases after 30 seconds to prevent memory leaks
- Write handles auto-cancel after 60 seconds to prevent resource exhaustion
- Apps SHOULD call `native_clipboard_release` promptly after retrieving data
- Format list pointers are invalidated by subsequent clipboard operations

### 10.4 Input Validation

The implementation must:
- Validate MIME type strings (reject invalid characters)
- Limit maximum data size per format (suggest 100MB)
- Limit total number of formats per write (suggest 32)
- Handle malformed data from other applications gracefully

---

## 11. Testing Strategy

### 11.1 Unit Tests

- MIME type normalization
- Error code propagation
- Timeout handling
- Handle lifecycle

### 11.2 Integration Tests

- Copy/paste roundtrip for each supported MIME type
- Multi-format write with single-format read
- Concurrent operations
- Cross-process copy/paste (manual or with helper process)

### 11.3 Platform-Specific Tests

- Primary selection on X11/Wayland
- CF_HTML header handling on Windows
- Large image transfer (>1MB)

---

## 12. Integration Points

### 12.1 Event Loop Integration

The clipboard API integrates with the native event loop through the existing event dispatch mechanism:

```
Event Loop:
    ┌─────────────────────────────────────────────────┐
    │                native_poll_event()              │
    │                       │                         │
    │         ┌─────────────┼─────────────┐           │
    │         ▼             ▼             ▼           │
    │    Mouse/Key    Clipboard    Timer/Frame        │
    │     events       events       events            │
    │  (0-99)        (200-299)     (100-199)          │
    └─────────────────────────────────────────────────┘

Clipboard events fire through the same NativeEventData structure.
```

**Trust boundary:** The implementation handles platform clipboard APIs (potentially untrusted data from other applications). All data from clipboard must be validated before use.

### 12.2 NativePlatform Integration

```
NativePlatform
    ├── window management
    ├── element tree
    ├── event dispatch  ◄─── clipboard events flow here
    └── clipboard state ◄─── new: ClipboardState struct
```

The `ClipboardState` struct is owned by `AppState` alongside existing window and element state.

### 12.3 Sigil FFI Boundary

All clipboard functions are exposed via C FFI for Sigil integration:

| Sigil Declaration | Rust Implementation |
|-------------------|---------------------|
| `⊞ "C" { rite native_clipboard_* }` | `#[no_mangle] pub extern "C" fn` |

**Contract:** Sigil provides valid pointers; Rust validates sizes and null-terminators.

### 12.4 Parent Spec Dependencies

| Spec | Relationship |
|------|--------------|
| NATIVE-RENDERING-SPEC.md | Parent spec - event loop, NativePlatform |
| (Future) DRAG-DROP-SPEC.md | Related - shares MIME negotiation concepts |

---

## 13. Open Questions

### 13.1 Primary Selection Behavior ❓

**Question:** How should primary selection behave on platforms without it (Windows, macOS)?

**Options:**
1. Silently fall back to standard clipboard
2. Return capability flag and let caller decide
3. Fail with `ERR_UNAVAILABLE`

**Current decision:** Option 1 (silent fallback) — documented in spec but may reconsider.

### 13.2 Large Data Streaming ❓

**Question:** Should we support streaming for very large clipboard data (>100MB)?

**Options:**
1. Current design: Copy all data to internal buffer
2. Chunked read: `native_clipboard_read_chunk(callback_id, offset, len)`
3. Memory-mapped: Return file descriptor for large data

**Current decision:** Option 1 for simplicity. May revisit if memory pressure becomes an issue.

### 13.3 Clipboard Change Notifications ❓

**Question:** Should we add passive clipboard monitoring?

**Options:**
1. Never - privacy concern, not needed for most apps
2. Future capability with explicit opt-in and capability flag
3. Platform-specific: only where easily supported

**Current decision:** Reserved `CAP_CHANGE_NOTIFY` flag for future use. Not in Phase 1-3.

### 13.4 Thread Safety ❓

**Question:** Can clipboard functions be called from any thread?

**Current assumption:** All FFI calls occur on main thread (same as other native platform calls). Not yet validated for all platforms.

### 13.5 Error Recovery ❓

**Question:** What happens if a write_begin() handle is never committed or cancelled?

**Current design:** Auto-cancel after 60 seconds. Should we also limit max concurrent write handles (suggest: 4)?

---

## Appendix A: Event Constants (Current)

For reference, current event constants in the codebase:

```rust
// Core events (0-99)
pub const EVENT_CLICK: i32 = 0;
pub const EVENT_DBLCLICK: i32 = 1;
pub const EVENT_MOUSEDOWN: i32 = 2;
pub const EVENT_MOUSEUP: i32 = 3;
pub const EVENT_MOUSEMOVE: i32 = 4;
pub const EVENT_MOUSEENTER: i32 = 5;
pub const EVENT_MOUSELEAVE: i32 = 6;
pub const EVENT_FOCUS: i32 = 7;
pub const EVENT_BLUR: i32 = 8;
pub const EVENT_KEYDOWN: i32 = 10;
pub const EVENT_KEYUP: i32 = 11;
pub const EVENT_INPUT: i32 = 12;
pub const EVENT_SCROLL: i32 = 13;
pub const EVENT_RESIZE: i32 = 14;
pub const EVENT_CLOSE: i32 = 15;

// Timer events (100-199)
pub const EVENT_TIMEOUT: i32 = 20;         // TODO: Move to 100
pub const EVENT_ANIMATION_FRAME: i32 = 21; // TODO: Move to 101

// Clipboard events (200-299)
pub const EVENT_CLIPBOARD_FORMATS_AVAILABLE: i32 = 200;
pub const EVENT_CLIPBOARD_DATA_READY: i32 = 201;
pub const EVENT_CLIPBOARD_WRITE_COMPLETE: i32 = 202;
pub const EVENT_CLIPBOARD_ERROR: i32 = 203;
pub const EVENT_CLIPBOARD_CHANGED: i32 = 204;
```

---

## Appendix B: Migration from Deprecated API

### Old API (Deprecated)

```sigil
native_clipboard_read(out_buf: *mut u8, max_len: usize) -> usize;
native_clipboard_write(content: *const u8);
```

### Migration Path

**Reading:**
```
// Old (synchronous, may block)
len ← native_clipboard_read(buf, max_len)

// New (asynchronous)
native_clipboard_read_format(0, "text/plain", callback_id)
// ... in event handler:
len ← native_clipboard_get_data(callback_id, buf, max_len)
native_clipboard_release(callback_id)
```

**Writing:**
```
// Old (synchronous)
native_clipboard_write(text)

// New (asynchronous)
h ← native_clipboard_write_begin(0)
native_clipboard_write_add_format(h, "text/plain", text, len(text))
native_clipboard_write_commit(h, callback_id)
```

---

## Changelog

### v0.5.1 (2025-02-18)
- **Audit fixes**: Addressed 4 issues from code review
- Fixed target-specific change notifications (clipboard vs primary now tracked separately)
- Improved SVG validation with `is_likely_svg()` heuristic function
- Added doc comments for image hash performance optimization
- Documented known limitations: binary custom formats, SVG validation, image hash
- Added 6 new tests for duplicate subscription prevention and SVG validation

### v0.5.0 (2025-02-17)
- **Phase 5 Complete**: Advanced Features
- Added `image/svg+xml` format support
- Added custom application formats (`application/*`)
- Added clipboard change notifications (polling-based, 500ms)
- Added chunked read API (`native_clipboard_read_chunk`)
- Added `native_clipboard_subscribe_changes` and `native_clipboard_unsubscribe_changes`
- Added `EVENT_CLIPBOARD_CHANGED` event constant
- Added capability flags: CAP_SVG, CAP_CUSTOM_FORMATS, CAP_CHUNKED_READ
- Updated CAP_CHANGE_NOTIFY to reflect implementation

### v0.4.0 (2025-02-17)
- Added Type Architecture section (Section 2)
- Added Integration Points section (Section 12)
- Added Open Questions section (Section 13)
- Fixed section numbering throughout
- Updated pseudocode to use SPEC-FORMATTING.md style
- Added status markers to section headers
- Compliance: SPEC-FORMATTING v1.0.0, SDD v1.1.0

### v0.3.0 (2025-02-17)
- Added compliance metadata to header

### v0.2.0 (2025-02-17)
- Added event system integration section
- Added `native_clipboard_get_formats_data` function
- Added `native_clipboard_cancel` and `native_clipboard_release` functions
- Added `native_clipboard_write_add_sensitive` function
- Added `native_clipboard_api_version` function
- Added explicit data lifecycle documentation
- Added empty clipboard handling specification
- Added timeout constants and behavior
- Added MIME type normalization rules
- Added encoding handling documentation
- Added platform format mapping table
- Added reserved event constant ranges
- Added non-goals section
- Fixed event constant range (200+ for clipboard)
- Fixed platform model descriptions (X11 owner-based, Wayland primary selection)
- Updated capability flags

### v0.1.0 (2025-02-17)
- Initial draft
