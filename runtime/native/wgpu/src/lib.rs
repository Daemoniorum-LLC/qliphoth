//! wgpu Native Backend for Qliphoth
//!
//! This crate provides GPU-accelerated native rendering for Qliphoth applications
//! using winit + wgpu + taffy for layout.
//!
//! # Architecture
//!
//! ```text
//! Sigil VNode → NativePlatform → This Crate → wgpu Pipeline → GPU → Pixels
//!                                     ↓
//!                               taffy (layout)
//!                                     ↓
//!                              glyphon (text)
//! ```
//!
//! # Advantages over GTK
//!
//! - Consistent rendering across all platforms (no native widget differences)
//! - Full control over appearance (custom themes, animations)
//! - GPU-accelerated (smooth scrolling, transitions)
//! - Better suited for IDE/editor applications like Wraith
//!
//! # Trade-offs
//!
//! - Doesn't use native widgets (no native look & feel)
//! - More code to maintain for custom rendering
//! - Need to implement accessibility ourselves

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use taffy::prelude::*;

// =============================================================================
// Bundled Fonts (Phase 3)
// =============================================================================

/// Noto Sans Regular font data (bundled at compile time)
static NOTO_SANS_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");

/// Noto Sans Bold font data (bundled at compile time)
static NOTO_SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSans-Bold.ttf");

// =============================================================================
// GPU Types (Phase 2)
// =============================================================================

/// Render mode selection - determines software vs GPU rendering path
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Software rendering via CPU framebuffer (used for tests)
    Software,
    /// GPU rendering via wgpu (used in production)
    #[allow(dead_code)] // Will be used when GPU path is activated
    Gpu,
}

impl Default for RenderMode {
    fn default() -> Self {
        // Default to software for backward compatibility with tests
        RenderMode::Software
    }
}

/// GPU state for a window - contains all wgpu resources
#[cfg(not(test))]
pub struct GpuState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub max_instances: usize,
}

/// Vertex for rectangle rendering (unit quad)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],   // Normalized position within quad (0-1)
    pub tex_coords: [f32; 2], // For SDF-based corner rounding
}

/// Per-rectangle instance data for GPU instanced rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RectInstance {
    pub rect: [f32; 4],       // x, y, width, height in pixels
    pub color: [f32; 4],      // RGBA (0.0-1.0)
    pub border_radius: f32,   // Corner radius in pixels
    pub opacity: f32,         // Overall opacity multiplier
    pub _padding: [f32; 2],   // Alignment to 16 bytes
}

/// Uniform data for the shader (viewport info)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub viewport_size: [f32; 2],
    pub _padding: [f32; 2],
}

// Unit quad vertices (will be transformed by instance data)
#[cfg(not(test))]
const QUAD_VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [1.0, 0.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [1.0, 1.0], tex_coords: [1.0, 1.0] },
    Vertex { position: [0.0, 1.0], tex_coords: [0.0, 1.0] },
];

#[cfg(not(test))]
const QUAD_INDICES: &[u16] = &[0, 1, 2, 0, 2, 3];

// =============================================================================
// WGSL Shader - SDF Rounded Rectangles
// =============================================================================

#[cfg(not(test))]
const RECT_SHADER: &str = r#"
// Uniforms
struct Uniforms {
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Vertex input (unit quad)
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

// Instance input (per rectangle)
struct InstanceInput {
    @location(2) rect: vec4<f32>,         // x, y, width, height
    @location(3) color: vec4<f32>,        // RGBA
    @location(4) border_radius: f32,
    @location(5) opacity: f32,
}

// Vertex output
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_coords: vec2<f32>,
    @location(1) rect_size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) border_radius: f32,
    @location(4) opacity: f32,
}

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let rect_pos = instance.rect.xy;
    let rect_size = instance.rect.zw;

    // Transform unit quad to rectangle position
    let world_pos = rect_pos + vertex.position * rect_size;

    // Convert to clip space (NDC): [-1, 1] range
    // Origin at top-left, Y increases downward
    let ndc_x = (world_pos.x / uniforms.viewport_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (world_pos.y / uniforms.viewport_size.y) * 2.0;

    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.local_coords = vertex.tex_coords * rect_size;
    out.rect_size = rect_size;
    out.color = instance.color;
    out.border_radius = instance.border_radius;
    out.opacity = instance.opacity;

    return out;
}

// Signed distance function for rounded rectangle
fn sd_rounded_rect(p: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let half_size = size * 0.5;
    let centered_p = p - half_size;

    // Clamp radius to not exceed half the smallest dimension
    let r = min(radius, min(half_size.x, half_size.y));

    // Calculate distance to rounded rectangle
    let q = abs(centered_p) - half_size + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate SDF for anti-aliased edges
    let dist = sd_rounded_rect(in.local_coords, in.rect_size, in.border_radius);

    // Anti-aliased edge (smooth step over ~1 pixel)
    let alpha = 1.0 - smoothstep(-0.5, 0.5, dist);

    // Apply opacity
    let final_alpha = alpha * in.color.a * in.opacity;

    // Premultiplied alpha output for proper blending
    return vec4<f32>(in.color.rgb * final_alpha, final_alpha);
}
"#;

// =============================================================================
// Core Types
// =============================================================================

/// A renderable element in our custom UI
#[derive(Debug, Clone)]
struct Element {
    #[allow(dead_code)] // Used for debugging and introspection
    handle: usize,
    #[allow(dead_code)] // Used for debugging and introspection
    tag: String,
    text_content: Option<String>,
    attributes: HashMap<String, String>,
    styles: StyleProperties,
    children: Vec<usize>,
    parent: Option<usize>,
    layout_node: Option<NodeId>,
}

/// Position type for CSS positioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
    Fixed,
}

/// Overflow behavior for containers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    #[default]
    Visible,
    Hidden,
    Scroll,
}

/// Parsed CSS-like style properties
#[derive(Debug, Clone)]
struct StyleProperties {
    // Layout (taffy)
    display: taffy::Display,
    flex_direction: taffy::FlexDirection,
    justify_content: Option<taffy::JustifyContent>,
    align_items: Option<taffy::AlignItems>,
    flex_grow: f32,
    flex_shrink: f32,
    width: taffy::Dimension,
    height: taffy::Dimension,
    min_width: taffy::Dimension,
    min_height: taffy::Dimension,
    max_width: taffy::Dimension,
    max_height: taffy::Dimension,
    margin: taffy::Rect<taffy::LengthPercentageAuto>,
    padding: taffy::Rect<taffy::LengthPercentage>,
    gap: taffy::Size<taffy::LengthPercentage>,

    // Positioning (Phase 4)
    position: Position,
    inset: taffy::Rect<taffy::LengthPercentageAuto>,  // top, right, bottom, left

    // Grid layout (Phase 4)
    grid_template_columns: Vec<taffy::TrackSizingFunction>,
    grid_template_rows: Vec<taffy::TrackSizingFunction>,
    grid_column: taffy::Line<taffy::GridPlacement>,
    grid_row: taffy::Line<taffy::GridPlacement>,

    // Overflow & scrolling (Phase 4)
    overflow: Overflow,
    scroll_offset_x: f32,
    scroll_offset_y: f32,

    // Z-index (Phase 4)
    z_index: i32,

    // Visual (custom rendering)
    background_color: Option<Color>,
    border_color: Option<Color>,
    border_width: f32,
    border_radius: f32,
    color: Option<Color>,
    font_size: f32,
    font_weight: u16,
    opacity: f32,
}

impl Default for StyleProperties {
    fn default() -> Self {
        Self {
            display: taffy::Display::Flex,
            flex_direction: taffy::FlexDirection::Column,
            justify_content: None,
            align_items: None,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            width: taffy::Dimension::Auto,
            height: taffy::Dimension::Auto,
            min_width: taffy::Dimension::Auto,
            min_height: taffy::Dimension::Auto,
            max_width: taffy::Dimension::Auto,
            max_height: taffy::Dimension::Auto,
            margin: taffy::Rect {
                left: taffy::LengthPercentageAuto::Length(0.0),
                right: taffy::LengthPercentageAuto::Length(0.0),
                top: taffy::LengthPercentageAuto::Length(0.0),
                bottom: taffy::LengthPercentageAuto::Length(0.0),
            },
            padding: taffy::Rect {
                left: length(0.0),
                right: length(0.0),
                top: length(0.0),
                bottom: length(0.0),
            },
            gap: taffy::Size {
                width: length(0.0),
                height: length(0.0),
            },
            // Positioning (Phase 4)
            position: Position::Relative,
            inset: taffy::Rect {
                left: taffy::LengthPercentageAuto::Auto,
                right: taffy::LengthPercentageAuto::Auto,
                top: taffy::LengthPercentageAuto::Auto,
                bottom: taffy::LengthPercentageAuto::Auto,
            },
            // Grid (Phase 4)
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_column: taffy::Line { start: taffy::GridPlacement::Auto, end: taffy::GridPlacement::Auto },
            grid_row: taffy::Line { start: taffy::GridPlacement::Auto, end: taffy::GridPlacement::Auto },
            // Overflow (Phase 4)
            overflow: Overflow::Visible,
            scroll_offset_x: 0.0,
            scroll_offset_y: 0.0,
            // Z-index (Phase 4)
            z_index: 0,
            // Visual
            background_color: None,
            border_color: None,
            border_width: 0.0,
            border_radius: 0.0,
            color: Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
            font_size: 16.0,
            font_weight: 400,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
    }
}

/// Internal native event representation
#[derive(Debug, Clone)]
pub enum NativeEvent {
    Click { x: f32, y: f32, button: i32, callback_id: u64 },
    DblClick { x: f32, y: f32, button: i32, callback_id: u64 },
    MouseDown { x: f32, y: f32, button: i32, callback_id: u64 },
    MouseUp { x: f32, y: f32, button: i32, callback_id: u64 },
    MouseMove { x: f32, y: f32, callback_id: u64 },
    MouseEnter { x: f32, y: f32, callback_id: u64 },
    MouseLeave { x: f32, y: f32, callback_id: u64 },
    KeyDown { key: i32, modifiers: i32, callback_id: u64 },
    KeyUp { key: i32, modifiers: i32, callback_id: u64 },
    TextInput { text: String, callback_id: u64 },
    Focus { callback_id: u64 },
    Blur { callback_id: u64 },
    Scroll { delta_x: f32, delta_y: f32, callback_id: u64 },
    Resize { width: u32, height: u32 },
    Close,
    AnimationFrame { callback_id: u64 },
    Timeout { callback_id: u64 },
    // Clipboard events
    ClipboardFormatsAvailable { callback_id: u64, format_count: usize },
    ClipboardDataReady { callback_id: u64, data_size: usize },
    ClipboardWriteComplete { callback_id: u64 },
    ClipboardError { callback_id: u64, error_code: i32 },
}

impl NativeEvent {
    /// Convert internal event to FFI-compatible NativeEventData
    fn to_event_data(&self) -> NativeEventData {
        match self {
            NativeEvent::Click { x, y, button, callback_id } => NativeEventData {
                event_type: EVENT_CLICK,
                callback_id: *callback_id,
                x: *x, y: *y, button: *button,
                ..Default::default()
            },
            NativeEvent::DblClick { x, y, button, callback_id } => NativeEventData {
                event_type: EVENT_DBLCLICK,
                callback_id: *callback_id,
                x: *x, y: *y, button: *button,
                ..Default::default()
            },
            NativeEvent::MouseDown { x, y, button, callback_id } => NativeEventData {
                event_type: EVENT_MOUSEDOWN,
                callback_id: *callback_id,
                x: *x, y: *y, button: *button,
                ..Default::default()
            },
            NativeEvent::MouseUp { x, y, button, callback_id } => NativeEventData {
                event_type: EVENT_MOUSEUP,
                callback_id: *callback_id,
                x: *x, y: *y, button: *button,
                ..Default::default()
            },
            NativeEvent::MouseMove { x, y, callback_id } => NativeEventData {
                event_type: EVENT_MOUSEMOVE,
                callback_id: *callback_id,
                x: *x, y: *y,
                ..Default::default()
            },
            NativeEvent::MouseEnter { x, y, callback_id } => NativeEventData {
                event_type: EVENT_MOUSEENTER,
                callback_id: *callback_id,
                x: *x, y: *y,
                ..Default::default()
            },
            NativeEvent::MouseLeave { x, y, callback_id } => NativeEventData {
                event_type: EVENT_MOUSELEAVE,
                callback_id: *callback_id,
                x: *x, y: *y,
                ..Default::default()
            },
            NativeEvent::KeyDown { key, modifiers, callback_id } => NativeEventData {
                event_type: EVENT_KEYDOWN,
                callback_id: *callback_id,
                key: *key, modifiers: *modifiers,
                ..Default::default()
            },
            NativeEvent::KeyUp { key, modifiers, callback_id } => NativeEventData {
                event_type: EVENT_KEYUP,
                callback_id: *callback_id,
                key: *key, modifiers: *modifiers,
                ..Default::default()
            },
            NativeEvent::TextInput { text, callback_id } => {
                // Store text in thread-local buffer and return pointer to it
                let (ptr, len) = TEXT_INPUT_BUFFER.with(|buf| {
                    let cstring = std::ffi::CString::new(text.as_str()).unwrap_or_default();
                    let len = cstring.as_bytes().len();
                    *buf.borrow_mut() = cstring;
                    (buf.borrow().as_ptr(), len)
                });
                NativeEventData {
                    event_type: EVENT_TEXTINPUT,
                    callback_id: *callback_id,
                    text_ptr: ptr,
                    text_len: len,
                    ..Default::default()
                }
            }
            NativeEvent::Focus { callback_id } => NativeEventData {
                event_type: EVENT_FOCUS,
                callback_id: *callback_id,
                ..Default::default()
            },
            NativeEvent::Blur { callback_id } => NativeEventData {
                event_type: EVENT_BLUR,
                callback_id: *callback_id,
                ..Default::default()
            },
            NativeEvent::Scroll { delta_x, delta_y, callback_id } => NativeEventData {
                event_type: EVENT_SCROLL,
                callback_id: *callback_id,
                delta_x: *delta_x, delta_y: *delta_y,
                ..Default::default()
            },
            NativeEvent::Resize { width, height } => NativeEventData {
                event_type: EVENT_RESIZE,
                width: *width, height: *height,
                ..Default::default()
            },
            NativeEvent::Close => NativeEventData {
                event_type: EVENT_CLOSE,
                ..Default::default()
            },
            NativeEvent::AnimationFrame { callback_id } => NativeEventData {
                event_type: EVENT_ANIMATION_FRAME,
                callback_id: *callback_id,
                ..Default::default()
            },
            NativeEvent::Timeout { callback_id } => NativeEventData {
                event_type: EVENT_TIMEOUT,
                callback_id: *callback_id,
                ..Default::default()
            },
            // Clipboard events
            NativeEvent::ClipboardFormatsAvailable { callback_id, format_count } => NativeEventData {
                event_type: EVENT_CLIPBOARD_FORMATS_AVAILABLE,
                callback_id: *callback_id,
                key: *format_count as i32, // format_count stored in key field per spec
                ..Default::default()
            },
            NativeEvent::ClipboardDataReady { callback_id, data_size } => NativeEventData {
                event_type: EVENT_CLIPBOARD_DATA_READY,
                callback_id: *callback_id,
                width: (*data_size & 0xFFFFFFFF) as u32,  // low 32 bits
                height: ((*data_size >> 32) & 0xFFFFFFFF) as u32, // high 32 bits
                ..Default::default()
            },
            NativeEvent::ClipboardWriteComplete { callback_id } => NativeEventData {
                event_type: EVENT_CLIPBOARD_WRITE_COMPLETE,
                callback_id: *callback_id,
                ..Default::default()
            },
            NativeEvent::ClipboardError { callback_id, error_code } => NativeEventData {
                event_type: EVENT_CLIPBOARD_ERROR,
                callback_id: *callback_id,
                button: *error_code, // error code stored in button field per spec
                ..Default::default()
            },
        }
    }
}

// =============================================================================
// Text System (Phase 3)
// =============================================================================

/// Text rendering system using cosmic-text for shaping and layout
struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl TextSystem {
    /// Create a new text system with bundled fonts
    fn new() -> Self {
        let mut font_system = FontSystem::new();

        // Load bundled fonts
        font_system.db_mut().load_font_data(NOTO_SANS_REGULAR.to_vec());
        font_system.db_mut().load_font_data(NOTO_SANS_BOLD.to_vec());

        Self {
            font_system,
            swash_cache: SwashCache::new(),
        }
    }

    /// Measure text dimensions for layout
    fn measure_text(&mut self, text: &str, font_size: f32, max_width: Option<f32>) -> (f32, f32) {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        let width = max_width.unwrap_or(f32::MAX);
        buffer.set_size(&mut self.font_system, Some(width), None);

        let attrs = Attrs::new().family(Family::SansSerif);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);

        // Shape the text
        buffer.shape_until_scroll(&mut self.font_system, false);

        // Calculate dimensions
        let mut total_width: f32 = 0.0;
        let mut total_height: f32 = 0.0;

        for run in buffer.layout_runs() {
            let line_width = run.line_w;
            total_width = total_width.max(line_width);
            total_height += metrics.line_height;
        }

        // Ensure minimum height for empty text
        if total_height == 0.0 && !text.is_empty() {
            total_height = metrics.line_height;
        }

        (total_width.ceil(), total_height.ceil())
    }

    /// Render text to a pixel buffer
    /// Returns Vec of TextGlyph for each glyph to render
    fn render_text(
        &mut self,
        text: &str,
        font_size: f32,
        color: Color,
        max_width: f32,
    ) -> Vec<TextGlyph> {
        let metrics = Metrics::new(font_size, font_size * 1.2);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);

        buffer.set_size(&mut self.font_system, Some(max_width), None);

        let attrs = Attrs::new().family(Family::SansSerif);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut glyphs = Vec::new();

        for run in buffer.layout_runs() {
            for glyph in run.glyphs.iter() {
                // physical() takes an offset (x, y) and scale factor
                // We pass the line's Y position as the Y offset
                let physical_glyph = glyph.physical((0.0, run.line_y), 1.0);

                if let Some(image) = self.swash_cache.get_image(&mut self.font_system, physical_glyph.cache_key) {
                    glyphs.push(TextGlyph {
                        x: physical_glyph.x,
                        y: physical_glyph.y,
                        width: image.placement.width as u32,
                        height: image.placement.height as u32,
                        left: image.placement.left,
                        top: image.placement.top,
                        data: image.data.clone(),
                        color,
                    });
                }
            }
        }

        glyphs
    }
}

/// Rendered glyph data for drawing to framebuffer
struct TextGlyph {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    data: Vec<u8>,
    color: Color,
}

// =============================================================================
// Global State
// =============================================================================

/// Send-safe wrapper for cached event data.
/// Nulls out the text_ptr since it's only valid until next poll call anyway.
#[derive(Debug, Clone, Copy)]
struct CachedEventData {
    event_type: i32,
    callback_id: u64,
    x: f32,
    y: f32,
    button: i32,
    key: i32,
    modifiers: i32,
    text_len: usize,
    width: u32,
    height: u32,
    delta_x: f32,
    delta_y: f32,
}

impl From<NativeEventData> for CachedEventData {
    fn from(data: NativeEventData) -> Self {
        Self {
            event_type: data.event_type,
            callback_id: data.callback_id,
            x: data.x,
            y: data.y,
            button: data.button,
            key: data.key,
            modifiers: data.modifiers,
            text_len: data.text_len,
            width: data.width,
            height: data.height,
            delta_x: data.delta_x,
            delta_y: data.delta_y,
        }
    }
}

impl CachedEventData {
    fn to_native_event_data(self) -> NativeEventData {
        NativeEventData {
            event_type: self.event_type,
            callback_id: self.callback_id,
            x: self.x,
            y: self.y,
            button: self.button,
            key: self.key,
            modifiers: self.modifiers,
            text_ptr: std::ptr::null(), // Cannot cache pointer across threads
            text_len: self.text_len,
            width: self.width,
            height: self.height,
            delta_x: self.delta_x,
            delta_y: self.delta_y,
        }
    }
}

struct AppState {
    elements: HashMap<usize, Element>,
    windows: HashMap<usize, WindowState>,
    next_handle: usize,
    event_queue: Vec<NativeEvent>,
    callbacks: HashMap<u64, (usize, i32)>,
    layout_tree: TaffyTree<()>,
    // Timer state
    timers: HashMap<u64, Timer>,
    animation_frames: HashMap<u64, u64>, // frame_id -> callback_id
    next_timer_id: u64,
    // Text rendering system
    text_system: TextSystem,
    // Cached event for Sigil FFI compatibility (native_get_event_data)
    last_polled_event: Option<CachedEventData>,
    // Clipboard state
    clipboard: ClipboardState,
}

struct Timer {
    callback_id: u64,
    fire_at_ms: u64,
}

// =============================================================================
// Clipboard Types
// =============================================================================

/// Clipboard target selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardTarget {
    /// Standard clipboard (Ctrl+C / Ctrl+V)
    Clipboard = 0,
    /// Primary selection (X11/Wayland: highlight to copy, middle-click to paste)
    PrimarySelection = 1,
}

impl From<i32> for ClipboardTarget {
    fn from(value: i32) -> Self {
        match value {
            1 => ClipboardTarget::PrimarySelection,
            _ => ClipboardTarget::Clipboard,
        }
    }
}

/// Completed clipboard data awaiting retrieval
struct ClipboardCompletedData {
    /// Retrieved clipboard data
    data: Vec<u8>,
    /// For GetFormats responses: list of available formats
    formats: Option<Vec<String>>,
    /// Cached CStrings for format pointers (valid until this entry is released)
    format_cstrings: Vec<std::ffi::CString>,
    /// When this data was completed (for timeout tracking)
    completed_at: std::time::Instant,
}

/// A clipboard write operation in progress
struct ClipboardWriteBuilder {
    #[allow(dead_code)]
    target: ClipboardTarget,
    /// Format entries: (mime_type, data, is_sensitive)
    formats: Vec<(String, Vec<u8>, bool)>,
    /// When this write handle was created (for timeout tracking)
    created_at: std::time::Instant,
}

/// State for clipboard operations
struct ClipboardState {
    /// Completed data awaiting retrieval (keyed by callback_id)
    completed: HashMap<u64, ClipboardCompletedData>,
    /// Pending write builders (keyed by write_handle)
    write_handles: HashMap<u64, ClipboardWriteBuilder>,
    /// Next handle ID for write operations
    next_write_handle: u64,
    /// Arboard clipboard instance (lazily initialized)
    clipboard: Option<arboard::Clipboard>,
}

impl Default for ClipboardState {
    fn default() -> Self {
        Self {
            completed: HashMap::new(),
            write_handles: HashMap::new(),
            next_write_handle: 1,
            clipboard: None,
        }
    }
}

struct WindowState {
    // Window dimensions
    width: u32,
    height: u32,
    // Element tree
    root_element: Option<usize>,
    focused_element: Option<usize>,
    // Software framebuffer for rendering/testing (always present)
    framebuffer: Vec<Pixel>,
    // Render mode selection (used in GPU event loop)
    #[allow(dead_code)]
    render_mode: RenderMode,
    // GPU resources (only present in non-test builds with GPU mode)
    #[cfg(not(test))]
    gpu_state: Option<GpuState>,
    // Winit window handle (only present in non-test builds)
    #[cfg(not(test))]
    winit_window: Option<Arc<winit::window::Window>>,
}

/// Layout data returned to FFI callers
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Pixel color for test verification
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Event data structure returned from poll_event (matches spec §2.1)
///
/// # Safety
///
/// The `text_ptr` field requires special handling:
/// - For `EVENT_TEXTINPUT` events, `text_ptr` points to a null-terminated UTF-8 string
/// - **IMPORTANT**: The pointer is only valid until the next call to `native_poll_event`
///   or `native_poll_event_timeout`. Callers must copy the text immediately if needed.
/// - The pointer is stored in thread-local storage and will be overwritten on the next
///   text input event.
/// - For non-text events, `text_ptr` is null and `text_len` is 0.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativeEventData {
    pub event_type: i32,      // -1 = no event, else EVENT_* constant
    pub callback_id: u64,     // Which listener to invoke
    // Click/Mouse data
    pub x: f32,
    pub y: f32,
    pub button: i32,          // MouseButton as int
    // Key data
    pub key: i32,             // KeyCode as int
    pub modifiers: i32,       // Modifier flags
    // Text data (for TextInput events)
    /// Pointer to text content. **Only valid until next poll_event call.**
    /// Callers must copy the string immediately if persistence is needed.
    pub text_ptr: *const c_char,
    pub text_len: usize,
    // Resize data
    pub width: u32,
    pub height: u32,
    // Scroll data
    pub delta_x: f32,
    pub delta_y: f32,
}

impl Default for NativeEventData {
    fn default() -> Self {
        Self {
            event_type: -1,
            callback_id: 0,
            x: 0.0,
            y: 0.0,
            button: 0,
            key: 0,
            modifiers: 0,
            text_ptr: std::ptr::null(),
            text_len: 0,
            width: 0,
            height: 0,
            delta_x: 0.0,
            delta_y: 0.0,
        }
    }
}

// Event type constants (matches spec Appendix B)
pub const EVENT_CLICK: i32 = 0;
pub const EVENT_DBLCLICK: i32 = 1;
pub const EVENT_MOUSEDOWN: i32 = 2;
pub const EVENT_MOUSEUP: i32 = 3;
pub const EVENT_MOUSEMOVE: i32 = 4;
pub const EVENT_MOUSEENTER: i32 = 5;
pub const EVENT_MOUSELEAVE: i32 = 6;
pub const EVENT_KEYDOWN: i32 = 10;
pub const EVENT_KEYUP: i32 = 11;
pub const EVENT_TEXTINPUT: i32 = 12;
pub const EVENT_FOCUS: i32 = 20;
pub const EVENT_BLUR: i32 = 21;
pub const EVENT_SCROLL: i32 = 30;
pub const EVENT_RESIZE: i32 = 40;
pub const EVENT_CLOSE: i32 = 50;
pub const EVENT_ANIMATION_FRAME: i32 = 60;
pub const EVENT_TIMEOUT: i32 = 61;

// Mouse button constants
pub const MOUSE_LEFT: i32 = 0;
pub const MOUSE_RIGHT: i32 = 1;
pub const MOUSE_MIDDLE: i32 = 2;

// Modifier flags
pub const MODIFIER_NONE: i32 = 0;
pub const MODIFIER_SHIFT: i32 = 1;
pub const MODIFIER_CTRL: i32 = 2;
pub const MODIFIER_ALT: i32 = 4;
pub const MODIFIER_META: i32 = 8;

// Clipboard events (200-299 reserved for clipboard per CLIPBOARD-SPEC.md)
pub const EVENT_CLIPBOARD_FORMATS_AVAILABLE: i32 = 200;
pub const EVENT_CLIPBOARD_DATA_READY: i32 = 201;
pub const EVENT_CLIPBOARD_WRITE_COMPLETE: i32 = 202;
pub const EVENT_CLIPBOARD_ERROR: i32 = 203;

// Clipboard error codes
pub const CLIPBOARD_OK: i32 = 0;
pub const CLIPBOARD_ERR_UNAVAILABLE: i32 = 1;
pub const CLIPBOARD_ERR_FORMAT_NOT_FOUND: i32 = 2;
pub const CLIPBOARD_ERR_ACCESS_DENIED: i32 = 3;
pub const CLIPBOARD_ERR_TIMEOUT: i32 = 4;
pub const CLIPBOARD_ERR_EMPTY: i32 = 5;
pub const CLIPBOARD_ERR_CANCELLED: i32 = 6;
pub const CLIPBOARD_ERR_INVALID_HANDLE: i32 = 7;
pub const CLIPBOARD_ERR_INTERNAL: i32 = 99;

// Clipboard capability flags
pub const CLIPBOARD_CAP_READ: u32 = 1 << 0;
pub const CLIPBOARD_CAP_WRITE: u32 = 1 << 1;
pub const CLIPBOARD_CAP_PRIMARY: u32 = 1 << 2;
pub const CLIPBOARD_CAP_IMAGES: u32 = 1 << 3;
pub const CLIPBOARD_CAP_HTML: u32 = 1 << 4;
pub const CLIPBOARD_CAP_FILES: u32 = 1 << 5;
pub const CLIPBOARD_CAP_SENSITIVE: u32 = 1 << 6;
pub const CLIPBOARD_CAP_CHANGE_NOTIFY: u32 = 1 << 7;

// Clipboard timeouts (seconds)
pub const CLIPBOARD_DATA_LIFETIME_SECONDS: u64 = 30;
pub const CLIPBOARD_WRITE_HANDLE_TIMEOUT_SECONDS: u64 = 60;

// Thread-local buffer for text input events (persists until next poll_event call)
thread_local! {
    static TEXT_INPUT_BUFFER: std::cell::RefCell<std::ffi::CString> =
        std::cell::RefCell::new(std::ffi::CString::new("").unwrap());
}


static STATE: Lazy<Mutex<AppState>> = Lazy::new(|| {
    Mutex::new(AppState {
        elements: HashMap::new(),
        windows: HashMap::new(),
        next_handle: 1,
        event_queue: Vec::new(),
        callbacks: HashMap::new(),
        layout_tree: TaffyTree::new(),
        timers: HashMap::new(),
        animation_frames: HashMap::new(),
        next_timer_id: 1,
        text_system: TextSystem::new(),
        last_polled_event: None,
        clipboard: ClipboardState::default(),
    })
});

// =============================================================================
// Helper Functions
// =============================================================================

fn allocate_handle(state: &mut AppState) -> usize {
    let h = state.next_handle;
    state.next_handle += 1;
    h
}

/// Validate a pointer for writing. Returns false if null or misaligned.
/// Logs error in debug builds but doesn't panic (per spec: silent failures).
fn validate_ptr_for_write<T>(ptr: *mut T, location: &str) -> bool {
    if ptr.is_null() {
        log::debug!("{}: null pointer", location);
        return false;
    }
    if (ptr as usize) % std::mem::align_of::<T>() != 0 {
        log::error!("{}: misaligned pointer {:p} (alignment {})",
            location, ptr, std::mem::align_of::<T>());
        return false;
    }
    true
}

/// Process clipboard operation timeouts.
/// Removes expired completed data and write handles.
fn process_clipboard_timeouts(state: &mut AppState) {
    let now = std::time::Instant::now();

    // Timeout completed data after DATA_LIFETIME_SECONDS
    let data_timeout = std::time::Duration::from_secs(CLIPBOARD_DATA_LIFETIME_SECONDS);
    let expired_completed: Vec<u64> = state.clipboard.completed
        .iter()
        .filter(|(_, c)| now.duration_since(c.completed_at) > data_timeout)
        .map(|(&id, _)| id)
        .collect();

    for callback_id in expired_completed {
        state.clipboard.completed.remove(&callback_id);
    }

    // Timeout write handles after WRITE_HANDLE_TIMEOUT_SECONDS
    let write_timeout = std::time::Duration::from_secs(CLIPBOARD_WRITE_HANDLE_TIMEOUT_SECONDS);
    let expired_handles: Vec<u64> = state.clipboard.write_handles
        .iter()
        .filter(|(_, w)| now.duration_since(w.created_at) > write_timeout)
        .map(|(&id, _)| id)
        .collect();

    for handle in expired_handles {
        state.clipboard.write_handles.remove(&handle);
        // Silent cleanup - no event fired for timed-out write handles
    }
}

fn c_str_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // Check alignment for c_char (typically 1, so this rarely fails)
    if (ptr as usize) % std::mem::align_of::<c_char>() != 0 {
        log::error!("c_str_to_string: misaligned pointer {:p}", ptr);
        return String::new();
    }
    // Safety: We've verified non-null and alignment. CStr::from_ptr requires
    // that the memory is valid and null-terminated - caller contract.
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

/// Convert element tag to default taffy style
fn default_style_for_tag(tag: &str) -> taffy::Style {
    match tag {
        "div" | "section" | "article" | "main" | "nav" => {
            taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Column,
                ..Default::default()
            }
        }
        "span" => {
            taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Row,
                ..Default::default()
            }
        }
        "button" => {
            taffy::Style {
                display: taffy::Display::Flex,
                justify_content: Some(taffy::JustifyContent::Center),
                align_items: Some(taffy::AlignItems::Center),
                padding: taffy::Rect {
                    left: length(8.0),
                    right: length(8.0),
                    top: length(4.0),
                    bottom: length(4.0),
                },
                ..Default::default()
            }
        }
        _ => taffy::Style::default(),
    }
}

// =============================================================================
// FFI Functions - Window Management
// =============================================================================

#[no_mangle]
pub extern "C" fn native_create_window(
    title: *const c_char,
    width: c_int,
    height: c_int,
) -> usize {
    let _title = c_str_to_string(title);
    let mut state = STATE.lock();
    let handle = allocate_handle(&mut state);

    let w = width as u32;
    let h = height as u32;
    let pixel_count = (w * h) as usize;

    // Create window state with appropriate render mode
    let window_state = WindowState {
        width: w,
        height: h,
        root_element: None,
        focused_element: None,
        // Software framebuffer (always present for tests and fallback)
        framebuffer: vec![Pixel { r: 0, g: 0, b: 0, a: 0 }; pixel_count],
        // Use software mode for tests, GPU mode for production
        #[cfg(test)]
        render_mode: RenderMode::Software,
        #[cfg(not(test))]
        render_mode: RenderMode::Software, // Start in software, GPU init happens in event loop
        // GPU state initialized later in event loop
        #[cfg(not(test))]
        gpu_state: None,
        #[cfg(not(test))]
        winit_window: None,
    };

    state.windows.insert(handle, window_state);

    // Note: Actual winit window and GPU resources are created in native_run_event_loop()
    // This allows the event loop to own the window lifetime properly

    handle
}

#[no_mangle]
pub extern "C" fn native_destroy_window(handle: usize) {
    let mut state = STATE.lock();
    // Use cleanup_window to properly destroy all elements and callbacks
    state.cleanup_window(handle);
}

#[no_mangle]
pub extern "C" fn native_window_size(
    handle: usize,
    width: *mut c_int,
    height: *mut c_int,
) {
    let state = STATE.lock();
    let (w, h) = if let Some(window) = state.windows.get(&handle) {
        (window.width as c_int, window.height as c_int)
    } else {
        // Invalid handle returns 0,0 per spec
        (0, 0)
    };

    // Write output values with validation
    if validate_ptr_for_write(width, "native_window_size:width") {
        unsafe { *width = w; }
    }
    if validate_ptr_for_write(height, "native_window_size:height") {
        unsafe { *height = h; }
    }
}

#[no_mangle]
pub extern "C" fn native_set_window_title(_handle: usize, _title: *const c_char) {
    // Would update winit window title
}

#[no_mangle]
pub extern "C" fn native_set_root(window: usize, element: usize) {
    let mut state = STATE.lock();
    if let Some(win) = state.windows.get_mut(&window) {
        win.root_element = Some(element);
    }
}

#[no_mangle]
pub extern "C" fn native_get_root(window: usize) -> usize {
    let state = STATE.lock();
    state.windows.get(&window)
        .and_then(|w| w.root_element)
        .unwrap_or(0)
}

// =============================================================================
// FFI Functions - Element Creation
// =============================================================================

#[no_mangle]
pub extern "C" fn native_create_element(_window: usize, tag: *const c_char) -> usize {
    let tag = c_str_to_string(tag);
    let mut state = STATE.lock();
    let handle = allocate_handle(&mut state);

    // Create layout node
    let style = default_style_for_tag(&tag);
    let layout_node = state.layout_tree.new_leaf(style).ok();

    let element = Element {
        handle,
        tag,
        text_content: None,
        attributes: HashMap::new(),
        styles: StyleProperties::default(),
        children: Vec::new(),
        parent: None,
        layout_node,
    };

    state.elements.insert(handle, element);
    handle
}

#[no_mangle]
pub extern "C" fn native_destroy_element(handle: usize) {
    let mut state = STATE.lock();

    // Remove from layout tree
    if let Some(element) = state.elements.get(&handle) {
        if let Some(node) = element.layout_node {
            let _ = state.layout_tree.remove(node);
        }
    }

    state.elements.remove(&handle);
}

// =============================================================================
// FFI Compatibility Aliases (Sigil uses "widget" terminology)
// =============================================================================

/// Alias for native_create_element (Sigil FFI compatibility)
#[no_mangle]
pub extern "C" fn native_create_widget(window: usize, tag: *const c_char) -> usize {
    native_create_element(window, tag)
}

/// Alias for native_destroy_element (Sigil FFI compatibility)
#[no_mangle]
pub extern "C" fn native_destroy_widget(handle: usize) {
    native_destroy_element(handle)
}

#[no_mangle]
pub extern "C" fn native_create_text(_window: usize, content: *const c_char) -> usize {
    let content = c_str_to_string(content);
    let mut state = STATE.lock();
    let handle = allocate_handle(&mut state);

    // Text nodes get a leaf layout node
    let style = taffy::Style::default();
    let layout_node = state.layout_tree.new_leaf(style).ok();

    let element = Element {
        handle,
        tag: "#text".to_string(),
        text_content: Some(content),
        attributes: HashMap::new(),
        styles: StyleProperties::default(),
        children: Vec::new(),
        parent: None,
        layout_node,
    };

    state.elements.insert(handle, element);
    handle
}

// =============================================================================
// FFI Functions - Element Tree Manipulation
// =============================================================================

#[no_mangle]
pub extern "C" fn native_append_child(parent: usize, child: usize) {
    let mut state = STATE.lock();

    // Update parent's children list
    if let Some(parent_elem) = state.elements.get_mut(&parent) {
        parent_elem.children.push(child);
    }

    // Update child's parent
    if let Some(child_elem) = state.elements.get_mut(&child) {
        child_elem.parent = Some(parent);
    }

    // Update layout tree
    let parent_node = state.elements.get(&parent).and_then(|e| e.layout_node);
    let child_node = state.elements.get(&child).and_then(|e| e.layout_node);

    if let (Some(p), Some(c)) = (parent_node, child_node) {
        let _ = state.layout_tree.add_child(p, c);
    }
}

#[no_mangle]
pub extern "C" fn native_remove_child(parent: usize, child: usize) {
    let mut state = STATE.lock();

    // Update parent's children list
    if let Some(parent_elem) = state.elements.get_mut(&parent) {
        parent_elem.children.retain(|&c| c != child);
    }

    // Update child's parent
    if let Some(child_elem) = state.elements.get_mut(&child) {
        child_elem.parent = None;
    }

    // Update layout tree
    let parent_node = state.elements.get(&parent).and_then(|e| e.layout_node);
    let child_node = state.elements.get(&child).and_then(|e| e.layout_node);

    if let (Some(p), Some(c)) = (parent_node, child_node) {
        let _ = state.layout_tree.remove_child(p, c);
    }
}

#[no_mangle]
pub extern "C" fn native_insert_before(parent: usize, child: usize, before: usize) {
    let mut state = STATE.lock();

    // Find position of 'before' in parent's children
    let position = state.elements.get(&parent)
        .and_then(|p| p.children.iter().position(|&c| c == before));

    if let Some(pos) = position {
        // Update parent's children list
        if let Some(parent_elem) = state.elements.get_mut(&parent) {
            parent_elem.children.insert(pos, child);
        }

        // Update child's parent
        if let Some(child_elem) = state.elements.get_mut(&child) {
            child_elem.parent = Some(parent);
        }

        // Update layout tree
        let parent_node = state.elements.get(&parent).and_then(|e| e.layout_node);
        let child_node = state.elements.get(&child).and_then(|e| e.layout_node);

        if let (Some(p), Some(c)) = (parent_node, child_node) {
            let _ = state.layout_tree.insert_child_at_index(p, pos, c);
        }
    }
}

#[no_mangle]
pub extern "C" fn native_get_child_count(element: usize) -> usize {
    let state = STATE.lock();
    state.elements.get(&element)
        .map(|e| e.children.len())
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn native_get_child_at(element: usize, index: usize) -> usize {
    let state = STATE.lock();
    state.elements.get(&element)
        .and_then(|e| e.children.get(index).copied())
        .unwrap_or(0)
}

// =============================================================================
// FFI Functions - Layout Queries
// =============================================================================

#[no_mangle]
pub extern "C" fn native_compute_layout(window: usize) {
    let mut state = STATE.lock();
    state.compute_layout(window);
}

#[no_mangle]
pub extern "C" fn native_get_layout(element: usize, out_layout: *mut Layout) {
    if !validate_ptr_for_write(out_layout, "native_get_layout") {
        return;
    }

    let state = STATE.lock();
    let layout = state.get_layout(element).map(|l| Layout {
        x: l.location.x,
        y: l.location.y,
        width: l.size.width,
        height: l.size.height,
    }).unwrap_or_default();

    unsafe { *out_layout = layout; }
}

#[no_mangle]
pub extern "C" fn native_get_text_content(
    element: usize,
    out_buf: *mut c_char,
    buf_len: usize,
) -> usize {
    let state = STATE.lock();
    let content = state.elements.get(&element)
        .and_then(|e| e.text_content.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("");

    // If null or zero length, just return content length (query mode)
    if out_buf.is_null() || buf_len == 0 {
        return content.len();
    }

    // Validate buffer pointer for write
    if !validate_ptr_for_write(out_buf, "native_get_text_content") {
        return 0;
    }

    let bytes = content.as_bytes();
    let copy_len = bytes.len().min(buf_len - 1);

    // Safety: We've validated out_buf is non-null and aligned.
    // copy_len is bounded by both content and buffer size.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, copy_len);
        *out_buf.add(copy_len) = 0; // Null terminator
    }

    copy_len
}

// =============================================================================
// FFI Functions - Focus Management
// =============================================================================

#[no_mangle]
pub extern "C" fn native_focus(element: usize) {
    let mut state = STATE.lock();

    // Find which window owns this element
    let window_handle = find_window_for_element(&state, element);

    if let Some(wh) = window_handle {
        // Get previous focused element and collect blur callbacks
        let prev_focused = state.windows.get(&wh).and_then(|w| w.focused_element);

        // Emit blur event for previously focused element
        if let Some(prev) = prev_focused {
            if prev != element {
                let blur_callbacks = collect_focus_callbacks(&state, prev, EVENT_BLUR);
                for callback_id in blur_callbacks {
                    state.event_queue.push(NativeEvent::Blur { callback_id });
                }
            }
        }

        // Update focused element
        if let Some(win) = state.windows.get_mut(&wh) {
            win.focused_element = Some(element);
        }

        // Emit focus event for newly focused element
        let focus_callbacks = collect_focus_callbacks(&state, element, EVENT_FOCUS);
        for callback_id in focus_callbacks {
            state.event_queue.push(NativeEvent::Focus { callback_id });
        }
    }
}

#[no_mangle]
pub extern "C" fn native_blur(element: usize) {
    let mut state = STATE.lock();

    // Find which window owns this element
    let window_handle = find_window_for_element(&state, element);

    if let Some(wh) = window_handle {
        let is_focused = state.windows.get(&wh)
            .map(|w| w.focused_element == Some(element))
            .unwrap_or(false);

        if is_focused {
            // Emit blur event
            let blur_callbacks = collect_focus_callbacks(&state, element, EVENT_BLUR);
            for callback_id in blur_callbacks {
                state.event_queue.push(NativeEvent::Blur { callback_id });
            }

            // Clear focused element
            if let Some(win) = state.windows.get_mut(&wh) {
                win.focused_element = None;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn native_get_focused(window: usize) -> usize {
    let state = STATE.lock();
    state.windows.get(&window)
        .and_then(|w| w.focused_element)
        .unwrap_or(0)
}

/// Collect callbacks for focus/blur events (does NOT bubble per spec)
fn collect_focus_callbacks(state: &AppState, element: usize, event_type: i32) -> Vec<u64> {
    let mut callbacks = Vec::new();
    for (&callback_id, &(elem, evt)) in &state.callbacks {
        if elem == element && evt == event_type {
            callbacks.push(callback_id);
        }
    }
    callbacks
}

/// Helper: Find window that contains an element by traversing to root
fn find_window_for_element(state: &AppState, element: usize) -> Option<usize> {
    // For now, simple approach: check all windows for this element as root
    // In a real impl, we'd traverse parent chain to find root
    for (wh, win) in &state.windows {
        if win.root_element == Some(element) {
            return Some(*wh);
        }
        // Check if element is descendant of root
        if let Some(root) = win.root_element {
            if is_descendant(state, element, root) {
                return Some(*wh);
            }
        }
    }
    None
}

fn is_descendant(state: &AppState, element: usize, root: usize) -> bool {
    if element == root {
        return true;
    }
    if let Some(elem) = state.elements.get(&root) {
        for &child in &elem.children {
            if is_descendant(state, element, child) {
                return true;
            }
        }
    }
    false
}

// =============================================================================
// FFI Functions - Widget Attributes & Styles
// =============================================================================

#[no_mangle]
pub extern "C" fn native_set_attribute(
    widget: usize,
    name: *const c_char,
    value: *const c_char,
) {
    let name = c_str_to_string(name);
    let value = c_str_to_string(value);

    let mut state = STATE.lock();
    if let Some(element) = state.elements.get_mut(&widget) {
        element.attributes.insert(name, value);
    }
}

#[no_mangle]
pub extern "C" fn native_remove_attribute(widget: usize, name: *const c_char) {
    let name = c_str_to_string(name);

    let mut state = STATE.lock();
    if let Some(element) = state.elements.get_mut(&widget) {
        element.attributes.remove(&name);
    }
}

#[no_mangle]
pub extern "C" fn native_set_text_content(widget: usize, content: *const c_char) {
    let content = c_str_to_string(content);

    let mut state = STATE.lock();
    if let Some(element) = state.elements.get_mut(&widget) {
        element.text_content = Some(content);
    }
}

#[no_mangle]
pub extern "C" fn native_set_style(
    widget: usize,
    property: *const c_char,
    value: *const c_char,
) {
    let property = c_str_to_string(property);
    let value = c_str_to_string(value);

    let mut state = STATE.lock();

    // Parse and apply style
    if let Some(element) = state.elements.get_mut(&widget) {
        apply_style_property(&mut element.styles, &property, &value);

        // Update taffy style
        if let Some(node) = element.layout_node {
            let taffy_style = styles_to_taffy(&element.styles);
            let _ = state.layout_tree.set_style(node, taffy_style);
        }
    }
}

fn apply_style_property(styles: &mut StyleProperties, property: &str, value: &str) {
    match property {
        "display" => {
            styles.display = match value {
                "flex" => taffy::Display::Flex,
                "grid" => taffy::Display::Grid,
                "none" => taffy::Display::None,
                _ => taffy::Display::Flex,
            };
        }
        "flex-direction" => {
            styles.flex_direction = match value {
                "row" => taffy::FlexDirection::Row,
                "row-reverse" => taffy::FlexDirection::RowReverse,
                "column" => taffy::FlexDirection::Column,
                "column-reverse" => taffy::FlexDirection::ColumnReverse,
                _ => taffy::FlexDirection::Row,
            };
        }
        "justify-content" => {
            styles.justify_content = Some(match value {
                "flex-start" | "start" => taffy::JustifyContent::FlexStart,
                "flex-end" | "end" => taffy::JustifyContent::FlexEnd,
                "center" => taffy::JustifyContent::Center,
                "space-between" => taffy::JustifyContent::SpaceBetween,
                "space-around" => taffy::JustifyContent::SpaceAround,
                "space-evenly" => taffy::JustifyContent::SpaceEvenly,
                _ => taffy::JustifyContent::FlexStart,
            });
        }
        "align-items" => {
            styles.align_items = Some(match value {
                "flex-start" | "start" => taffy::AlignItems::FlexStart,
                "flex-end" | "end" => taffy::AlignItems::FlexEnd,
                "center" => taffy::AlignItems::Center,
                "stretch" => taffy::AlignItems::Stretch,
                "baseline" => taffy::AlignItems::Baseline,
                _ => taffy::AlignItems::Stretch,
            });
        }
        "width" => {
            styles.width = parse_dimension(value);
        }
        "height" => {
            styles.height = parse_dimension(value);
        }
        "background-color" | "background" => {
            styles.background_color = parse_color(value);
        }
        "color" => {
            styles.color = parse_color(value);
        }
        "font-size" => {
            styles.font_size = parse_length(value).unwrap_or(16.0);
        }
        "opacity" => {
            styles.opacity = value.parse().unwrap_or(1.0);
        }
        "border-radius" => {
            styles.border_radius = parse_length(value).unwrap_or(0.0);
        }
        "border-width" => {
            styles.border_width = parse_length(value).unwrap_or(0.0);
        }
        "margin" => {
            if let Some(m) = parse_length(value) {
                styles.margin = taffy::Rect {
                    left: taffy::LengthPercentageAuto::Length(m),
                    right: taffy::LengthPercentageAuto::Length(m),
                    top: taffy::LengthPercentageAuto::Length(m),
                    bottom: taffy::LengthPercentageAuto::Length(m),
                };
            }
        }
        "padding" => {
            if let Some(p) = parse_length(value) {
                styles.padding = taffy::Rect {
                    left: length(p),
                    right: length(p),
                    top: length(p),
                    bottom: length(p),
                };
            }
        }
        "gap" => {
            if let Some(g) = parse_length(value) {
                styles.gap = taffy::Size {
                    width: length(g),
                    height: length(g),
                };
            }
        }
        // Phase 4: Positioning
        "position" => {
            styles.position = match value {
                "relative" => Position::Relative,
                "absolute" => Position::Absolute,
                "fixed" => Position::Fixed,
                _ => Position::Relative,
            };
        }
        "top" => {
            styles.inset.top = parse_length_percentage_auto(value);
        }
        "right" => {
            styles.inset.right = parse_length_percentage_auto(value);
        }
        "bottom" => {
            styles.inset.bottom = parse_length_percentage_auto(value);
        }
        "left" => {
            styles.inset.left = parse_length_percentage_auto(value);
        }
        // Phase 4: Grid layout
        "grid-template-columns" => {
            styles.grid_template_columns = parse_track_list(value);
        }
        "grid-template-rows" => {
            styles.grid_template_rows = parse_track_list(value);
        }
        "grid-column" => {
            styles.grid_column = parse_grid_line(value);
        }
        "grid-row" => {
            styles.grid_row = parse_grid_line(value);
        }
        // Phase 4: Overflow
        "overflow" => {
            styles.overflow = match value {
                "visible" => Overflow::Visible,
                "hidden" => Overflow::Hidden,
                "scroll" => Overflow::Scroll,
                "auto" => Overflow::Scroll,  // Treat auto as scroll
                _ => Overflow::Visible,
            };
        }
        // Phase 4: Z-index
        "z-index" => {
            styles.z_index = value.parse().unwrap_or(0);
        }
        // Flex properties
        "flex-grow" => {
            styles.flex_grow = value.parse().unwrap_or(0.0);
        }
        "flex-shrink" => {
            styles.flex_shrink = value.parse().unwrap_or(1.0);
        }
        "min-width" => {
            styles.min_width = parse_dimension(value);
        }
        "min-height" => {
            styles.min_height = parse_dimension(value);
        }
        "max-width" => {
            styles.max_width = parse_dimension(value);
        }
        "max-height" => {
            styles.max_height = parse_dimension(value);
        }
        _ => {}
    }
}

fn parse_length_percentage_auto(value: &str) -> taffy::LengthPercentageAuto {
    let value = value.trim();
    if value == "auto" {
        return taffy::LengthPercentageAuto::Auto;
    }
    if value.ends_with('%') {
        if let Ok(pct) = value.trim_end_matches('%').parse::<f32>() {
            return taffy::LengthPercentageAuto::Percent(pct / 100.0);
        }
    }
    if let Some(len) = parse_length(value) {
        return taffy::LengthPercentageAuto::Length(len);
    }
    taffy::LengthPercentageAuto::Auto
}

/// Parse a grid track list like "100px 1fr 2fr" or "repeat(3, 1fr)"
fn parse_track_list(value: &str) -> Vec<taffy::TrackSizingFunction> {
    let mut tracks = Vec::new();
    for part in value.split_whitespace() {
        if let Some(track) = parse_track_sizing(part) {
            tracks.push(track);
        }
    }
    tracks
}

/// Parse a single track sizing like "100px", "1fr", "auto", "minmax(100px, 1fr)"
fn parse_track_sizing(value: &str) -> Option<taffy::TrackSizingFunction> {
    let value = value.trim();

    if value == "auto" {
        return Some(taffy::TrackSizingFunction::Single(
            taffy::NonRepeatedTrackSizingFunction::AUTO
        ));
    }

    if value.ends_with("fr") {
        if let Ok(fr) = value.trim_end_matches("fr").parse::<f32>() {
            return Some(taffy::TrackSizingFunction::Single(
                taffy::NonRepeatedTrackSizingFunction::from_flex(fr)
            ));
        }
    }

    if let Some(len) = parse_length(value) {
        return Some(taffy::TrackSizingFunction::Single(
            taffy::NonRepeatedTrackSizingFunction::from_length(len)
        ));
    }

    None
}

/// Parse grid-column or grid-row like "1 / 3" or "span 2"
fn parse_grid_line(value: &str) -> taffy::Line<taffy::GridPlacement> {
    let parts: Vec<&str> = value.split('/').map(|s| s.trim()).collect();

    let start = parse_grid_placement(parts.first().copied().unwrap_or("auto"));
    let end = if parts.len() > 1 {
        parse_grid_placement(parts.get(1).copied().unwrap_or("auto"))
    } else {
        taffy::GridPlacement::Auto
    };

    taffy::Line { start, end }
}

fn parse_grid_placement(value: &str) -> taffy::GridPlacement {
    let value = value.trim();

    if value == "auto" {
        return taffy::GridPlacement::Auto;
    }

    if value.starts_with("span") {
        if let Ok(span) = value.trim_start_matches("span").trim().parse::<u16>() {
            return taffy::GridPlacement::from_span(span);
        }
    }

    if let Ok(line) = value.parse::<i16>() {
        return taffy::GridPlacement::from_line_index(line);
    }

    taffy::GridPlacement::Auto
}

fn parse_dimension(value: &str) -> taffy::Dimension {
    if value == "auto" {
        return taffy::Dimension::Auto;
    }
    if value.ends_with('%') {
        if let Ok(pct) = value.trim_end_matches('%').parse::<f32>() {
            return taffy::Dimension::Percent(pct / 100.0);
        }
    }
    if let Some(len) = parse_length(value) {
        return taffy::Dimension::Length(len);
    }
    taffy::Dimension::Auto
}

fn parse_length(value: &str) -> Option<f32> {
    let value = value.trim();
    if value.ends_with("px") {
        value.trim_end_matches("px").parse().ok()
    } else if value.ends_with("rem") {
        value.trim_end_matches("rem").parse::<f32>().ok().map(|v| v * 16.0)
    } else if value.ends_with("em") {
        value.trim_end_matches("em").parse::<f32>().ok().map(|v| v * 16.0)
    } else {
        value.parse().ok()
    }
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim();

    // Hex colors
    if value.starts_with('#') {
        let hex = &value[1..];
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
            return Some(Color { r, g, b, a: 1.0 });
        }
    }

    // Named colors (basic set)
    match value {
        "transparent" => Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
        "white" => Some(Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }),
        "black" => Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
        "red" => Some(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }),
        "green" => Some(Color { r: 0.0, g: 0.5, b: 0.0, a: 1.0 }),
        "blue" => Some(Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }),
        _ => None,
    }
}

fn styles_to_taffy(styles: &StyleProperties) -> taffy::Style {
    taffy::Style {
        display: styles.display,
        flex_direction: styles.flex_direction,
        justify_content: styles.justify_content,
        align_items: styles.align_items,
        flex_grow: styles.flex_grow,
        flex_shrink: styles.flex_shrink,
        size: taffy::Size {
            width: styles.width,
            height: styles.height,
        },
        min_size: taffy::Size {
            width: styles.min_width,
            height: styles.min_height,
        },
        max_size: taffy::Size {
            width: styles.max_width,
            height: styles.max_height,
        },
        margin: styles.margin,
        padding: styles.padding,
        gap: styles.gap,
        // Phase 4: Positioning
        position: match styles.position {
            Position::Relative => taffy::Position::Relative,
            Position::Absolute => taffy::Position::Absolute,
            Position::Fixed => taffy::Position::Absolute,  // Fixed treated as absolute in taffy
        },
        inset: styles.inset,
        // Phase 4: Grid layout
        grid_template_columns: styles.grid_template_columns.clone(),
        grid_template_rows: styles.grid_template_rows.clone(),
        grid_column: styles.grid_column,
        grid_row: styles.grid_row,
        // Phase 4: Overflow (taffy supports x/y separately)
        overflow: taffy::Point {
            x: match styles.overflow {
                Overflow::Visible => taffy::Overflow::Visible,
                Overflow::Hidden => taffy::Overflow::Clip,
                Overflow::Scroll => taffy::Overflow::Scroll,
            },
            y: match styles.overflow {
                Overflow::Visible => taffy::Overflow::Visible,
                Overflow::Hidden => taffy::Overflow::Clip,
                Overflow::Scroll => taffy::Overflow::Scroll,
            },
        },
        ..Default::default()
    }
}

// =============================================================================
// FFI Functions - Event Handling
// =============================================================================

#[no_mangle]
pub extern "C" fn native_add_event_listener(
    widget: usize,
    event_type: c_int,
    callback_id: u64,
) {
    let mut state = STATE.lock();
    state.callbacks.insert(callback_id, (widget, event_type));
}

#[no_mangle]
pub extern "C" fn native_remove_event_listener(
    _widget: usize,
    _event_type: c_int,
    callback_id: u64,
) {
    let mut state = STATE.lock();
    state.callbacks.remove(&callback_id);
}

// =============================================================================
// FFI Functions - Event Loop
// =============================================================================

/// Poll for a single event, filling out_event with data.
/// Also processes pending timers and animation frames before checking queue.
/// Returns event_type on success, -1 if no event available.
#[no_mangle]
pub extern "C" fn native_poll_event(out_event: *mut NativeEventData) -> i32 {
    let mut state = STATE.lock();

    // Process animation frames first
    let frames: Vec<_> = state.animation_frames.drain().collect();
    for (_frame_id, callback_id) in frames {
        state.event_queue.push(NativeEvent::AnimationFrame { callback_id });
    }

    // Process any elapsed timers
    let now = native_now_ms();
    let fired: Vec<_> = state.timers
        .iter()
        .filter(|(_, timer)| timer.fire_at_ms <= now)
        .map(|(&id, timer)| (id, timer.callback_id))
        .collect();

    for (timer_id, callback_id) in fired {
        state.timers.remove(&timer_id);
        state.event_queue.push(NativeEvent::Timeout { callback_id });
    }

    // Process clipboard timeouts
    process_clipboard_timeouts(&mut state);

    // Use remove(0) for FIFO order - events should be processed in the order they were queued
    if !state.event_queue.is_empty() {
        let event = state.event_queue.remove(0);
        let data = event.to_event_data();
        if validate_ptr_for_write(out_event, "native_poll_event") {
            unsafe { *out_event = data; }
        }
        data.event_type
    } else {
        if validate_ptr_for_write(out_event, "native_poll_event") {
            unsafe { *out_event = NativeEventData::default(); }
        }
        -1
    }
}

/// Poll for event with timeout (milliseconds)
/// Returns event_type on success, -1 if timeout or no event
#[no_mangle]
pub extern "C" fn native_poll_event_timeout(
    timeout_ms: u64,
    out_event: *mut NativeEventData,
) -> i32 {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(1); // Check every 1ms

    loop {
        // Process any pending timers first
        {
            let mut state = STATE.lock();
            let now = native_now_ms();

            // Fire any elapsed timers
            let fired: Vec<_> = state.timers
                .iter()
                .filter(|(_, timer)| timer.fire_at_ms <= now)
                .map(|(&id, timer)| (id, timer.callback_id))
                .collect();

            for (timer_id, callback_id) in fired {
                state.timers.remove(&timer_id);
                state.event_queue.push(NativeEvent::Timeout { callback_id });
            }
        }

        // Try to get an event
        let result = native_poll_event(out_event);
        if result != -1 {
            return result; // Got an event
        }

        // Check if we've exceeded the timeout
        if Instant::now() >= deadline {
            return -1; // Timeout with no event
        }

        // Sleep briefly before polling again
        std::thread::sleep(poll_interval);
    }
}

/// Process pending timers/animation frames, poll one event, cache it, return event type.
/// Sigil FFI compatible: returns event_type (-1 if no event).
/// Use native_get_event_data() to retrieve the cached event data.
#[no_mangle]
pub extern "C" fn native_poll_events() -> i32 {
    let mut state = STATE.lock();

    // Process animation frames - fire all pending frames immediately
    let frames: Vec<_> = state.animation_frames.drain().collect();
    for (_frame_id, callback_id) in frames {
        state.event_queue.push(NativeEvent::AnimationFrame { callback_id });
    }

    // Process timers - fire any that have elapsed
    let now = native_now_ms();
    let fired: Vec<_> = state.timers
        .iter()
        .filter(|(_, timer)| timer.fire_at_ms <= now)
        .map(|(&id, timer)| (id, timer.callback_id))
        .collect();

    for (timer_id, callback_id) in fired {
        state.timers.remove(&timer_id);
        state.event_queue.push(NativeEvent::Timeout { callback_id });
    }

    // Dequeue one event and cache it for native_get_event_data
    if !state.event_queue.is_empty() {
        let event = state.event_queue.remove(0);
        let data = event.to_event_data();
        let event_type = data.event_type;
        state.last_polled_event = Some(CachedEventData::from(data));
        event_type
    } else {
        state.last_polled_event = None;
        -1
    }
}

/// Get the raw data for the last polled event.
/// Sigil FFI compatible: copies NativeEventData bytes to provided buffer.
/// Returns number of bytes written.
#[no_mangle]
pub extern "C" fn native_get_event_data(out_data: *mut u8, max_len: usize) -> usize {
    let state = STATE.lock();

    if let Some(cached) = state.last_polled_event {
        // Convert cached data back to NativeEventData for FFI
        let event_data = cached.to_native_event_data();
        let data_size = std::mem::size_of::<NativeEventData>();
        let copy_size = data_size.min(max_len);

        if !out_data.is_null() && copy_size > 0 {
            unsafe {
                let src = &event_data as *const NativeEventData as *const u8;
                std::ptr::copy_nonoverlapping(src, out_data, copy_size);
            }
        }
        copy_size
    } else {
        0
    }
}

// =============================================================================
// GPU Initialization and Rendering (Non-Test Only)
// =============================================================================

/// Initialize GPU resources for a window
#[cfg(not(test))]
fn initialize_gpu(
    window: Arc<winit::window::Window>,
    width: u32,
    height: u32,
) -> Result<GpuState, String> {
    use wgpu::util::DeviceExt;

    // Create wgpu instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Create surface from window
    let surface = instance.create_surface(window)
        .map_err(|e| format!("Failed to create surface: {}", e))?;

    // Request adapter
    let adapter = pollster::block_on(instance.request_adapter(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        },
    )).ok_or("Failed to find suitable GPU adapter")?;

    // Request device and queue
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: Some("Qliphoth GPU Device"),
            memory_hints: Default::default(),
        },
        None,
    )).map_err(|e| format!("Failed to create device: {}", e))?;

    // Configure surface
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats.iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // Create shader module
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Rectangle Shader"),
        source: wgpu::ShaderSource::Wgsl(RECT_SHADER.into()),
    });

    // Create uniform buffer
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Uniform Buffer"),
        contents: bytemuck::cast_slice(&[Uniforms {
            viewport_size: [width as f32, height as f32],
            _padding: [0.0, 0.0],
        }]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    // Create bind group layout
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Uniform Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    // Create bind group
    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Uniform Bind Group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    // Create pipeline layout
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    // Create render pipeline
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[
                // Vertex buffer layout
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                    ],
                },
                // Instance buffer layout
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<RectInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        // rect (x, y, w, h)
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        // color
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        // border_radius
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Float32,
                        },
                        // opacity
                        wgpu::VertexAttribute {
                            offset: 36,
                            shader_location: 5,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                },
            ],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });

    // Create vertex buffer (unit quad)
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(QUAD_VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Create index buffer
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(QUAD_INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });

    // Create instance buffer (sized for max_instances rectangles)
    let max_instances = 10000;
    let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Instance Buffer"),
        size: (max_instances * std::mem::size_of::<RectInstance>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    Ok(GpuState {
        surface,
        device,
        queue,
        config,
        render_pipeline,
        vertex_buffer,
        index_buffer,
        instance_buffer,
        uniform_buffer,
        uniform_bind_group,
        max_instances,
    })
}

/// Collect GPU render instances from element tree
#[cfg(not(test))]
fn collect_gpu_instances(
    state: &AppState,
    handle: usize,
    parent_x: f32,
    parent_y: f32,
    instances: &mut Vec<RectInstance>,
) {
    let element = match state.elements.get(&handle) {
        Some(e) => e,
        None => return,
    };

    let layout = match state.get_layout(handle) {
        Some(l) => l,
        None => return,
    };

    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;

    // Add instance for this element if it has a background color
    if let Some(color) = &element.styles.background_color {
        instances.push(RectInstance {
            rect: [abs_x, abs_y, layout.size.width, layout.size.height],
            color: [color.r, color.g, color.b, color.a],
            border_radius: element.styles.border_radius,
            opacity: element.styles.opacity,
            _padding: [0.0, 0.0],
        });
    }

    // Recurse into children
    let children = element.children.clone();
    for child in children {
        collect_gpu_instances(state, child, abs_x, abs_y, instances);
    }
}

/// Non-test versions of hit testing (needed for event loop)
#[cfg(not(test))]
fn hit_test_runtime(state: &AppState, window: usize, x: f32, y: f32) -> Option<usize> {
    let root = state.windows.get(&window)?.root_element?;
    hit_test_element_runtime(state, root, x, y, 0.0, 0.0)
}

#[cfg(not(test))]
fn hit_test_element_runtime(
    state: &AppState,
    handle: usize,
    x: f32, y: f32,
    parent_x: f32, parent_y: f32,
) -> Option<usize> {
    let element = state.elements.get(&handle)?;
    let layout = state.get_layout(handle)?;

    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;

    if x >= abs_x && x < abs_x + layout.size.width &&
       y >= abs_y && y < abs_y + layout.size.height {
        for &child in element.children.iter().rev() {
            if let Some(hit) = hit_test_element_runtime(state, child, x, y, abs_x, abs_y) {
                return Some(hit);
            }
        }
        Some(handle)
    } else {
        None
    }
}

#[cfg(not(test))]
fn collect_callbacks_runtime(
    state: &AppState,
    target: Option<usize>,
    event_type: i32,
) -> Vec<u64> {
    let mut callbacks = Vec::new();
    let mut current = target;

    while let Some(handle) = current {
        for (&callback_id, &(elem, evt)) in &state.callbacks {
            if elem == handle && evt == event_type {
                callbacks.push(callback_id);
            }
        }
        current = state.elements.get(&handle).and_then(|e| e.parent);
    }

    callbacks
}

#[no_mangle]
pub extern "C" fn native_run_event_loop() {
    // In test mode, this is a no-op (tests use software rendering)
    #[cfg(test)]
    {
        log::debug!("native_run_event_loop: no-op in test mode");
        return;
    }

    // In production mode, run the actual GPU event loop
    #[cfg(not(test))]
    {
        run_gpu_event_loop();
    }
}

/// Run the GPU-accelerated event loop (production only)
#[cfg(not(test))]
fn run_gpu_event_loop() {
    use winit::application::ApplicationHandler;
    use winit::event::{ElementState, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
    use winit::window::WindowId;

    struct App {
        windows: HashMap<WindowId, usize>, // winit ID -> our handle
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            // Initialize all pending windows
            let mut state = STATE.lock();
            let handles: Vec<usize> = state.windows.keys().copied().collect();

            for handle in handles {
                let win_state = match state.windows.get(&handle) {
                    Some(w) => w,
                    None => continue,
                };

                // Skip if already has a winit window
                if win_state.winit_window.is_some() {
                    continue;
                }

                let width = win_state.width;
                let height = win_state.height;

                // Create winit window
                let window_attrs = winit::window::WindowAttributes::default()
                    .with_title("Qliphoth Application")
                    .with_inner_size(winit::dpi::PhysicalSize::new(width, height));

                match event_loop.create_window(window_attrs) {
                    Ok(window) => {
                        let window = Arc::new(window);
                        let window_id = window.id();

                        // Initialize GPU
                        match initialize_gpu(window.clone(), width, height) {
                            Ok(gpu_state) => {
                                if let Some(win) = state.windows.get_mut(&handle) {
                                    win.gpu_state = Some(gpu_state);
                                    win.winit_window = Some(window);
                                    win.render_mode = RenderMode::Gpu;
                                }
                                self.windows.insert(window_id, handle);
                                log::info!("GPU initialized for window {}", handle);
                            }
                            Err(e) => {
                                log::error!("GPU init failed: {}, using software rendering", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Window creation failed: {}", e);
                    }
                }
            }
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            window_id: WindowId,
            event: WindowEvent,
        ) {
            let handle = match self.windows.get(&window_id) {
                Some(&h) => h,
                None => return,
            };

            match event {
                WindowEvent::CloseRequested => {
                    let mut state = STATE.lock();
                    state.event_queue.push(NativeEvent::Close);
                    event_loop.exit();
                }

                WindowEvent::Resized(size) => {
                    let mut state = STATE.lock();
                    if let Some(win) = state.windows.get_mut(&handle) {
                        win.width = size.width;
                        win.height = size.height;

                        // Resize GPU surface
                        if let Some(ref mut gpu) = win.gpu_state {
                            gpu.config.width = size.width.max(1);
                            gpu.config.height = size.height.max(1);
                            gpu.surface.configure(&gpu.device, &gpu.config);

                            // Update uniform buffer
                            gpu.queue.write_buffer(
                                &gpu.uniform_buffer,
                                0,
                                bytemuck::cast_slice(&[Uniforms {
                                    viewport_size: [size.width as f32, size.height as f32],
                                    _padding: [0.0, 0.0],
                                }]),
                            );
                        }

                        // Resize framebuffer
                        let pixel_count = (size.width * size.height) as usize;
                        win.framebuffer.resize(pixel_count, Pixel::default());
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let mut state = STATE.lock();
                    state.compute_layout(handle);

                    let target = hit_test_runtime(&state, handle, position.x as f32, position.y as f32);
                    let callbacks = collect_callbacks_runtime(&state, target, EVENT_MOUSEMOVE);

                    for callback_id in callbacks {
                        state.event_queue.push(NativeEvent::MouseMove {
                            x: position.x as f32,
                            y: position.y as f32,
                            callback_id,
                        });
                    }
                }

                WindowEvent::MouseInput { state: btn_state, button, .. } => {
                    if btn_state == ElementState::Released {
                        // Get cursor position from window (simplified - would need tracking)
                        let mut state = STATE.lock();
                        // For a complete implementation, we'd track cursor position
                        // For now, queue a click at 0,0 (placeholder)
                        let callbacks = collect_callbacks_runtime(&state, None, EVENT_CLICK);
                        for callback_id in callbacks {
                            let btn = match button {
                                winit::event::MouseButton::Left => MOUSE_LEFT,
                                winit::event::MouseButton::Right => MOUSE_RIGHT,
                                winit::event::MouseButton::Middle => MOUSE_MIDDLE,
                                _ => MOUSE_LEFT,
                            };
                            state.event_queue.push(NativeEvent::Click {
                                x: 0.0,
                                y: 0.0,
                                button: btn,
                                callback_id,
                            });
                        }
                    }
                }

                WindowEvent::RedrawRequested => {
                    // Render the frame
                    // First pass: compute layout and collect instances (immutable borrow)
                    let instances = {
                        let mut state = STATE.lock();
                        state.compute_layout(handle);

                        let win = match state.windows.get(&handle) {
                            Some(w) => w,
                            None => return,
                        };

                        if win.render_mode != RenderMode::Gpu || win.gpu_state.is_none() {
                            return;
                        }

                        let mut instances = Vec::new();
                        if let Some(root) = win.root_element {
                            collect_gpu_instances(&state, root, 0.0, 0.0, &mut instances);
                        }
                        instances
                    };

                    // Second pass: render with GPU (need mutable access for surface)
                    let state = STATE.lock();
                    let win = match state.windows.get(&handle) {
                        Some(w) => w,
                        None => return,
                    };

                    let gpu = match &win.gpu_state {
                        Some(g) => g,
                        None => return,
                    };

                    // Get surface texture
                    let output = match gpu.surface.get_current_texture() {
                        Ok(t) => t,
                        Err(wgpu::SurfaceError::Lost) => {
                            gpu.surface.configure(&gpu.device, &gpu.config);
                            return;
                        }
                        Err(e) => {
                            log::error!("Surface error: {:?}", e);
                            return;
                        }
                    };

                    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

                    // Upload instance data
                    let instance_count = instances.len().min(gpu.max_instances);
                    if instance_count > 0 {
                        gpu.queue.write_buffer(
                            &gpu.instance_buffer,
                            0,
                            bytemuck::cast_slice(&instances[..instance_count]),
                        );
                    }

                    // Create command encoder
                    let mut encoder = gpu.device.create_command_encoder(
                        &wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        }
                    );

                    {
                        let mut render_pass = encoder.begin_render_pass(
                            &wgpu::RenderPassDescriptor {
                                label: Some("Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                            r: 1.0, g: 1.0, b: 1.0, a: 1.0,
                                        }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            }
                        );

                        render_pass.set_pipeline(&gpu.render_pipeline);
                        render_pass.set_bind_group(0, &gpu.uniform_bind_group, &[]);
                        render_pass.set_vertex_buffer(0, gpu.vertex_buffer.slice(..));
                        render_pass.set_vertex_buffer(1, gpu.instance_buffer.slice(..));
                        render_pass.set_index_buffer(gpu.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

                        // Draw all rectangles as instanced quads
                        render_pass.draw_indexed(0..6, 0, 0..instance_count as u32);
                    }

                    // Submit commands
                    gpu.queue.submit(std::iter::once(encoder.finish()));
                    output.present();
                }

                _ => {}
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            // Request redraw for all windows
            let state = STATE.lock();
            for win_state in state.windows.values() {
                if let Some(ref window) = win_state.winit_window {
                    window.request_redraw();
                }
            }
        }
    }

    // Create and run event loop
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App {
        windows: HashMap::new(),
    };

    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop error: {}", e);
    }
}

/// Render a window to its framebuffer
/// Call this after layout changes to update the visual output
#[no_mangle]
pub extern "C" fn native_render(window: usize) {
    let mut state = STATE.lock();

    // Compute layout first
    state.compute_layout(window);

    // Render to framebuffer
    render_to_framebuffer(&mut state, window);
}

#[no_mangle]
pub extern "C" fn native_request_redraw(_handle: usize) {
    // In a real implementation, this would request a redraw from winit
    // For now, we don't queue an event since Redraw was removed from NativeEvent
}

// =============================================================================
// FFI Functions - Timing
// =============================================================================

/// Schedule a callback to fire after delay_ms milliseconds
/// Returns a timer_id that can be used to cancel
#[no_mangle]
pub extern "C" fn native_set_timeout(callback_id: u64, delay_ms: u64) -> u64 {
    let mut state = STATE.lock();
    let timer_id = state.next_timer_id;
    state.next_timer_id += 1;

    let fire_at_ms = native_now_ms() + delay_ms;
    state.timers.insert(timer_id, Timer {
        callback_id,
        fire_at_ms,
    });

    timer_id
}

/// Cancel a pending timeout
#[no_mangle]
pub extern "C" fn native_clear_timeout(timer_id: u64) {
    let mut state = STATE.lock();
    state.timers.remove(&timer_id);
}

/// Request a callback on the next animation frame
/// Returns a frame_id that can be used to cancel
#[no_mangle]
pub extern "C" fn native_request_animation_frame(callback_id: u64) -> u64 {
    let mut state = STATE.lock();
    let frame_id = state.next_timer_id;
    state.next_timer_id += 1;

    state.animation_frames.insert(frame_id, callback_id);

    frame_id
}

/// Cancel a pending animation frame request
#[no_mangle]
pub extern "C" fn native_cancel_animation_frame(frame_id: u64) {
    let mut state = STATE.lock();
    state.animation_frames.remove(&frame_id);
}

#[no_mangle]
pub extern "C" fn native_now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// =============================================================================
// FFI Functions - Clipboard
// =============================================================================

/// Get clipboard API version.
/// Returns: (major << 16) | (minor << 8) | patch
/// Current: 0x000200 (0.2.0) - Phase 1 complete
#[no_mangle]
pub extern "C" fn native_clipboard_api_version() -> u32 {
    0x000200 // Version 0.2.0
}

/// Query clipboard capabilities for the current platform.
/// Returns: Bitfield of CLIPBOARD_CAP_* flags
#[no_mangle]
pub extern "C" fn native_clipboard_capabilities() -> u32 {
    let mut caps = CLIPBOARD_CAP_READ | CLIPBOARD_CAP_WRITE;

    // Primary selection support on Linux
    #[cfg(target_os = "linux")]
    {
        caps |= CLIPBOARD_CAP_PRIMARY;
    }

    caps
}

/// Request available formats from clipboard.
/// Phase 1: Returns only "text/plain" if clipboard has text.
/// Triggers EVENT_CLIPBOARD_FORMATS_AVAILABLE or EVENT_CLIPBOARD_ERROR.
#[no_mangle]
pub extern "C" fn native_clipboard_get_formats(target: i32, callback_id: u64) -> i32 {
    let mut state = STATE.lock();
    let target_enum = ClipboardTarget::from(target);

    // Log warning for Primary selection (not yet supported)
    if target_enum == ClipboardTarget::PrimarySelection {
        log::warn!("Primary selection not yet supported, using clipboard");
    }

    // Ensure clipboard is initialized
    if state.clipboard.clipboard.is_none() {
        match arboard::Clipboard::new() {
            Ok(clip) => state.clipboard.clipboard = Some(clip),
            Err(_) => {
                state.event_queue.push(NativeEvent::ClipboardError {
                    callback_id,
                    error_code: CLIPBOARD_ERR_UNAVAILABLE,
                });
                return 0;
            }
        }
    }

    // Check if text is available
    let clipboard = state.clipboard.clipboard.as_mut().unwrap();
    let formats = match clipboard.get_text() {
        Ok(_) => vec!["text/plain".to_string()],
        Err(arboard::Error::ContentNotAvailable) => vec![],
        Err(_) => {
            state.event_queue.push(NativeEvent::ClipboardError {
                callback_id,
                error_code: CLIPBOARD_ERR_INTERNAL,
            });
            return 0;
        }
    };

    let format_count = formats.len();

    // Warn if callback_id is already in use (caller error)
    if state.clipboard.completed.contains_key(&callback_id) {
        log::warn!("Callback ID {} already in use, overwriting", callback_id);
    }

    // Store completed data
    state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
        data: Vec::new(),
        formats: Some(formats),
        format_cstrings: Vec::new(),
        completed_at: std::time::Instant::now(),
    });

    // Queue success event
    state.event_queue.push(NativeEvent::ClipboardFormatsAvailable {
        callback_id,
        format_count,
    });

    1
}

/// Get the format list after EVENT_CLIPBOARD_FORMATS_AVAILABLE.
/// Returns: Number of formats written.
/// Pointers are valid until native_clipboard_release(callback_id) is called.
#[no_mangle]
pub extern "C" fn native_clipboard_get_formats_data(
    callback_id: u64,
    out_formats: *mut *const u8,
    max_formats: usize,
) -> usize {
    if out_formats.is_null() || max_formats == 0 {
        return 0;
    }

    let mut state = STATE.lock();

    let completed = match state.clipboard.completed.get_mut(&callback_id) {
        Some(c) => c,
        None => return 0,
    };

    let formats = match &completed.formats {
        Some(f) => f.clone(),
        None => return 0,
    };

    // Build CStrings and store in per-callback storage (valid until release)
    completed.format_cstrings.clear();
    let count = formats.len().min(max_formats);
    for i in 0..count {
        if let Ok(cstr) = std::ffi::CString::new(formats[i].as_str()) {
            completed.format_cstrings.push(cstr);
        }
    }

    // Write pointers to output array
    for (i, cstr) in completed.format_cstrings.iter().enumerate() {
        unsafe {
            *out_formats.add(i) = cstr.as_ptr() as *const u8;
        }
    }

    completed.format_cstrings.len()
}

/// Request clipboard data in specific format.
/// Triggers EVENT_CLIPBOARD_DATA_READY or EVENT_CLIPBOARD_ERROR.
#[no_mangle]
pub extern "C" fn native_clipboard_read_format(
    target: i32,
    mime_type: *const u8,
    callback_id: u64,
) -> i32 {
    if mime_type.is_null() {
        return 0;
    }

    let mime = c_str_to_string(mime_type as *const c_char);
    let mut state = STATE.lock();
    let target_enum = ClipboardTarget::from(target);

    // Log warning for Primary selection (not yet supported)
    if target_enum == ClipboardTarget::PrimarySelection {
        log::warn!("Primary selection not yet supported, using clipboard");
    }

    // Ensure clipboard is initialized
    if state.clipboard.clipboard.is_none() {
        match arboard::Clipboard::new() {
            Ok(clip) => state.clipboard.clipboard = Some(clip),
            Err(_) => {
                state.event_queue.push(NativeEvent::ClipboardError {
                    callback_id,
                    error_code: CLIPBOARD_ERR_UNAVAILABLE,
                });
                return 0;
            }
        }
    }

    // Phase 1: Only support text/plain
    if mime != "text/plain" && mime != "text/plain;charset=utf-8" {
        state.event_queue.push(NativeEvent::ClipboardError {
            callback_id,
            error_code: CLIPBOARD_ERR_FORMAT_NOT_FOUND,
        });
        return 0;
    }

    // Warn if callback_id is already in use (caller error)
    if state.clipboard.completed.contains_key(&callback_id) {
        log::warn!("Callback ID {} already in use, overwriting", callback_id);
    }

    let clipboard = state.clipboard.clipboard.as_mut().unwrap();
    match clipboard.get_text() {
        Ok(text) => {
            let data = text.into_bytes();
            let data_size = data.len();

            state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
                data,
                formats: None,
                format_cstrings: Vec::new(),
                completed_at: std::time::Instant::now(),
            });

            state.event_queue.push(NativeEvent::ClipboardDataReady {
                callback_id,
                data_size,
            });

            1
        }
        Err(arboard::Error::ContentNotAvailable) => {
            state.event_queue.push(NativeEvent::ClipboardError {
                callback_id,
                error_code: CLIPBOARD_ERR_EMPTY,
            });
            0
        }
        Err(_) => {
            state.event_queue.push(NativeEvent::ClipboardError {
                callback_id,
                error_code: CLIPBOARD_ERR_INTERNAL,
            });
            0
        }
    }
}

/// Get the total size of clipboard data after EVENT_CLIPBOARD_DATA_READY.
#[no_mangle]
pub extern "C" fn native_clipboard_get_data_size(callback_id: u64) -> usize {
    let state = STATE.lock();
    state.clipboard.completed
        .get(&callback_id)
        .map(|c| c.data.len())
        .unwrap_or(0)
}

/// Get the data from a completed clipboard read.
/// May be called multiple times; data is not consumed.
#[no_mangle]
pub extern "C" fn native_clipboard_get_data(
    callback_id: u64,
    out_buf: *mut u8,
    max_len: usize,
) -> usize {
    if out_buf.is_null() || max_len == 0 {
        return 0;
    }

    let state = STATE.lock();

    let completed = match state.clipboard.completed.get(&callback_id) {
        Some(c) => c,
        None => return 0,
    };

    let copy_len = completed.data.len().min(max_len);
    if copy_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(
                completed.data.as_ptr(),
                out_buf,
                copy_len,
            );
        }
    }

    copy_len
}

/// Cancel a pending read operation or release completed data.
#[no_mangle]
pub extern "C" fn native_clipboard_cancel(callback_id: u64) {
    let mut state = STATE.lock();

    // Remove from completed if present
    // Phase 1: Operations complete synchronously, so no "pending" state exists
    // Just silently remove - don't fire events for unknown callback_ids
    if state.clipboard.completed.remove(&callback_id).is_none() {
        log::debug!("native_clipboard_cancel: callback_id {} not found", callback_id);
    }
}

/// Release resources associated with a completed clipboard operation.
#[no_mangle]
pub extern "C" fn native_clipboard_release(callback_id: u64) {
    let mut state = STATE.lock();
    state.clipboard.completed.remove(&callback_id);
}

/// Begin a clipboard write operation.
/// Returns: Write handle (non-zero on success, 0 on failure)
#[no_mangle]
pub extern "C" fn native_clipboard_write_begin(target: i32) -> u64 {
    let mut state = STATE.lock();
    let target_enum = ClipboardTarget::from(target);

    // Log warning for Primary selection (not yet supported)
    if target_enum == ClipboardTarget::PrimarySelection {
        log::warn!("Primary selection not yet supported, using clipboard");
    }

    // Handle overflow (return 0 if we would wrap to 0)
    if state.clipboard.next_write_handle == 0 {
        log::error!("Write handle counter overflow");
        return 0;
    }

    let handle = state.clipboard.next_write_handle;
    state.clipboard.next_write_handle = state.clipboard.next_write_handle.wrapping_add(1);

    state.clipboard.write_handles.insert(handle, ClipboardWriteBuilder {
        target: target_enum,
        formats: Vec::new(),
        created_at: std::time::Instant::now(),
    });

    handle
}

/// Add a format to the pending clipboard write.
/// Data is copied; caller may free after this returns.
/// Returns: 1 on success, 0 on failure (invalid handle, null pointer)
#[no_mangle]
pub extern "C" fn native_clipboard_write_add_format(
    write_handle: u64,
    mime_type: *const u8,
    data: *const u8,
    data_len: usize,
) -> i32 {
    if mime_type.is_null() || (data.is_null() && data_len > 0) {
        return 0; // Failure
    }

    let mime = c_str_to_string(mime_type as *const c_char);
    let mut state = STATE.lock();

    let builder = match state.clipboard.write_handles.get_mut(&write_handle) {
        Some(b) => b,
        None => return 0, // Failure - invalid handle
    };

    // Copy data
    let data_vec = if data_len > 0 && !data.is_null() {
        unsafe {
            std::slice::from_raw_parts(data, data_len).to_vec()
        }
    } else {
        Vec::new()
    };

    builder.formats.push((mime, data_vec, false));

    1 // Success
}

/// Add a sensitive format (excluded from clipboard managers/history).
/// Phase 1: Stored but not specially handled.
/// Returns: 1 on success, 0 on failure (invalid handle, null pointer)
#[no_mangle]
pub extern "C" fn native_clipboard_write_add_sensitive(
    write_handle: u64,
    mime_type: *const u8,
    data: *const u8,
    data_len: usize,
) -> i32 {
    if mime_type.is_null() || (data.is_null() && data_len > 0) {
        return 0; // Failure
    }

    let mime = c_str_to_string(mime_type as *const c_char);
    let mut state = STATE.lock();

    let builder = match state.clipboard.write_handles.get_mut(&write_handle) {
        Some(b) => b,
        None => return 0, // Failure - invalid handle
    };

    // Copy data
    let data_vec = if data_len > 0 && !data.is_null() {
        unsafe {
            std::slice::from_raw_parts(data, data_len).to_vec()
        }
    } else {
        Vec::new()
    };

    // Mark as sensitive (Phase 1: stored but not used)
    builder.formats.push((mime, data_vec, true));

    1 // Success
}

/// Commit the clipboard write.
/// Triggers EVENT_CLIPBOARD_WRITE_COMPLETE or EVENT_CLIPBOARD_ERROR.
#[no_mangle]
pub extern "C" fn native_clipboard_write_commit(
    write_handle: u64,
    callback_id: u64,
) -> i32 {
    let mut state = STATE.lock();

    // Take the write builder
    let builder = match state.clipboard.write_handles.remove(&write_handle) {
        Some(b) => b,
        None => {
            state.event_queue.push(NativeEvent::ClipboardError {
                callback_id,
                error_code: CLIPBOARD_ERR_INVALID_HANDLE,
            });
            return 0;
        }
    };

    // Ensure clipboard is initialized
    if state.clipboard.clipboard.is_none() {
        match arboard::Clipboard::new() {
            Ok(clip) => state.clipboard.clipboard = Some(clip),
            Err(_) => {
                state.event_queue.push(NativeEvent::ClipboardError {
                    callback_id,
                    error_code: CLIPBOARD_ERR_UNAVAILABLE,
                });
                return 0;
            }
        }
    }

    let clipboard = state.clipboard.clipboard.as_mut().unwrap();

    // Phase 1: Only write text/plain
    let text_data = builder.formats.iter()
        .find(|(mime, _, _)| mime == "text/plain" || mime == "text/plain;charset=utf-8")
        .map(|(_, data, _)| data);

    match text_data {
        Some(data) => {
            match String::from_utf8(data.clone()) {
                Ok(text) => {
                    match clipboard.set_text(&text) {
                        Ok(()) => {
                            state.event_queue.push(NativeEvent::ClipboardWriteComplete {
                                callback_id,
                            });
                            1
                        }
                        Err(_) => {
                            state.event_queue.push(NativeEvent::ClipboardError {
                                callback_id,
                                error_code: CLIPBOARD_ERR_INTERNAL,
                            });
                            0
                        }
                    }
                }
                Err(_) => {
                    state.event_queue.push(NativeEvent::ClipboardError {
                        callback_id,
                        error_code: CLIPBOARD_ERR_INTERNAL,
                    });
                    0
                }
            }
        }
        None => {
            // No text/plain format provided
            state.event_queue.push(NativeEvent::ClipboardError {
                callback_id,
                error_code: CLIPBOARD_ERR_FORMAT_NOT_FOUND,
            });
            0
        }
    }
}

/// Cancel a pending clipboard write.
#[no_mangle]
pub extern "C" fn native_clipboard_write_cancel(write_handle: u64) {
    let mut state = STATE.lock();
    state.clipboard.write_handles.remove(&write_handle);
}

// -----------------------------------------------------------------------------
// Deprecated Clipboard API (backward compatibility)
// -----------------------------------------------------------------------------

/// DEPRECATED: Use native_clipboard_read_format instead.
/// Synchronous read, blocks thread, text/plain only.
#[no_mangle]
pub extern "C" fn native_clipboard_read(out_buf: *mut c_char, max_len: usize) -> usize {
    if out_buf.is_null() || max_len == 0 {
        return 0;
    }

    let mut state = STATE.lock();

    // Ensure clipboard is initialized
    if state.clipboard.clipboard.is_none() {
        match arboard::Clipboard::new() {
            Ok(clip) => state.clipboard.clipboard = Some(clip),
            Err(_) => return 0,
        }
    }

    let clipboard = state.clipboard.clipboard.as_mut().unwrap();

    match clipboard.get_text() {
        Ok(text) => {
            let bytes = text.as_bytes();
            let copy_len = bytes.len().min(max_len.saturating_sub(1));

            unsafe {
                std::ptr::copy_nonoverlapping(
                    bytes.as_ptr() as *const c_char,
                    out_buf,
                    copy_len,
                );
                *out_buf.add(copy_len) = 0; // Null terminate
            }

            copy_len
        }
        Err(_) => 0,
    }
}

/// DEPRECATED: Use native_clipboard_write_* instead.
/// Synchronous write, blocks thread, text/plain only.
#[no_mangle]
pub extern "C" fn native_clipboard_write(content: *const c_char) {
    if content.is_null() {
        return;
    }

    let text = c_str_to_string(content);
    let mut state = STATE.lock();

    // Ensure clipboard is initialized
    if state.clipboard.clipboard.is_none() {
        match arboard::Clipboard::new() {
            Ok(clip) => state.clipboard.clipboard = Some(clip),
            Err(e) => {
                log::error!("Failed to initialize clipboard: {:?}", e);
                return;
            }
        }
    }

    let clipboard = state.clipboard.clipboard.as_mut().unwrap();
    if let Err(e) = clipboard.set_text(&text) {
        log::error!("Failed to write to clipboard: {:?}", e);
    }
}

// =============================================================================
// FFI Functions - Scroll (Phase 4)
// =============================================================================

/// Set the scroll offset for an element
#[no_mangle]
pub extern "C" fn native_set_scroll_offset(element: usize, x: f32, y: f32) {
    let mut state = STATE.lock();
    if let Some(elem) = state.elements.get_mut(&element) {
        elem.styles.scroll_offset_x = x;
        elem.styles.scroll_offset_y = y;
    }
}

/// Get the scroll offset for an element
#[no_mangle]
pub extern "C" fn native_get_scroll_offset(element: usize, out_x: *mut f32, out_y: *mut f32) {
    if !validate_ptr_for_write(out_x, "native_get_scroll_offset:out_x")
        || !validate_ptr_for_write(out_y, "native_get_scroll_offset:out_y") {
        return;
    }

    let state = STATE.lock();
    if let Some(elem) = state.elements.get(&element) {
        unsafe {
            *out_x = elem.styles.scroll_offset_x;
            *out_y = elem.styles.scroll_offset_y;
        }
    } else {
        unsafe {
            *out_x = 0.0;
            *out_y = 0.0;
        }
    }
}

/// Get the content size of an element (for scroll bounds calculation)
#[no_mangle]
pub extern "C" fn native_get_content_size(element: usize, out_width: *mut f32, out_height: *mut f32) {
    if !validate_ptr_for_write(out_width, "native_get_content_size:out_width")
        || !validate_ptr_for_write(out_height, "native_get_content_size:out_height") {
        return;
    }

    let state = STATE.lock();
    // Calculate total content size by measuring children bounds
    let (width, height) = if let Some(elem) = state.elements.get(&element) {
        let mut max_right: f32 = 0.0;
        let mut max_bottom: f32 = 0.0;

        for &child in &elem.children {
            if let Some(layout) = state.get_layout(child) {
                max_right = max_right.max(layout.location.x + layout.size.width);
                max_bottom = max_bottom.max(layout.location.y + layout.size.height);
            }
        }

        (max_right, max_bottom)
    } else {
        (0.0, 0.0)
    };

    unsafe {
        *out_width = width;
        *out_height = height;
    }
}

// =============================================================================
// FFI Functions - Test Infrastructure
// =============================================================================
// These functions are for testing only. They are compiled out in production builds.

/// Simulate a mouse click at the given window coordinates
#[cfg(test)]
#[no_mangle]
pub extern "C" fn native_simulate_click(window: usize, x: f32, y: f32) {
    let mut state = STATE.lock();

    // Compute layout first to ensure hit testing works
    state.compute_layout(window);

    // Hit test to find the target element
    let target = hit_test(&state, window, x, y);

    // Find all callbacks for click events on target and ancestors (bubbling)
    let callbacks = collect_callbacks_for_event(&state, target, EVENT_CLICK);

    // Queue events for each callback (bubbling order: target first, then ancestors)
    for callback_id in callbacks {
        state.event_queue.push(NativeEvent::Click {
            x, y,
            button: MOUSE_LEFT,
            callback_id,
        });
    }
}

/// Simulate a key press
#[cfg(test)]
#[no_mangle]
pub extern "C" fn native_simulate_key(window: usize, key: i32, modifiers: i32) {
    let mut state = STATE.lock();

    // Find focused element or root
    let target = state.windows.get(&window)
        .and_then(|w| w.focused_element.or(w.root_element))
        .unwrap_or(0);

    // Find callbacks for keydown on target
    let callbacks = collect_callbacks_for_event(&state, Some(target), EVENT_KEYDOWN);

    for callback_id in callbacks {
        state.event_queue.push(NativeEvent::KeyDown {
            key,
            modifiers,
            callback_id,
        });
    }
}

/// Simulate text input
#[cfg(test)]
#[no_mangle]
pub extern "C" fn native_simulate_text_input(window: usize, text: *const c_char) {
    let text = c_str_to_string(text);
    let mut state = STATE.lock();

    // Find focused element
    let target = state.windows.get(&window)
        .and_then(|w| w.focused_element)
        .unwrap_or(0);

    let callbacks = collect_callbacks_for_event(&state, Some(target), EVENT_TEXTINPUT);

    for callback_id in callbacks {
        state.event_queue.push(NativeEvent::TextInput {
            text: text.clone(),
            callback_id,
        });
    }
}

/// Simulate mouse movement
#[cfg(test)]
#[no_mangle]
pub extern "C" fn native_simulate_mouse_move(window: usize, x: f32, y: f32) {
    let mut state = STATE.lock();

    state.compute_layout(window);
    let target = hit_test(&state, window, x, y);
    let callbacks = collect_callbacks_for_event(&state, target, EVENT_MOUSEMOVE);

    for callback_id in callbacks {
        state.event_queue.push(NativeEvent::MouseMove {
            x, y,
            callback_id,
        });
    }
}

/// Simulate scroll event
#[cfg(test)]
#[no_mangle]
pub extern "C" fn native_simulate_scroll(window: usize, delta_x: f32, delta_y: f32) {
    let mut state = STATE.lock();

    // Get root element for scroll
    let target = state.windows.get(&window)
        .and_then(|w| w.root_element)
        .unwrap_or(0);

    let callbacks = collect_callbacks_for_event(&state, Some(target), EVENT_SCROLL);

    for callback_id in callbacks {
        state.event_queue.push(NativeEvent::Scroll {
            delta_x, delta_y,
            callback_id,
        });
    }
}

/// Sample a pixel from the rendered output
#[cfg(test)]
#[no_mangle]
pub extern "C" fn native_sample_pixel(
    window: usize,
    x: i32,
    y: i32,
    out_pixel: *mut Pixel,
) {
    // Validate output pointer first
    if !validate_ptr_for_write(out_pixel, "native_sample_pixel") {
        return;
    }

    let state = STATE.lock();

    if let Some(win) = state.windows.get(&window) {
        if x >= 0 && y >= 0 && (x as u32) < win.width && (y as u32) < win.height {
            let idx = (y as u32 * win.width + x as u32) as usize;
            if idx < win.framebuffer.len() {
                unsafe { *out_pixel = win.framebuffer[idx]; }
                return;
            }
        }
    }

    // Out of bounds or no window - return transparent black
    unsafe { *out_pixel = Pixel { r: 0, g: 0, b: 0, a: 0 }; }
}

/// Check if window has pixels matching a color range
#[cfg(test)]
#[no_mangle]
pub extern "C" fn native_has_pixels_matching(
    window: usize,
    r_min: u8, r_max: u8,
    g_min: u8, g_max: u8,
    b_min: u8, b_max: u8,
) -> i32 {
    let state = STATE.lock();

    if let Some(win) = state.windows.get(&window) {
        for pixel in &win.framebuffer {
            if pixel.r >= r_min && pixel.r <= r_max &&
               pixel.g >= g_min && pixel.g <= g_max &&
               pixel.b >= b_min && pixel.b <= b_max {
                return 1; // Found a match
            }
        }
    }

    0 // No match
}

/// Render the window to its framebuffer (software renderer)
fn render_to_framebuffer(state: &mut AppState, window: usize) {
    // Extract window info first
    let (width, height, root) = {
        let win = match state.windows.get(&window) {
            Some(w) => w,
            None => return,
        };
        (win.width, win.height, win.root_element)
    };

    let root = match root {
        Some(r) => r,
        None => {
            // No root - just clear to white
            if let Some(win) = state.windows.get_mut(&window) {
                for pixel in &mut win.framebuffer {
                    *pixel = Pixel { r: 255, g: 255, b: 255, a: 255 };
                }
            }
            return;
        }
    };

    // Collect render commands (reads from elements)
    let mut render_commands = RenderCommands {
        rects: Vec::new(),
        texts: Vec::new(),
    };
    collect_render_commands(state, root, 0.0, 0.0, &mut render_commands);

    // Sort by z-index (stable sort preserves document order for equal z-index)
    render_commands.sort_by_z_index();

    // Render text glyphs (needs mutable text_system)
    let mut text_glyphs: Vec<(f32, f32, Vec<TextGlyph>)> = Vec::new();
    for text_cmd in &render_commands.texts {
        let glyphs = state.text_system.render_text(
            &text_cmd.text,
            text_cmd.font_size,
            text_cmd.color,
            text_cmd.max_width,
        );
        text_glyphs.push((text_cmd.x, text_cmd.y, glyphs));
    }

    // Now render to framebuffer
    let win = match state.windows.get_mut(&window) {
        Some(w) => w,
        None => return,
    };

    // Clear framebuffer to white background
    for pixel in &mut win.framebuffer {
        *pixel = Pixel { r: 255, g: 255, b: 255, a: 255 };
    }

    // Draw all rectangle commands
    for cmd in &render_commands.rects {
        draw_rect_to_framebuffer(
            &mut win.framebuffer,
            width, height,
            cmd.x as i32, cmd.y as i32,
            cmd.width as i32, cmd.height as i32,
            cmd.color,
        );
    }

    // Draw all text glyphs
    for (base_x, base_y, glyphs) in text_glyphs {
        for glyph in glyphs {
            draw_glyph_to_framebuffer(
                &mut win.framebuffer,
                width, height,
                base_x as i32 + glyph.x + glyph.left,
                base_y as i32 + glyph.y - glyph.top,
                &glyph,
            );
        }
    }
}

/// Command to render a filled rectangle
struct RectRenderCommand {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Pixel,
    z_index: i32,
}

/// Command to render text
struct TextRenderCommand {
    x: f32,
    y: f32,
    max_width: f32,
    text: String,
    font_size: f32,
    color: Color,
    z_index: i32,
}

/// Combined render commands for an element tree
struct RenderCommands {
    rects: Vec<RectRenderCommand>,
    texts: Vec<TextRenderCommand>,
}

impl RenderCommands {
    /// Sort all commands by z-index (stable sort preserves document order)
    fn sort_by_z_index(&mut self) {
        self.rects.sort_by_key(|cmd| cmd.z_index);
        self.texts.sort_by_key(|cmd| cmd.z_index);
    }
}

fn collect_render_commands(
    state: &AppState,
    handle: usize,
    parent_x: f32,
    parent_y: f32,
    commands: &mut RenderCommands,
) {
    collect_render_commands_with_scroll(state, handle, parent_x, parent_y, 0.0, 0.0, commands);
}

fn collect_render_commands_with_scroll(
    state: &AppState,
    handle: usize,
    parent_x: f32,
    parent_y: f32,
    scroll_x: f32,
    scroll_y: f32,
    commands: &mut RenderCommands,
) {
    let element = match state.elements.get(&handle) {
        Some(e) => e,
        None => return,
    };

    let layout = match state.get_layout(handle) {
        Some(l) => l,
        None => return,
    };

    // Apply scroll offset from parent
    let abs_x = parent_x + layout.location.x - scroll_x;
    let abs_y = parent_y + layout.location.y - scroll_y;

    let z_index = element.styles.z_index;

    // Add rect command for this element if it has a background color
    if let Some(color) = &element.styles.background_color {
        commands.rects.push(RectRenderCommand {
            x: abs_x,
            y: abs_y,
            width: layout.size.width,
            height: layout.size.height,
            color: Pixel {
                r: (color.r * 255.0) as u8,
                g: (color.g * 255.0) as u8,
                b: (color.b * 255.0) as u8,
                a: (color.a * 255.0) as u8,
            },
            z_index,
        });
    }

    // Add text command if this element has text content
    if let Some(text) = &element.text_content {
        if !text.is_empty() {
            let text_color = element.styles.color.unwrap_or(Color::default());
            // Extract padding values using pattern matching
            let pad_left = match element.styles.padding.left {
                taffy::LengthPercentage::Length(v) => v,
                taffy::LengthPercentage::Percent(p) => p * layout.size.width,
            };
            let pad_top = match element.styles.padding.top {
                taffy::LengthPercentage::Length(v) => v,
                taffy::LengthPercentage::Percent(p) => p * layout.size.height,
            };
            commands.texts.push(TextRenderCommand {
                x: abs_x + pad_left,
                y: abs_y + pad_top,
                max_width: layout.size.width,
                text: text.clone(),
                font_size: element.styles.font_size,
                color: text_color,
                z_index,
            });
        }
    }

    // Recurse into children with this element's scroll offset
    let child_scroll_x = element.styles.scroll_offset_x;
    let child_scroll_y = element.styles.scroll_offset_y;
    let children = element.children.clone();
    for child in children {
        collect_render_commands_with_scroll(
            state, child,
            abs_x, abs_y,
            child_scroll_x, child_scroll_y,
            commands
        );
    }
}

fn draw_rect_to_framebuffer(
    framebuffer: &mut [Pixel],
    fb_width: u32,
    fb_height: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: Pixel,
) {
    let x_start = x.max(0) as u32;
    let y_start = y.max(0) as u32;
    let x_end = ((x + width) as u32).min(fb_width);
    let y_end = ((y + height) as u32).min(fb_height);

    for py in y_start..y_end {
        for px in x_start..x_end {
            let idx = (py * fb_width + px) as usize;
            if idx < framebuffer.len() {
                // Simple alpha blending
                if color.a == 255 {
                    framebuffer[idx] = color;
                } else if color.a > 0 {
                    let dst = &framebuffer[idx];
                    let alpha = color.a as f32 / 255.0;
                    let inv_alpha = 1.0 - alpha;
                    framebuffer[idx] = Pixel {
                        r: (color.r as f32 * alpha + dst.r as f32 * inv_alpha) as u8,
                        g: (color.g as f32 * alpha + dst.g as f32 * inv_alpha) as u8,
                        b: (color.b as f32 * alpha + dst.b as f32 * inv_alpha) as u8,
                        a: 255,
                    };
                }
            }
        }
    }
}

/// Draw a text glyph to the framebuffer with alpha blending
fn draw_glyph_to_framebuffer(
    framebuffer: &mut [Pixel],
    fb_width: u32,
    fb_height: u32,
    x: i32,
    y: i32,
    glyph: &TextGlyph,
) {
    // Glyph data is typically 8-bit alpha coverage
    for gy in 0..glyph.height {
        for gx in 0..glyph.width {
            let px = x + gx as i32;
            let py = y + gy as i32;

            // Bounds check
            if px < 0 || py < 0 || px >= fb_width as i32 || py >= fb_height as i32 {
                continue;
            }

            let glyph_idx = (gy * glyph.width + gx) as usize;
            if glyph_idx >= glyph.data.len() {
                continue;
            }

            let alpha = glyph.data[glyph_idx] as f32 / 255.0;
            if alpha < 0.01 {
                continue;
            }

            let fb_idx = (py as u32 * fb_width + px as u32) as usize;
            if fb_idx >= framebuffer.len() {
                continue;
            }

            // Alpha blend glyph color with background
            let dst = &framebuffer[fb_idx];
            let inv_alpha = 1.0 - alpha;
            framebuffer[fb_idx] = Pixel {
                r: (glyph.color.r * 255.0 * alpha + dst.r as f32 * inv_alpha) as u8,
                g: (glyph.color.g * 255.0 * alpha + dst.g as f32 * inv_alpha) as u8,
                b: (glyph.color.b * 255.0 * alpha + dst.b as f32 * inv_alpha) as u8,
                a: 255,
            };
        }
    }
}

/// Hit test: find the deepest element at the given coordinates
#[cfg(test)]
fn hit_test(state: &AppState, window: usize, x: f32, y: f32) -> Option<usize> {
    let root = state.windows.get(&window)?.root_element?;
    hit_test_element(state, root, x, y, 0.0, 0.0)
}

#[cfg(test)]
fn hit_test_element(
    state: &AppState,
    handle: usize,
    x: f32, y: f32,
    parent_x: f32, parent_y: f32,
) -> Option<usize> {
    let element = state.elements.get(&handle)?;
    let layout = state.get_layout(handle)?;

    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;

    // Check if point is within this element's bounds
    if x >= abs_x && x < abs_x + layout.size.width &&
       y >= abs_y && y < abs_y + layout.size.height {
        // Check children (in reverse order for proper z-order)
        for &child in element.children.iter().rev() {
            if let Some(hit) = hit_test_element(state, child, x, y, abs_x, abs_y) {
                return Some(hit);
            }
        }
        // No child hit, this element is the target
        Some(handle)
    } else {
        None
    }
}

/// Collect callbacks for an event type, following bubbling order
#[cfg(test)]
fn collect_callbacks_for_event(
    state: &AppState,
    target: Option<usize>,
    event_type: i32,
) -> Vec<u64> {
    let mut callbacks = Vec::new();
    let mut current = target;

    while let Some(handle) = current {
        // Find callbacks registered for this element and event type
        for (&callback_id, &(elem, evt)) in &state.callbacks {
            if elem == handle && evt == event_type {
                callbacks.push(callback_id);
            }
        }

        // Move to parent for bubbling
        current = state.elements.get(&handle).and_then(|e| e.parent);
    }

    callbacks
}

// =============================================================================
// Layout & Rendering (Internal)
// =============================================================================

impl AppState {
    /// Compute layout for a window
    fn compute_layout(&mut self, window_handle: usize) {
        let Some(window) = self.windows.get(&window_handle) else {
            return;
        };
        let Some(root) = window.root_element else {
            return;
        };
        let Some(element) = self.elements.get(&root) else {
            return;
        };
        let Some(root_node) = element.layout_node else {
            return;
        };

        // Compute layout
        let available_space = taffy::Size {
            width: taffy::AvailableSpace::Definite(window.width as f32),
            height: taffy::AvailableSpace::Definite(window.height as f32),
        };

        let _ = self.layout_tree.compute_layout(root_node, available_space);
    }

    /// Get computed layout for an element
    fn get_layout(&self, handle: usize) -> Option<taffy::Layout> {
        let element = self.elements.get(&handle)?;
        let node = element.layout_node?;
        self.layout_tree.layout(node).ok().copied()
    }

    /// Recursively destroy an element and all its children
    /// Removes layout nodes, callbacks, and element data
    fn destroy_element_tree(&mut self, handle: usize) {
        // Get children first (to avoid borrow issues)
        let children: Vec<usize> = self.elements
            .get(&handle)
            .map(|e| e.children.clone())
            .unwrap_or_default();

        // Recursively destroy children
        for child in children {
            self.destroy_element_tree(child);
        }

        // Remove callbacks associated with this element
        self.callbacks.retain(|_, (elem, _)| *elem != handle);

        // Remove layout node from taffy tree
        if let Some(element) = self.elements.get(&handle) {
            if let Some(node) = element.layout_node {
                if let Err(e) = self.layout_tree.remove(node) {
                    log::debug!("destroy_element_tree: taffy remove failed for {}: {:?}", handle, e);
                }
            }
        }

        // Remove the element itself
        self.elements.remove(&handle);
    }

    /// Clean up a window and all its associated resources
    /// Destroys all elements in the window's tree and removes callbacks
    fn cleanup_window(&mut self, window_handle: usize) {
        // Get root element before removing window
        let root = self.windows.get(&window_handle).and_then(|w| w.root_element);

        // Recursively destroy all elements in this window's tree
        if let Some(root) = root {
            self.destroy_element_tree(root);
        }

        // Remove the window itself
        self.windows.remove(&window_handle);

        log::debug!("cleanup_window: destroyed window {} with root {:?}", window_handle, root);
    }
}

// =============================================================================
// Tests - TDD Green Phase
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::CString;

    /// Helper to create a C string for FFI calls
    fn cstr(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    /// Reset global state between tests
    fn reset_state() {
        let mut state = STATE.lock();
        state.elements.clear();
        state.windows.clear();
        state.event_queue.clear();
        state.callbacks.clear();
        state.next_handle = 1;
        // Reset the layout tree to prevent stale node references
        state.layout_tree = TaffyTree::new();
        // Reset timer state
        state.timers.clear();
        state.animation_frames.clear();
        state.next_timer_id = 1;
        // Reset cached event
        state.last_polled_event = None;
        // Reset clipboard state
        state.clipboard.completed.clear();
        state.clipboard.write_handles.clear();
        state.clipboard.next_write_handle = 1;
    }

    // =========================================================================
    // Phase 1: Window Management
    // =========================================================================

    #[test]
    #[serial]
    fn test_create_window_returns_nonzero_handle() {
        reset_state();
        let title = cstr("Test Window");
        let handle = native_create_window(title.as_ptr(), 800, 600);
        assert!(handle > 0, "Window handle should be non-zero");
    }

    #[test]
    #[serial]
    fn test_window_size_matches_requested() {
        reset_state();
        let title = cstr("Test Window");
        let handle = native_create_window(title.as_ptr(), 1024, 768);

        let mut w: c_int = 0;
        let mut h: c_int = 0;
        native_window_size(handle, &mut w, &mut h);

        assert_eq!(w, 1024);
        assert_eq!(h, 768);
    }

    #[test]
    #[serial]
    fn test_destroy_window_invalidates_handle() {
        reset_state();
        let title = cstr("Test Window");
        let handle = native_create_window(title.as_ptr(), 800, 600);

        native_destroy_window(handle);

        let mut w: c_int = 0;
        let mut h: c_int = 0;
        native_window_size(handle, &mut w, &mut h);

        // Invalid handle returns 0,0 per spec
        assert_eq!(w, 0);
        assert_eq!(h, 0);
    }

    // =========================================================================
    // Phase 2: Element Creation
    // =========================================================================

    #[test]
    #[serial]
    fn test_create_element_returns_nonzero_handle() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");
        let elem = native_create_element(win, tag.as_ptr());
        assert!(elem > 0, "Element handle should be non-zero");
    }

    #[test]
    #[serial]
    fn test_create_text_stores_content() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let content = cstr("Hello, World!");
        let elem = native_create_text(win, content.as_ptr());

        let mut buf = [0i8; 64];
        let len = native_get_text_content(elem, buf.as_mut_ptr(), 64);

        assert_eq!(len, 13); // "Hello, World!" is 13 chars
    }

    #[test]
    #[serial]
    fn test_destroy_element_removes_from_state() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");
        let elem = native_create_element(win, tag.as_ptr());

        native_destroy_element(elem);

        // After destruction, get_child_count on destroyed element returns 0
        // (it's no longer in the elements map)
        assert_eq!(native_get_child_count(elem), 0);
    }

    // =========================================================================
    // Phase 3: Element Tree
    // =========================================================================

    #[test]
    #[serial]
    fn test_append_child_increases_count() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");
        let parent = native_create_element(win, tag.as_ptr());
        let child = native_create_element(win, tag.as_ptr());

        assert_eq!(native_get_child_count(parent), 0);
        native_append_child(parent, child);
        assert_eq!(native_get_child_count(parent), 1);
        assert_eq!(native_get_child_at(parent, 0), child);
    }

    #[test]
    #[serial]
    fn test_remove_child_decreases_count() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");
        let parent = native_create_element(win, tag.as_ptr());
        let child = native_create_element(win, tag.as_ptr());

        native_append_child(parent, child);
        assert_eq!(native_get_child_count(parent), 1);

        native_remove_child(parent, child);
        assert_eq!(native_get_child_count(parent), 0);
    }

    #[test]
    #[serial]
    fn test_children_maintain_order() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("span");
        let parent = native_create_element(win, tag.as_ptr());
        let child1 = native_create_element(win, tag.as_ptr());
        let child2 = native_create_element(win, tag.as_ptr());
        let child3 = native_create_element(win, tag.as_ptr());

        native_append_child(parent, child1);
        native_append_child(parent, child2);
        native_append_child(parent, child3);

        assert_eq!(native_get_child_count(parent), 3);
        assert_eq!(native_get_child_at(parent, 0), child1);
        assert_eq!(native_get_child_at(parent, 1), child2);
        assert_eq!(native_get_child_at(parent, 2), child3);
    }

    #[test]
    #[serial]
    fn test_insert_before_correct_position() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("span");
        let parent = native_create_element(win, tag.as_ptr());
        let child1 = native_create_element(win, tag.as_ptr());
        let child2 = native_create_element(win, tag.as_ptr());
        let child3 = native_create_element(win, tag.as_ptr());

        native_append_child(parent, child1);
        native_append_child(parent, child3);
        native_insert_before(parent, child2, child3);

        assert_eq!(native_get_child_count(parent), 3);
        assert_eq!(native_get_child_at(parent, 0), child1);
        assert_eq!(native_get_child_at(parent, 1), child2);
        assert_eq!(native_get_child_at(parent, 2), child3);
    }

    // =========================================================================
    // Phase 4: Flexbox Layout
    // =========================================================================

    #[test]
    #[serial]
    fn test_flex_row_layout() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        let prop_display = cstr("display");
        let val_flex = cstr("flex");
        let prop_dir = cstr("flex-direction");
        let val_row = cstr("row");
        let prop_width = cstr("width");
        let val_300 = cstr("300px");
        let prop_height = cstr("height");
        let val_100 = cstr("100px");
        let val_50 = cstr("50px");

        native_set_style(parent, prop_display.as_ptr(), val_flex.as_ptr());
        native_set_style(parent, prop_dir.as_ptr(), val_row.as_ptr());
        native_set_style(parent, prop_width.as_ptr(), val_300.as_ptr());
        native_set_style(parent, prop_height.as_ptr(), val_100.as_ptr());

        let child1 = native_create_element(win, tag.as_ptr());
        native_set_style(child1, prop_width.as_ptr(), val_50.as_ptr());
        native_set_style(child1, prop_height.as_ptr(), val_50.as_ptr());

        let child2 = native_create_element(win, tag.as_ptr());
        native_set_style(child2, prop_width.as_ptr(), val_50.as_ptr());
        native_set_style(child2, prop_height.as_ptr(), val_50.as_ptr());

        native_append_child(parent, child1);
        native_append_child(parent, child2);
        native_set_root(win, parent);
        native_compute_layout(win);

        let mut layout1 = Layout::default();
        let mut layout2 = Layout::default();
        native_get_layout(child1, &mut layout1);
        native_get_layout(child2, &mut layout2);

        // In row layout, children should be side by side
        assert_eq!(layout1.x, 0.0);
        assert_eq!(layout2.x, 50.0); // Second child after first
        assert_eq!(layout1.width, 50.0);
        assert_eq!(layout2.width, 50.0);
    }

    #[test]
    #[serial]
    fn test_flex_column_layout() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        let prop_display = cstr("display");
        let val_flex = cstr("flex");
        let prop_dir = cstr("flex-direction");
        let val_col = cstr("column");
        let prop_width = cstr("width");
        let val_100 = cstr("100px");
        let prop_height = cstr("height");
        let val_200 = cstr("200px");
        let val_50 = cstr("50px");

        native_set_style(parent, prop_display.as_ptr(), val_flex.as_ptr());
        native_set_style(parent, prop_dir.as_ptr(), val_col.as_ptr());
        native_set_style(parent, prop_width.as_ptr(), val_100.as_ptr());
        native_set_style(parent, prop_height.as_ptr(), val_200.as_ptr());

        let child1 = native_create_element(win, tag.as_ptr());
        native_set_style(child1, prop_width.as_ptr(), val_50.as_ptr());
        native_set_style(child1, prop_height.as_ptr(), val_50.as_ptr());

        let child2 = native_create_element(win, tag.as_ptr());
        native_set_style(child2, prop_width.as_ptr(), val_50.as_ptr());
        native_set_style(child2, prop_height.as_ptr(), val_50.as_ptr());

        native_append_child(parent, child1);
        native_append_child(parent, child2);
        native_set_root(win, parent);
        native_compute_layout(win);

        let mut layout1 = Layout::default();
        let mut layout2 = Layout::default();
        native_get_layout(child1, &mut layout1);
        native_get_layout(child2, &mut layout2);

        // In column layout, children should be stacked vertically
        assert_eq!(layout1.y, 0.0);
        assert_eq!(layout2.y, 50.0); // Second child below first
    }

    #[test]
    #[serial]
    fn test_gap_adds_spacing() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        native_set_style(parent, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(parent, cstr("flex-direction").as_ptr(), cstr("row").as_ptr());
        native_set_style(parent, cstr("gap").as_ptr(), cstr("20px").as_ptr());
        native_set_style(parent, cstr("width").as_ptr(), cstr("300px").as_ptr());

        let child1 = native_create_element(win, tag.as_ptr());
        native_set_style(child1, cstr("width").as_ptr(), cstr("50px").as_ptr());
        native_set_style(child1, cstr("height").as_ptr(), cstr("50px").as_ptr());

        let child2 = native_create_element(win, tag.as_ptr());
        native_set_style(child2, cstr("width").as_ptr(), cstr("50px").as_ptr());
        native_set_style(child2, cstr("height").as_ptr(), cstr("50px").as_ptr());

        native_append_child(parent, child1);
        native_append_child(parent, child2);
        native_set_root(win, parent);
        native_compute_layout(win);

        let mut layout2 = Layout::default();
        native_get_layout(child2, &mut layout2);

        // Second child should be at 50 + 20 = 70
        assert_eq!(layout2.x, 70.0);
    }

    #[test]
    #[serial]
    fn test_justify_content_center() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        native_set_style(parent, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(parent, cstr("flex-direction").as_ptr(), cstr("row").as_ptr());
        native_set_style(parent, cstr("justify-content").as_ptr(), cstr("center").as_ptr());
        native_set_style(parent, cstr("width").as_ptr(), cstr("300px").as_ptr());
        native_set_style(parent, cstr("height").as_ptr(), cstr("100px").as_ptr());

        let child = native_create_element(win, tag.as_ptr());
        native_set_style(child, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(child, cstr("height").as_ptr(), cstr("100px").as_ptr());

        native_append_child(parent, child);
        native_set_root(win, parent);
        native_compute_layout(win);

        let mut layout = Layout::default();
        native_get_layout(child, &mut layout);

        // Child should be centered: (300 - 100) / 2 = 100
        assert_eq!(layout.x, 100.0);
    }

    #[test]
    #[serial]
    fn test_justify_content_space_between() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        native_set_style(parent, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(parent, cstr("flex-direction").as_ptr(), cstr("row").as_ptr());
        native_set_style(parent, cstr("justify-content").as_ptr(), cstr("space-between").as_ptr());
        native_set_style(parent, cstr("width").as_ptr(), cstr("300px").as_ptr());
        native_set_style(parent, cstr("height").as_ptr(), cstr("100px").as_ptr());

        let child1 = native_create_element(win, tag.as_ptr());
        native_set_style(child1, cstr("width").as_ptr(), cstr("50px").as_ptr());
        native_set_style(child1, cstr("height").as_ptr(), cstr("50px").as_ptr());

        let child2 = native_create_element(win, tag.as_ptr());
        native_set_style(child2, cstr("width").as_ptr(), cstr("50px").as_ptr());
        native_set_style(child2, cstr("height").as_ptr(), cstr("50px").as_ptr());

        native_append_child(parent, child1);
        native_append_child(parent, child2);
        native_set_root(win, parent);
        native_compute_layout(win);

        let mut layout1 = Layout::default();
        let mut layout2 = Layout::default();
        native_get_layout(child1, &mut layout1);
        native_get_layout(child2, &mut layout2);

        // First child at start, second at end
        assert_eq!(layout1.x, 0.0);
        assert_eq!(layout2.x, 250.0); // 300 - 50 = 250
    }

    #[test]
    #[serial]
    fn test_align_items_center() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        native_set_style(parent, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(parent, cstr("flex-direction").as_ptr(), cstr("row").as_ptr());
        native_set_style(parent, cstr("align-items").as_ptr(), cstr("center").as_ptr());
        native_set_style(parent, cstr("width").as_ptr(), cstr("300px").as_ptr());
        native_set_style(parent, cstr("height").as_ptr(), cstr("100px").as_ptr());

        let child = native_create_element(win, tag.as_ptr());
        native_set_style(child, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(child, cstr("height").as_ptr(), cstr("50px").as_ptr());

        native_append_child(parent, child);
        native_set_root(win, parent);
        native_compute_layout(win);

        let mut layout = Layout::default();
        native_get_layout(child, &mut layout);

        // Child should be vertically centered: (100 - 50) / 2 = 25
        assert_eq!(layout.y, 25.0);
    }

    #[test]
    #[serial]
    fn test_padding_offsets_children() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        native_set_style(parent, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(parent, cstr("padding").as_ptr(), cstr("10px").as_ptr());
        native_set_style(parent, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(parent, cstr("height").as_ptr(), cstr("100px").as_ptr());

        let child = native_create_element(win, tag.as_ptr());
        native_set_style(child, cstr("width").as_ptr(), cstr("50px").as_ptr());
        native_set_style(child, cstr("height").as_ptr(), cstr("50px").as_ptr());

        native_append_child(parent, child);
        native_set_root(win, parent);
        native_compute_layout(win);

        let mut layout = Layout::default();
        native_get_layout(child, &mut layout);

        // Child should be offset by padding
        assert_eq!(layout.x, 10.0);
        assert_eq!(layout.y, 10.0);
    }

    #[test]
    #[serial]
    fn test_nested_flex_layout() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");

        // Outer container: row
        let outer = native_create_element(win, tag.as_ptr());
        native_set_style(outer, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(outer, cstr("flex-direction").as_ptr(), cstr("row").as_ptr());
        native_set_style(outer, cstr("width").as_ptr(), cstr("200px").as_ptr());
        native_set_style(outer, cstr("height").as_ptr(), cstr("100px").as_ptr());

        // Inner container: column
        let inner = native_create_element(win, tag.as_ptr());
        native_set_style(inner, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(inner, cstr("flex-direction").as_ptr(), cstr("column").as_ptr());
        native_set_style(inner, cstr("width").as_ptr(), cstr("100px").as_ptr());

        let child1 = native_create_element(win, tag.as_ptr());
        native_set_style(child1, cstr("width").as_ptr(), cstr("50px").as_ptr());
        native_set_style(child1, cstr("height").as_ptr(), cstr("30px").as_ptr());

        let child2 = native_create_element(win, tag.as_ptr());
        native_set_style(child2, cstr("width").as_ptr(), cstr("50px").as_ptr());
        native_set_style(child2, cstr("height").as_ptr(), cstr("30px").as_ptr());

        native_append_child(inner, child1);
        native_append_child(inner, child2);
        native_append_child(outer, inner);
        native_set_root(win, outer);
        native_compute_layout(win);

        let mut layout1 = Layout::default();
        let mut layout2 = Layout::default();
        native_get_layout(child1, &mut layout1);
        native_get_layout(child2, &mut layout2);

        // Children should be stacked vertically within inner
        assert_eq!(layout1.y, 0.0);
        assert_eq!(layout2.y, 30.0); // Second child below first
        assert_eq!(layout1.x, layout2.x); // Same X position
    }

    // =========================================================================
    // Phase 5: Rendering
    // =========================================================================

    #[test]
    #[serial]
    fn test_background_color_renders() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("div");

        let elem = native_create_element(win, tag.as_ptr());
        native_set_style(elem, cstr("width").as_ptr(), cstr("200px").as_ptr());
        native_set_style(elem, cstr("height").as_ptr(), cstr("200px").as_ptr());
        native_set_style(elem, cstr("background-color").as_ptr(), cstr("#ff0000").as_ptr());
        native_set_root(win, elem);

        // Render the window
        native_render(win);

        // Sample pixel at center of the red element (100, 100)
        let mut pixel = Pixel::default();
        native_sample_pixel(win, 100, 100, &mut pixel);

        // Should be red (255, 0, 0)
        assert!(pixel.r > 200, "Red channel should be high, got {}", pixel.r);
        assert!(pixel.g < 50, "Green channel should be low, got {}", pixel.g);
        assert!(pixel.b < 50, "Blue channel should be low, got {}", pixel.b);
    }

    #[test]
    #[serial]
    fn test_pixel_sampling_outside_element() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("div");

        let elem = native_create_element(win, tag.as_ptr());
        native_set_style(elem, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(elem, cstr("height").as_ptr(), cstr("100px").as_ptr());
        native_set_style(elem, cstr("background-color").as_ptr(), cstr("#0000ff").as_ptr());
        native_set_root(win, elem);

        // Render the window
        native_render(win);

        // Sample pixel outside the blue element (should be white background)
        let mut pixel = Pixel::default();
        native_sample_pixel(win, 200, 200, &mut pixel);

        // Should be white (255, 255, 255) - the default background
        assert!(pixel.r > 200, "Should be white background (R)");
        assert!(pixel.g > 200, "Should be white background (G)");
        assert!(pixel.b > 200, "Should be white background (B)");
    }

    #[test]
    #[serial]
    fn test_has_pixels_matching_finds_color() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("div");

        let elem = native_create_element(win, tag.as_ptr());
        native_set_style(elem, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(elem, cstr("height").as_ptr(), cstr("100px").as_ptr());
        native_set_style(elem, cstr("background-color").as_ptr(), cstr("#00ff00").as_ptr());
        native_set_root(win, elem);

        // Render the window
        native_render(win);

        // Should find green pixels
        let found = native_has_pixels_matching(win, 0, 50, 200, 255, 0, 50);
        assert_eq!(found, 1, "Should find green pixels");

        // Should not find blue pixels (no pure blue in window)
        let not_found = native_has_pixels_matching(win, 0, 50, 0, 50, 200, 255);
        assert_eq!(not_found, 0, "Should not find blue pixels");
    }

    #[test]
    #[serial]
    fn test_nested_elements_render() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("div");

        // Parent with blue background
        let parent = native_create_element(win, tag.as_ptr());
        native_set_style(parent, cstr("width").as_ptr(), cstr("200px").as_ptr());
        native_set_style(parent, cstr("height").as_ptr(), cstr("200px").as_ptr());
        native_set_style(parent, cstr("background-color").as_ptr(), cstr("#0000ff").as_ptr());

        // Child with red background positioned inside parent
        let child = native_create_element(win, tag.as_ptr());
        native_set_style(child, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(child, cstr("height").as_ptr(), cstr("100px").as_ptr());
        native_set_style(child, cstr("background-color").as_ptr(), cstr("#ff0000").as_ptr());

        native_append_child(parent, child);
        native_set_root(win, parent);

        // Render the window
        native_render(win);

        // Sample inside child (should be red)
        let mut pixel_child = Pixel::default();
        native_sample_pixel(win, 50, 50, &mut pixel_child);
        assert!(pixel_child.r > 200, "Child area should be red");
        assert!(pixel_child.b < 50, "Child area should not be blue");

        // Sample outside child but inside parent (should be blue)
        let mut pixel_parent = Pixel::default();
        native_sample_pixel(win, 150, 150, &mut pixel_parent);
        assert!(pixel_parent.b > 200, "Parent area should be blue");
        assert!(pixel_parent.r < 50, "Parent area should not be red");
    }

    // =========================================================================
    // Phase 6: Events
    // =========================================================================

    #[test]
    #[serial]
    fn test_click_event_dispatched() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("div");

        let elem = native_create_element(win, tag.as_ptr());
        native_set_style(elem, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(elem, cstr("height").as_ptr(), cstr("100px").as_ptr());
        native_set_root(win, elem);

        let callback_id = 42u64;
        native_add_event_listener(elem, EVENT_CLICK, callback_id);

        native_simulate_click(win, 50.0, 50.0);

        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        assert_eq!(result, EVENT_CLICK);
        assert_eq!(event.event_type, EVENT_CLICK);
        assert_eq!(event.callback_id, callback_id);
    }

    #[test]
    #[serial]
    fn test_focus_event_dispatched() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("input");

        let elem = native_create_element(win, tag.as_ptr());
        native_set_root(win, elem);

        let callback_id = 50u64;
        native_add_event_listener(elem, EVENT_FOCUS, callback_id);

        native_focus(elem);

        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        assert_eq!(result, EVENT_FOCUS);
        assert_eq!(event.callback_id, callback_id);
        assert_eq!(native_get_focused(win), elem);
    }

    #[test]
    #[serial]
    fn test_blur_event_dispatched() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("input");

        let elem = native_create_element(win, tag.as_ptr());
        native_set_root(win, elem);

        let blur_callback = 51u64;
        native_add_event_listener(elem, EVENT_BLUR, blur_callback);

        native_focus(elem);
        // Clear focus event
        let mut event = NativeEventData::default();
        native_poll_event(&mut event);

        native_blur(elem);

        let result = native_poll_event(&mut event);
        assert_eq!(result, EVENT_BLUR);
        assert_eq!(event.callback_id, blur_callback);
    }

    #[test]
    #[serial]
    fn test_event_bubbling() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("div");

        let parent = native_create_element(win, tag.as_ptr());
        native_set_style(parent, cstr("width").as_ptr(), cstr("200px").as_ptr());
        native_set_style(parent, cstr("height").as_ptr(), cstr("200px").as_ptr());

        let child = native_create_element(win, tag.as_ptr());
        native_set_style(child, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(child, cstr("height").as_ptr(), cstr("100px").as_ptr());

        native_append_child(parent, child);
        native_set_root(win, parent);

        let parent_callback = 54u64;
        let child_callback = 55u64;
        native_add_event_listener(parent, EVENT_CLICK, parent_callback);
        native_add_event_listener(child, EVENT_CLICK, child_callback);

        // Click on child
        native_simulate_click(win, 50.0, 50.0);

        // Should receive child event first (target)
        let mut event1 = NativeEventData::default();
        native_poll_event(&mut event1);
        assert_eq!(event1.callback_id, child_callback);

        // Then parent event (bubbling)
        let mut event2 = NativeEventData::default();
        native_poll_event(&mut event2);
        assert_eq!(event2.callback_id, parent_callback);
    }

    #[test]
    #[serial]
    fn test_remove_event_listener() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 400, 300);
        let tag = cstr("div");

        let elem = native_create_element(win, tag.as_ptr());
        native_set_style(elem, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(elem, cstr("height").as_ptr(), cstr("100px").as_ptr());
        native_set_root(win, elem);

        let callback_id = 44u64;
        native_add_event_listener(elem, EVENT_CLICK, callback_id);
        native_remove_event_listener(elem, EVENT_CLICK, callback_id);

        native_simulate_click(win, 50.0, 50.0);

        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        // No event should be queued
        assert_eq!(result, -1);
    }

    // =========================================================================
    // Phase 6: Timing
    // =========================================================================

    #[test]
    #[serial]
    fn test_now_ms_increases() {
        let t1 = native_now_ms();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let t2 = native_now_ms();
        assert!(t2 > t1, "Time should increase");
    }

    #[test]
    #[serial]
    fn test_set_timeout_fires() {
        reset_state();
        let callback_id = 100u64;
        let timer_id = native_set_timeout(callback_id, 50); // 50ms delay

        assert!(timer_id > 0, "Timer ID should be non-zero");

        // Wait for timeout to elapse
        std::thread::sleep(std::time::Duration::from_millis(60));

        // native_poll_event processes timers internally, no need for native_poll_events()
        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        assert_eq!(result, EVENT_TIMEOUT);
        assert_eq!(event.callback_id, callback_id);
    }

    #[test]
    #[serial]
    fn test_clear_timeout_prevents_fire() {
        reset_state();
        let callback_id = 101u64;
        let timer_id = native_set_timeout(callback_id, 50);

        // Cancel the timeout immediately
        native_clear_timeout(timer_id);

        // Wait past when it would have fired
        std::thread::sleep(std::time::Duration::from_millis(60));

        // native_poll_event processes timers internally
        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        assert_eq!(result, -1, "Cleared timeout should not fire");
    }

    #[test]
    #[serial]
    fn test_request_animation_frame_fires() {
        reset_state();
        let callback_id = 102u64;
        let frame_id = native_request_animation_frame(callback_id);

        assert!(frame_id > 0, "Frame ID should be non-zero");

        // native_poll_event processes animation frames internally
        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        assert_eq!(result, EVENT_ANIMATION_FRAME);
        assert_eq!(event.callback_id, callback_id);
    }

    #[test]
    #[serial]
    fn test_cancel_animation_frame_prevents_fire() {
        reset_state();
        let callback_id = 103u64;
        let frame_id = native_request_animation_frame(callback_id);

        // Cancel the animation frame
        native_cancel_animation_frame(frame_id);

        // native_poll_event processes animation frames internally
        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        assert_eq!(result, -1, "Cancelled animation frame should not fire");
    }

    // =========================================================================
    // Phase 7: Root Element
    // =========================================================================

    #[test]
    #[serial]
    fn test_set_and_get_root() {
        reset_state();
        let title = cstr("Test");
        let win = native_create_window(title.as_ptr(), 800, 600);
        let tag = cstr("div");
        let elem = native_create_element(win, tag.as_ptr());

        assert_eq!(native_get_root(win), 0); // No root initially

        native_set_root(win, elem);
        assert_eq!(native_get_root(win), elem);
    }

    // =========================================================================
    // Phase 8: Integration Test - Counter App
    // =========================================================================

    #[test]
    #[serial]
    fn integration_counter_app() {
        reset_state();

        // Create window
        let title = cstr("Counter");
        let win = native_create_window(title.as_ptr(), 400, 200);

        // Build UI
        let div_tag = cstr("div");
        let button_tag = cstr("button");

        // Container
        let container = native_create_element(win, div_tag.as_ptr());
        native_set_style(container, cstr("display").as_ptr(), cstr("flex").as_ptr());
        native_set_style(container, cstr("flex-direction").as_ptr(), cstr("column").as_ptr());
        native_set_style(container, cstr("align-items").as_ptr(), cstr("center").as_ptr());
        native_set_style(container, cstr("padding").as_ptr(), cstr("20px").as_ptr());
        native_set_style(container, cstr("width").as_ptr(), cstr("400px").as_ptr());
        native_set_style(container, cstr("height").as_ptr(), cstr("200px").as_ptr());
        native_set_style(container, cstr("background-color").as_ptr(), cstr("#f0f0f0").as_ptr());

        // Count display
        let count_text = native_create_element(win, div_tag.as_ptr());
        native_set_style(count_text, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(count_text, cstr("height").as_ptr(), cstr("40px").as_ptr());
        native_set_style(count_text, cstr("background-color").as_ptr(), cstr("#ffffff").as_ptr());
        let content = cstr("Count: 0");
        native_set_text_content(count_text, content.as_ptr());

        // Increment button
        let button = native_create_element(win, button_tag.as_ptr());
        native_set_style(button, cstr("width").as_ptr(), cstr("100px").as_ptr());
        native_set_style(button, cstr("height").as_ptr(), cstr("40px").as_ptr());
        native_set_style(button, cstr("background-color").as_ptr(), cstr("#4CAF50").as_ptr());
        let button_text = cstr("Increment");
        native_set_text_content(button, button_text.as_ptr());

        // Build tree
        native_append_child(container, count_text);
        native_append_child(container, button);
        native_set_root(win, container);

        // Add click listener to button
        let callback_id = 100u64;
        native_add_event_listener(button, EVENT_CLICK, callback_id);

        // Render initial state
        native_render(win);

        // Get button layout for click coordinates
        let mut button_layout = Layout::default();
        native_get_layout(button, &mut button_layout);

        // Verify initial render has our elements
        // Check that green button is rendered somewhere
        let has_green = native_has_pixels_matching(win, 0, 100, 150, 200, 0, 100);
        assert_eq!(has_green, 1, "Should have green button pixels");

        // Simulate click on button
        native_simulate_click(win, button_layout.x + 50.0, button_layout.y + 20.0);

        // Process click event
        let mut event = NativeEventData::default();
        let result = native_poll_event(&mut event);

        assert_eq!(result, EVENT_CLICK, "Should receive click event");
        assert_eq!(event.callback_id, callback_id, "Callback ID should match");

        // In a real app, we would:
        // 1. Look up the callback
        // 2. Execute the handler (count += 1)
        // 3. Update the text content
        // 4. Re-render

        // For this test, we verify the event was received correctly
        // The handler would update: native_set_text_content(count_text, "Count: 1");

        // Update count (simulating what the handler would do)
        let new_content = cstr("Count: 1");
        native_set_text_content(count_text, new_content.as_ptr());

        // Re-render
        native_render(win);

        // Verify text content was updated
        let len = native_get_text_content(count_text, std::ptr::null_mut(), 0);
        assert_eq!(len, 8); // "Count: 1" is 8 chars

        // Clean up
        native_destroy_window(win);
    }

    // =========================================================================
    // Phase 3: Text Rendering Tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_text_renders_to_framebuffer() {
        reset_state();

        // Create window and element with text
        let title = cstr("Text Test");
        let win = native_create_window(title.as_ptr(), 200, 100);

        let tag = cstr("div");
        let container = native_create_element(win, tag.as_ptr());

        // Set background to white and text to black
        let bg_prop = cstr("background-color");
        let bg_val = cstr("white");
        native_set_style(container, bg_prop.as_ptr(), bg_val.as_ptr());

        let color_prop = cstr("color");
        let color_val = cstr("black");
        native_set_style(container, color_prop.as_ptr(), color_val.as_ptr());

        // Set dimensions
        let w_prop = cstr("width");
        let w_val = cstr("200px");
        native_set_style(container, w_prop.as_ptr(), w_val.as_ptr());

        let h_prop = cstr("height");
        let h_val = cstr("100px");
        native_set_style(container, h_prop.as_ptr(), h_val.as_ptr());

        // Set text content
        let text = cstr("Hello");
        native_set_text_content(container, text.as_ptr());

        native_set_root(win, container);
        native_compute_layout(win);
        native_render(win);

        // Check that non-white pixels exist (text should be rendered)
        // Text pixels will be somewhere between black and white due to anti-aliasing
        // Look for pixels that are darker than pure white (255,255,255)
        let has_text = native_has_pixels_matching(win, 0, 200, 0, 200, 0, 200);
        assert_eq!(has_text, 1, "Text should render dark pixels to framebuffer");

        native_destroy_window(win);
    }

    #[test]
    #[serial]
    fn test_text_measurement() {
        reset_state();

        // Test that text measurement works via the TextSystem
        let mut state = STATE.lock();
        let (width, height) = state.text_system.measure_text("Hello", 16.0, None);

        // Text should have non-zero dimensions
        assert!(width > 0.0, "Text width should be positive, got {}", width);
        assert!(height > 0.0, "Text height should be positive, got {}", height);

        // "Hello" at 16px should be roughly 40-60px wide
        assert!(width > 20.0, "Text width should be reasonable (>20px), got {}", width);
        assert!(width < 100.0, "Text width should be reasonable (<100px), got {}", width);
    }

    #[test]
    #[serial]
    fn test_text_with_color() {
        reset_state();

        // Create window and element with colored text
        let title = cstr("Color Test");
        let win = native_create_window(title.as_ptr(), 200, 100);

        let tag = cstr("div");
        let container = native_create_element(win, tag.as_ptr());

        // White background
        let bg_prop = cstr("background-color");
        let bg_val = cstr("white");
        native_set_style(container, bg_prop.as_ptr(), bg_val.as_ptr());

        // Red text
        let color_prop = cstr("color");
        let color_val = cstr("red");
        native_set_style(container, color_prop.as_ptr(), color_val.as_ptr());

        // Set dimensions
        let w_prop = cstr("width");
        let w_val = cstr("200px");
        native_set_style(container, w_prop.as_ptr(), w_val.as_ptr());

        let h_prop = cstr("height");
        let h_val = cstr("100px");
        native_set_style(container, h_prop.as_ptr(), h_val.as_ptr());

        // Set text content
        let text = cstr("Red");
        native_set_text_content(container, text.as_ptr());

        native_set_root(win, container);
        native_compute_layout(win);
        native_render(win);

        // Look for reddish pixels (high red, low green/blue)
        let has_red = native_has_pixels_matching(win, 100, 255, 0, 150, 0, 150);
        assert_eq!(has_red, 1, "Red text should render with high red channel");

        native_destroy_window(win);
    }

    // =========================================================================
    // Phase 4: Additional Layout Features Tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_grid_layout() {
        reset_state();

        let title = cstr("Grid Test");
        let win = native_create_window(title.as_ptr(), 300, 200);

        // Create a grid container
        let tag = cstr("div");
        let container = native_create_element(win, tag.as_ptr());

        // Set grid display
        let display_prop = cstr("display");
        let display_val = cstr("grid");
        native_set_style(container, display_prop.as_ptr(), display_val.as_ptr());

        // Set grid template columns: 100px 100px 100px
        let cols_prop = cstr("grid-template-columns");
        let cols_val = cstr("100px 100px 100px");
        native_set_style(container, cols_prop.as_ptr(), cols_val.as_ptr());

        // Container size
        let w_prop = cstr("width");
        let w_val = cstr("300px");
        native_set_style(container, w_prop.as_ptr(), w_val.as_ptr());

        let h_prop = cstr("height");
        let h_val = cstr("200px");
        native_set_style(container, h_prop.as_ptr(), h_val.as_ptr());

        // Create three grid items
        let item1 = native_create_element(win, tag.as_ptr());
        let item2 = native_create_element(win, tag.as_ptr());
        let item3 = native_create_element(win, tag.as_ptr());

        // Set backgrounds
        let bg_prop = cstr("background-color");
        let red = cstr("red");
        let green = cstr("green");
        let blue = cstr("blue");
        native_set_style(item1, bg_prop.as_ptr(), red.as_ptr());
        native_set_style(item2, bg_prop.as_ptr(), green.as_ptr());
        native_set_style(item3, bg_prop.as_ptr(), blue.as_ptr());

        native_append_child(container, item1);
        native_append_child(container, item2);
        native_append_child(container, item3);

        native_set_root(win, container);
        native_compute_layout(win);

        // Check that items are laid out in a row (grid)
        let mut layout1 = Layout::default();
        let mut layout2 = Layout::default();
        let mut layout3 = Layout::default();
        native_get_layout(item1, &mut layout1);
        native_get_layout(item2, &mut layout2);
        native_get_layout(item3, &mut layout3);

        // Items should be at x=0, x=100, x=200
        assert!((layout1.x - 0.0).abs() < 1.0, "Item 1 should be at x=0, got {}", layout1.x);
        assert!((layout2.x - 100.0).abs() < 1.0, "Item 2 should be at x=100, got {}", layout2.x);
        assert!((layout3.x - 200.0).abs() < 1.0, "Item 3 should be at x=200, got {}", layout3.x);

        native_destroy_window(win);
    }

    #[test]
    #[serial]
    fn test_absolute_positioning() {
        reset_state();

        let title = cstr("Position Test");
        let win = native_create_window(title.as_ptr(), 400, 400);

        let tag = cstr("div");
        let container = native_create_element(win, tag.as_ptr());

        // Container setup
        let w_prop = cstr("width");
        let w_val = cstr("400px");
        native_set_style(container, w_prop.as_ptr(), w_val.as_ptr());

        let h_prop = cstr("height");
        let h_val = cstr("400px");
        native_set_style(container, h_prop.as_ptr(), h_val.as_ptr());

        // Create absolutely positioned child
        let child = native_create_element(win, tag.as_ptr());

        let pos_prop = cstr("position");
        let pos_val = cstr("absolute");
        native_set_style(child, pos_prop.as_ptr(), pos_val.as_ptr());

        let top_prop = cstr("top");
        let top_val = cstr("50px");
        native_set_style(child, top_prop.as_ptr(), top_val.as_ptr());

        let left_prop = cstr("left");
        let left_val = cstr("100px");
        native_set_style(child, left_prop.as_ptr(), left_val.as_ptr());

        let child_w = cstr("80px");
        let child_h = cstr("60px");
        native_set_style(child, w_prop.as_ptr(), child_w.as_ptr());
        native_set_style(child, h_prop.as_ptr(), child_h.as_ptr());

        let bg_prop = cstr("background-color");
        let blue = cstr("blue");
        native_set_style(child, bg_prop.as_ptr(), blue.as_ptr());

        native_append_child(container, child);
        native_set_root(win, container);
        native_compute_layout(win);

        // Check that child is positioned at (100, 50)
        let mut layout = Layout::default();
        native_get_layout(child, &mut layout);

        assert!((layout.x - 100.0).abs() < 1.0, "Child should be at x=100, got {}", layout.x);
        assert!((layout.y - 50.0).abs() < 1.0, "Child should be at y=50, got {}", layout.y);

        native_destroy_window(win);
    }

    #[test]
    #[serial]
    fn test_z_index_ordering() {
        reset_state();

        let title = cstr("Z-Index Test");
        let win = native_create_window(title.as_ptr(), 200, 200);

        let tag = cstr("div");
        let container = native_create_element(win, tag.as_ptr());

        let w_prop = cstr("width");
        let h_prop = cstr("height");
        let w_val = cstr("200px");
        let h_val = cstr("200px");
        native_set_style(container, w_prop.as_ptr(), w_val.as_ptr());
        native_set_style(container, h_prop.as_ptr(), h_val.as_ptr());

        // Create two overlapping elements
        let bg_prop = cstr("background-color");
        let pos_prop = cstr("position");
        let abs_val = cstr("absolute");
        let z_prop = cstr("z-index");

        // First child: red box, z-index: 1
        let child1 = native_create_element(win, tag.as_ptr());
        native_set_style(child1, pos_prop.as_ptr(), abs_val.as_ptr());
        let top0 = cstr("0px");
        let left0 = cstr("0px");
        let top_prop = cstr("top");
        let left_prop = cstr("left");
        native_set_style(child1, top_prop.as_ptr(), top0.as_ptr());
        native_set_style(child1, left_prop.as_ptr(), left0.as_ptr());
        let red = cstr("red");
        native_set_style(child1, bg_prop.as_ptr(), red.as_ptr());
        let size100 = cstr("100px");
        native_set_style(child1, w_prop.as_ptr(), size100.as_ptr());
        native_set_style(child1, h_prop.as_ptr(), size100.as_ptr());
        let z1 = cstr("1");
        native_set_style(child1, z_prop.as_ptr(), z1.as_ptr());

        // Second child: blue box, z-index: 2 (should render on top)
        let child2 = native_create_element(win, tag.as_ptr());
        native_set_style(child2, pos_prop.as_ptr(), abs_val.as_ptr());
        let top50 = cstr("50px");
        let left50 = cstr("50px");
        native_set_style(child2, top_prop.as_ptr(), top50.as_ptr());
        native_set_style(child2, left_prop.as_ptr(), left50.as_ptr());
        let blue = cstr("blue");
        native_set_style(child2, bg_prop.as_ptr(), blue.as_ptr());
        native_set_style(child2, w_prop.as_ptr(), size100.as_ptr());
        native_set_style(child2, h_prop.as_ptr(), size100.as_ptr());
        let z2 = cstr("2");
        native_set_style(child2, z_prop.as_ptr(), z2.as_ptr());

        native_append_child(container, child1);
        native_append_child(container, child2);
        native_set_root(win, container);
        native_compute_layout(win);
        native_render(win);

        // In the overlap region (75, 75), blue should be on top
        let mut pixel = Pixel::default();
        native_sample_pixel(win, 75, 75, &mut pixel);

        // Blue has r=0, b=255
        assert!(pixel.b > pixel.r, "Blue should be on top (b={}, r={})", pixel.b, pixel.r);

        native_destroy_window(win);
    }

    #[test]
    #[serial]
    fn test_scroll_offset() {
        reset_state();

        let title = cstr("Scroll Test");
        let win = native_create_window(title.as_ptr(), 200, 200);

        let tag = cstr("div");
        let container = native_create_element(win, tag.as_ptr());

        let w_prop = cstr("width");
        let h_prop = cstr("height");
        let w_val = cstr("200px");
        let h_val = cstr("200px");
        native_set_style(container, w_prop.as_ptr(), w_val.as_ptr());
        native_set_style(container, h_prop.as_ptr(), h_val.as_ptr());

        // Set overflow to scroll
        let overflow_prop = cstr("overflow");
        let scroll_val = cstr("scroll");
        native_set_style(container, overflow_prop.as_ptr(), scroll_val.as_ptr());

        // Create a child element
        let child = native_create_element(win, tag.as_ptr());
        let bg_prop = cstr("background-color");
        let blue = cstr("blue");
        native_set_style(child, bg_prop.as_ptr(), blue.as_ptr());
        let child_w = cstr("100px");
        let child_h = cstr("100px");
        native_set_style(child, w_prop.as_ptr(), child_w.as_ptr());
        native_set_style(child, h_prop.as_ptr(), child_h.as_ptr());

        native_append_child(container, child);
        native_set_root(win, container);
        native_compute_layout(win);

        // Test set/get scroll offset
        native_set_scroll_offset(container, 10.0, 20.0);

        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        native_get_scroll_offset(container, &mut x, &mut y);

        assert!((x - 10.0).abs() < 0.01, "Scroll X should be 10.0, got {}", x);
        assert!((y - 20.0).abs() < 0.01, "Scroll Y should be 20.0, got {}", y);

        native_destroy_window(win);
    }

    #[test]
    #[serial]
    fn test_min_max_dimensions() {
        reset_state();

        let title = cstr("MinMax Test");
        let win = native_create_window(title.as_ptr(), 400, 400);

        let tag = cstr("div");
        let container = native_create_element(win, tag.as_ptr());

        // Container that's 400x400
        let w_prop = cstr("width");
        let h_prop = cstr("height");
        let w_val = cstr("400px");
        let h_val = cstr("400px");
        native_set_style(container, w_prop.as_ptr(), w_val.as_ptr());
        native_set_style(container, h_prop.as_ptr(), h_val.as_ptr());

        // Child with max-width: 100px
        let child = native_create_element(win, tag.as_ptr());
        let max_w_prop = cstr("max-width");
        let max_val = cstr("100px");
        native_set_style(child, max_w_prop.as_ptr(), max_val.as_ptr());

        // Try to set width to 200px
        let large_w = cstr("200px");
        native_set_style(child, w_prop.as_ptr(), large_w.as_ptr());

        let child_h = cstr("50px");
        native_set_style(child, h_prop.as_ptr(), child_h.as_ptr());

        let bg_prop = cstr("background-color");
        let red = cstr("red");
        native_set_style(child, bg_prop.as_ptr(), red.as_ptr());

        native_append_child(container, child);
        native_set_root(win, container);
        native_compute_layout(win);

        // Child should be clamped to max-width: 100px
        let mut layout = Layout::default();
        native_get_layout(child, &mut layout);

        assert!((layout.width - 100.0).abs() < 1.0, "Width should be clamped to 100, got {}", layout.width);

        native_destroy_window(win);
    }

    // =========================================================================
    // Clipboard API Tests (CLIPBOARD-SPEC.md Phase 1)
    // =========================================================================

    #[test]
    #[serial]
    fn test_clipboard_api_version() {
        let version = native_clipboard_api_version();
        // v0.2.0 = 0x000200
        assert_eq!(version, 0x000200, "API version should be 0x000200 (v0.2.0)");
    }

    #[test]
    #[serial]
    fn test_clipboard_capabilities() {
        let caps = native_clipboard_capabilities();
        // Should have at minimum read and write capabilities
        assert!(caps & CLIPBOARD_CAP_READ != 0, "Should have read capability");
        assert!(caps & CLIPBOARD_CAP_WRITE != 0, "Should have write capability");
    }

    #[test]
    #[serial]
    fn test_write_begin_returns_nonzero_handle() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        assert!(handle > 0, "Write handle should be non-zero");
    }

    #[test]
    #[serial]
    fn test_write_begin_increments_handle() {
        reset_state();
        let handle1 = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        let handle2 = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        assert!(handle2 > handle1, "Second handle should be greater than first");
    }

    #[test]
    #[serial]
    fn test_write_add_format_with_valid_handle() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        let mime = cstr("text/plain");
        let data = b"Hello, clipboard!";
        let result = native_clipboard_write_add_format(
            handle,
            mime.as_ptr() as *const u8,
            data.as_ptr(),
            data.len()
        );
        assert_eq!(result, 1, "Adding format with valid handle should return 1 (success)");
    }

    #[test]
    #[serial]
    fn test_write_add_format_with_invalid_handle() {
        reset_state();
        let mime = cstr("text/plain");
        let data = b"Hello!";
        let result = native_clipboard_write_add_format(
            99999, // Invalid handle
            mime.as_ptr() as *const u8,
            data.as_ptr(),
            data.len()
        );
        assert_eq!(result, 0, "Invalid handle should return 0 (failure)");
    }

    #[test]
    #[serial]
    fn test_write_add_sensitive_with_valid_handle() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        let mime = cstr("text/plain");
        let data = b"secret password";
        let result = native_clipboard_write_add_sensitive(
            handle,
            mime.as_ptr() as *const u8,
            data.as_ptr(),
            data.len()
        );
        assert_eq!(result, 1, "Adding sensitive format should return 1 (success)");
    }

    #[test]
    #[serial]
    fn test_write_add_sensitive_with_invalid_handle() {
        reset_state();
        let mime = cstr("text/plain");
        let data = b"secret";
        let result = native_clipboard_write_add_sensitive(
            99999, // Invalid handle
            mime.as_ptr() as *const u8,
            data.as_ptr(),
            data.len()
        );
        assert_eq!(result, 0, "Invalid handle should return 0 (failure)");
    }

    #[test]
    #[serial]
    fn test_write_cancel_removes_handle() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        native_clipboard_write_cancel(handle);

        // Try to add format to cancelled handle - should fail
        let mime = cstr("text/plain");
        let data = b"test";
        let result = native_clipboard_write_add_format(
            handle,
            mime.as_ptr() as *const u8,
            data.as_ptr(),
            data.len()
        );
        assert_eq!(result, 0, "Cancelled handle should return 0 (failure)");
    }

    #[test]
    #[serial]
    fn test_write_commit_invalid_handle_fires_error() {
        reset_state();
        let callback_id: u64 = 12345;

        // Commit with invalid handle
        native_clipboard_write_commit(99999, callback_id);

        // Poll for error event
        let mut event_data = NativeEventData::default();
        let event_type = native_poll_event(&mut event_data);
        assert_eq!(event_type, EVENT_CLIPBOARD_ERROR, "Should get clipboard error event");

        // Verify error details
        assert_eq!(event_data.callback_id, callback_id, "Callback ID should match");
        assert_eq!(event_data.button, CLIPBOARD_ERR_INVALID_HANDLE as i32, "Should be invalid handle error");
    }

    #[test]
    #[serial]
    fn test_clipboard_release_removes_completed_data() {
        reset_state();
        let callback_id: u64 = 54321;

        // Manually insert completed data to simulate a completed read
        {
            let mut state = STATE.lock();
            state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
                data: b"test data".to_vec(),
                formats: None,
                format_cstrings: Vec::new(),
                completed_at: std::time::Instant::now(),
            });
        }

        // Verify data exists
        assert_eq!(native_clipboard_get_data_size(callback_id), 9);

        // Release the data
        native_clipboard_release(callback_id);

        // Data should be gone
        assert_eq!(native_clipboard_get_data_size(callback_id), 0);
    }

    #[test]
    #[serial]
    fn test_clipboard_cancel_removes_pending() {
        reset_state();
        let callback_id: u64 = 11111;

        // Manually insert completed data
        {
            let mut state = STATE.lock();
            state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
                data: b"pending data".to_vec(),
                formats: None,
                format_cstrings: Vec::new(),
                completed_at: std::time::Instant::now(),
            });
        }

        // Cancel should remove it (same as release)
        native_clipboard_cancel(callback_id);

        // Data should be gone
        assert_eq!(native_clipboard_get_data_size(callback_id), 0);
    }

    #[test]
    #[serial]
    fn test_clipboard_get_data() {
        reset_state();
        let callback_id: u64 = 22222;
        let test_data = b"Hello from clipboard!";

        // Insert test data
        {
            let mut state = STATE.lock();
            state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
                data: test_data.to_vec(),
                formats: None,
                format_cstrings: Vec::new(),
                completed_at: std::time::Instant::now(),
            });
        }

        // Read it back
        let mut buf = [0u8; 64];
        let len = native_clipboard_get_data(callback_id, buf.as_mut_ptr(), 64);
        assert_eq!(len, test_data.len(), "Length should match");
        assert_eq!(&buf[..len], test_data, "Data should match");
    }

    #[test]
    #[serial]
    fn test_clipboard_get_data_truncates() {
        reset_state();
        let callback_id: u64 = 33333;
        let test_data = b"This is a longer string";

        {
            let mut state = STATE.lock();
            state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
                data: test_data.to_vec(),
                formats: None,
                format_cstrings: Vec::new(),
                completed_at: std::time::Instant::now(),
            });
        }

        // Read with small buffer
        let mut buf = [0u8; 10];
        let len = native_clipboard_get_data(callback_id, buf.as_mut_ptr(), 10);
        assert_eq!(len, 10, "Should truncate to buffer size");
        assert_eq!(&buf[..], &test_data[..10], "Should get first 10 bytes");
    }

    #[test]
    #[serial]
    #[ignore] // Requires GUI environment with actual clipboard access
    fn test_copy_paste_roundtrip() {
        reset_state();
        let title = cstr("Clipboard Test");
        let _win = native_create_window(title.as_ptr(), 400, 300);

        // Write to clipboard
        let write_handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        let mime = cstr("text/plain");
        let test_text = b"Clipboard roundtrip test!";
        native_clipboard_write_add_format(
            write_handle,
            mime.as_ptr() as *const u8,
            test_text.as_ptr(),
            test_text.len()
        );

        let write_callback: u64 = 100;
        native_clipboard_write_commit(write_handle, write_callback);

        // Poll for write complete
        let mut event_data = NativeEventData::default();
        let mut event_type = native_poll_event(&mut event_data);
        while event_type != EVENT_CLIPBOARD_WRITE_COMPLETE && event_type != EVENT_CLIPBOARD_ERROR {
            std::thread::sleep(std::time::Duration::from_millis(10));
            event_type = native_poll_event(&mut event_data);
        }
        assert_eq!(event_type, EVENT_CLIPBOARD_WRITE_COMPLETE, "Write should succeed");

        // Read back from clipboard
        let read_callback: u64 = 200;
        native_clipboard_read_format(
            ClipboardTarget::Clipboard as i32,
            mime.as_ptr() as *const u8,
            read_callback
        );

        // Poll for data ready
        event_type = native_poll_event(&mut event_data);
        while event_type != EVENT_CLIPBOARD_DATA_READY && event_type != EVENT_CLIPBOARD_ERROR {
            std::thread::sleep(std::time::Duration::from_millis(10));
            event_type = native_poll_event(&mut event_data);
        }
        assert_eq!(event_type, EVENT_CLIPBOARD_DATA_READY, "Read should succeed");

        // Get the data
        let size = native_clipboard_get_data_size(read_callback);
        let mut buf = vec![0u8; size];
        let len = native_clipboard_get_data(read_callback, buf.as_mut_ptr(), size);
        assert_eq!(len, test_text.len(), "Length should match");
        assert_eq!(&buf[..len], test_text, "Content should match");

        // Cleanup
        native_clipboard_release(read_callback);
    }

    // =========================================================================
    // Additional Clipboard Tests (Coverage gaps)
    // =========================================================================

    #[test]
    #[serial]
    fn test_get_formats_data_retrieves_formats() {
        reset_state();
        let callback_id: u64 = 44444;

        // Manually insert completed data with formats
        {
            let mut state = STATE.lock();
            state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
                data: Vec::new(),
                formats: Some(vec!["text/plain".to_string(), "text/html".to_string()]),
                format_cstrings: Vec::new(),
                completed_at: std::time::Instant::now(),
            });
        }

        // Get formats
        let mut format_ptrs: [*const u8; 4] = [std::ptr::null(); 4];
        let count = native_clipboard_get_formats_data(
            callback_id,
            format_ptrs.as_mut_ptr(),
            4
        );

        assert_eq!(count, 2, "Should return 2 formats");

        // Verify format strings
        unsafe {
            let fmt0 = std::ffi::CStr::from_ptr(format_ptrs[0] as *const i8);
            let fmt1 = std::ffi::CStr::from_ptr(format_ptrs[1] as *const i8);
            assert_eq!(fmt0.to_str().unwrap(), "text/plain");
            assert_eq!(fmt1.to_str().unwrap(), "text/html");
        }

        // Cleanup
        native_clipboard_release(callback_id);
    }

    #[test]
    #[serial]
    fn test_get_formats_data_with_max_limit() {
        reset_state();
        let callback_id: u64 = 55555;

        // Insert 3 formats
        {
            let mut state = STATE.lock();
            state.clipboard.completed.insert(callback_id, ClipboardCompletedData {
                data: Vec::new(),
                formats: Some(vec![
                    "text/plain".to_string(),
                    "text/html".to_string(),
                    "text/uri-list".to_string(),
                ]),
                format_cstrings: Vec::new(),
                completed_at: std::time::Instant::now(),
            });
        }

        // Request only 2
        let mut format_ptrs: [*const u8; 2] = [std::ptr::null(); 2];
        let count = native_clipboard_get_formats_data(
            callback_id,
            format_ptrs.as_mut_ptr(),
            2
        );

        assert_eq!(count, 2, "Should return max 2 formats");

        native_clipboard_release(callback_id);
    }

    #[test]
    #[serial]
    fn test_get_formats_data_null_pointer() {
        reset_state();
        let count = native_clipboard_get_formats_data(12345, std::ptr::null_mut(), 10);
        assert_eq!(count, 0, "Null pointer should return 0");
    }

    #[test]
    #[serial]
    fn test_get_formats_data_zero_max() {
        reset_state();
        let mut format_ptrs: [*const u8; 4] = [std::ptr::null(); 4];
        let count = native_clipboard_get_formats_data(
            12345,
            format_ptrs.as_mut_ptr(),
            0
        );
        assert_eq!(count, 0, "Zero max_formats should return 0");
    }

    #[test]
    #[serial]
    fn test_get_formats_data_invalid_callback() {
        reset_state();
        let mut format_ptrs: [*const u8; 4] = [std::ptr::null(); 4];
        let count = native_clipboard_get_formats_data(
            99999, // Non-existent callback
            format_ptrs.as_mut_ptr(),
            4
        );
        assert_eq!(count, 0, "Invalid callback should return 0");
    }

    #[test]
    #[serial]
    fn test_write_add_format_null_mime() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        let data = b"test";
        let result = native_clipboard_write_add_format(
            handle,
            std::ptr::null(),
            data.as_ptr(),
            data.len()
        );
        assert_eq!(result, 0, "Null mime should return failure");
    }

    #[test]
    #[serial]
    fn test_write_add_format_null_data_with_len() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        let mime = cstr("text/plain");
        let result = native_clipboard_write_add_format(
            handle,
            mime.as_ptr() as *const u8,
            std::ptr::null(),
            10 // Non-zero length with null data
        );
        assert_eq!(result, 0, "Null data with non-zero len should return failure");
    }

    #[test]
    #[serial]
    fn test_write_add_format_empty_data() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        let mime = cstr("text/plain");
        let result = native_clipboard_write_add_format(
            handle,
            mime.as_ptr() as *const u8,
            std::ptr::null(),
            0 // Zero length is OK with null data
        );
        assert_eq!(result, 1, "Empty data should succeed");
    }

    #[test]
    #[serial]
    fn test_write_multiple_formats() {
        reset_state();
        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);

        let mime1 = cstr("text/plain");
        let data1 = b"plain text";
        let r1 = native_clipboard_write_add_format(
            handle,
            mime1.as_ptr() as *const u8,
            data1.as_ptr(),
            data1.len()
        );

        let mime2 = cstr("text/html");
        let data2 = b"<p>html</p>";
        let r2 = native_clipboard_write_add_format(
            handle,
            mime2.as_ptr() as *const u8,
            data2.as_ptr(),
            data2.len()
        );

        assert_eq!(r1, 1, "First format should succeed");
        assert_eq!(r2, 1, "Second format should succeed");

        native_clipboard_write_cancel(handle);
    }

    #[test]
    #[serial]
    fn test_cancel_unknown_callback_no_event() {
        reset_state();

        // Cancel an unknown callback_id
        native_clipboard_cancel(99999);

        // Should NOT fire any event
        let mut event_data = NativeEventData::default();
        let event_type = native_poll_event(&mut event_data);
        assert_eq!(event_type, -1, "Should not fire event for unknown callback");
    }

    #[test]
    #[serial]
    fn test_write_handle_overflow_protection() {
        reset_state();

        // Set next_write_handle to 0 to test overflow protection
        {
            let mut state = STATE.lock();
            state.clipboard.next_write_handle = 0;
        }

        let handle = native_clipboard_write_begin(ClipboardTarget::Clipboard as i32);
        assert_eq!(handle, 0, "Should return 0 when handle would overflow to 0");
    }
}
