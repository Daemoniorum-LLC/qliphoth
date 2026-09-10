# Scry — Terminal Emulator Specification

> *To scry: to perceive visions through a dark medium.*

Pure Sigil terminal emulator for Qliphoth native applications.

## Status

| Field | Value |
|-------|-------|
| **Status** | Draft |
| **Created** | 2025-02-17 |
| **Target** | Wraith IDE, general Qliphoth apps |
| **Repository** | `~/dev/scry` |
| **Dependencies** | qliphoth-sys (PTY bindings) |

## Overview

A terminal emulator implemented entirely in Sigil, with only the PTY layer requiring platform bindings. This follows the Athame pattern: system boundaries are wrapped in qliphoth-sys, everything above is pure Sigil.

### Design Goals

1. **Pure Sigil** — No external Rust crates for parsing or terminal logic
2. **Actor-Native** — PTY events, parser state, and terminal buffer modeled as actors
3. **Shared Infrastructure** — Reuse patterns from Athame (text grids, cursor, selection)
4. **Minimal but Complete** — Support common terminal applications (shells, vim, htop, etc.)
5. **Embeddable** — Works as a component in any Qliphoth application

### Non-Goals

- Full VT520 compatibility (focus on practical xterm subset)
- Hardware terminal emulation (DEC modes, printer support)
- Sixel graphics (future consideration)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    ScryView (Component)                      │
│                    Renders grid → VNodes                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Scry (Actor)                              │
│         Interprets actions, mutates grid state               │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Parser (Struct)                           │
│         State machine: bytes → TerminalAction                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Pty (Actor)                               │
│         Platform PTY wrapper (qliphoth-sys)                  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    OS PTY APIs                               │
│         Unix: openpty/forkpty | Windows: ConPTY              │
└─────────────────────────────────────────────────────────────┘
```

### Message Flow

```
┌─────────────────────────────────────────────────────────────┐
│                      User Input                              │
└─────────────────────────────────────────────────────────────┘
                              │
                    Key/Mouse Event
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ScryView                                  │
│   1. Encode key/mouse to escape sequence                     │
│   2. Send to Scry actor                                      │
└─────────────────────────────────────────────────────────────┘
                              │
                    KeyInput / MouseInput
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Scry                                      │
│   1. Write encoded bytes to PTY                              │
│   2. Receive PtyData, parse, execute                         │
│   3. Mutate grid/cursor state                                │
│   4. Signal ScryView to re-render                            │
└─────────────────────────────────────────────────────────────┘
                              │
                    GridUpdated / CursorMoved
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    ScryView                                  │
│   1. Query Scry for grid/cursor state                        │
│   2. Diff dirty lines                                        │
│   3. Re-render changed lines to VNodes                       │
└─────────────────────────────────────────────────────────────┘
```

**Ownership Model:**
- `ScryView` owns `Scry` actor (created in `spawn`)
- `Scry` owns `Parser` struct and `Pty` actor
- `Pty` sends `PtyData` messages to `Scry`
- `Scry` can signal `ScryView` via callback or reactive subscription

## Module Structure

```
~/dev/scry/
├── Sigil.toml
├── README.md
└── src/
    ├── lib.sigil              # Public exports
    ├── pty.sigil              # PTY actor (uses qliphoth-sys)
    ├── parser.sigil           # Escape sequence parser (struct)
    ├── grid.sigil             # Terminal grid/buffer
    ├── cell.sigil             # Cell and attribute types
    ├── cursor.sigil           # Cursor state and movement
    ├── charset.sigil          # Character set handling (line drawing)
    ├── modes.sigil            # Terminal mode flags
    ├── terminal.sigil         # Terminal actor (action → grid)
    ├── component.sigil        # ScryView component
    ├── selection.sigil        # Text selection handling
    ├── search.sigil           # Scrollback search
    ├── keys.sigil             # Key event → escape sequence
    ├── mouse.sigil            # Mouse event handling
    └── shell_integration.sigil # OSC 7/133 command tracking
```

---

## Component Specifications

### 1. PTY Layer (qliphoth-sys)

The platform boundary. Provides async PTY management.

```sigil
// qliphoth-sys/src/pty.sigil

/// Terminal dimensions
☉ Σ PtySize {
    ☉ rows: u16,
    ☉ cols: u16,
    ☉ pixel_width: u16,   // for sixel/kitty graphics (future)
    ☉ pixel_height: u16,
}

/// PTY configuration
☉ Σ PtyConfig {
    ☉ shell: Option<String>,      // None = use $SHELL or default
    ☉ working_dir: Option<String>,
    ☉ env: Vec<(String, String)>, // Additional environment variables
    ☉ size: PtySize,
}

/// Events from PTY
☉ ᛈ PtyEvent {
    /// Data received from child process
    Data { bytes: Vec<u8> },
    /// Child process exited
    Exit { code: i32 },
    /// Error occurred
    Error { message: String },
}

/// PTY actor - manages a pseudo-terminal
☉ actor Pty {
    state handle: PtyHandle!,      // Platform-specific handle
    state child_pid: u32!,
    state size: PtySize!,

    /// Spawn a new PTY with the configured shell
    ☉ rite spawn(config: PtyConfig) -> Result<Self, Error>?;

    /// Write bytes to the PTY (user input → child process)
    ☉ rite write(&self, data: &[u8]) -> Result<(), Error>?;

    /// Resize the PTY
    ☉ rite resize(&Δ self, size: PtySize) -> Result<(), Error>?;

    /// Get current size
    ☉ rite size(&self) -> PtySize!;

    /// Kill the child process
    ☉ rite kill(&self) -> Result<(), Error>?;
}
```

#### Platform Implementation Notes

**Unix (Linux, macOS, BSD):**
```c
// Uses POSIX PTY APIs
int openpty(int *amaster, int *aslave, char *name,
            const struct termios *termp,
            const struct winsize *winp);
pid_t forkpty(int *amaster, char *name,
              const struct termios *termp,
              const struct winsize *winp);
int ioctl(fd, TIOCSWINSZ, &winsize);  // resize
```

**Windows:**
```c
// Uses ConPTY (Windows 10 1809+)
HRESULT CreatePseudoConsole(COORD size, HANDLE hInput, HANDLE hOutput,
                            DWORD dwFlags, HPCON* phPC);
HRESULT ResizePseudoConsole(HPCON hPC, COORD size);
void ClosePseudoConsole(HPCON hPC);
```

---

### 2. Escape Sequence Parser

State machine implementing a subset of the VT500 parser. This is a **struct**, not an actor — it's purely synchronous with no I/O or message handling.

#### Parser States

```sigil
// terminal/src/parser.sigil

/// Parser state (VT500-derived state machine)
☉ ᛈ ParserState {
    /// Normal character processing
    Ground,
    /// Collecting UTF-8 multi-byte sequence
    Utf8 { expected: u8, collected: u8 },
    /// After ESC (0x1B)
    Escape,
    /// ESC followed by intermediate byte
    EscapeIntermediate,
    /// After CSI (ESC [)
    CsiEntry,
    /// Collecting CSI parameters
    CsiParam,
    /// CSI intermediate bytes
    CsiIntermediate,
    /// Ignoring malformed CSI
    CsiIgnore,
    /// After OSC (ESC ])
    OscString,
    /// After DCS (ESC P)
    DcsEntry,
    /// DCS parameters
    DcsParam,
    /// DCS intermediate
    DcsIntermediate,
    /// DCS passthrough data
    DcsPassthrough,
    /// Ignoring malformed DCS
    DcsIgnore,
    /// SOS/PM/APC string (ignored)
    SosPmApcString,
}
```

#### Terminal Actions

```sigil
/// Actions emitted by the parser
☉ ᛈ TerminalAction {
    /// Print a character at cursor position
    Print { ch: char },

    /// Execute C0/C1 control code
    Execute { code: u8 },

    /// CSI (Control Sequence Introducer) dispatch
    /// Examples: cursor movement, colors, erase
    CsiDispatch {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },

    /// ESC sequence dispatch
    /// Examples: save/restore cursor, charset selection
    EscDispatch {
        intermediates: Vec<u8>,
        final_byte: u8,
    },

    /// OSC (Operating System Command) dispatch
    /// Examples: set window title, set colors
    OscDispatch { params: Vec<Vec<u8>> },

    /// DCS (Device Control String) hook
    DcsHook {
        params: Vec<u16>,
        intermediates: Vec<u8>,
        final_byte: u8,
    },

    /// DCS data byte
    DcsPut { byte: u8 },

    /// DCS sequence complete
    DcsUnhook,
}
```

#### Parser Struct

```sigil
/// Escape sequence parser - synchronous state machine
☉ Σ Parser {
    /// Current parser state
    ☉ state: ParserState,
    /// CSI/DCS parameter accumulator
    ☉ params: Vec<u16>,
    /// Current parameter being built
    ☉ current_param: u32,
    /// Intermediate bytes (between ESC/CSI and final byte)
    ☉ intermediates: Vec<u8>,
    /// OSC string buffer
    ☉ osc_buffer: Vec<u8>,
    /// OSC parameters (split by ;)
    ☉ osc_params: Vec<Vec<u8>>,
    /// UTF-8 byte buffer
    ☉ utf8_buffer: Vec<u8>,
    /// Last printed character (for REP)
    ☉ last_char: char,
}

/// Maximum parameters in a CSI sequence
≔ MAX_PARAMS: usize = 16;

/// Maximum intermediate bytes
≔ MAX_INTERMEDIATES: usize = 2;

⊢ Parser {
    /// Create a new parser in ground state
    ☉ rite new() -> Self! {
        Parser {
            state: ParserState·Ground,
            params: vec![],
            current_param: 0,
            intermediates: vec![],
            osc_buffer: vec![],
            osc_params: vec![],
            utf8_buffer: vec![],
            last_char: ' ',
        }
    }

    /// Parse a single byte, potentially returning an action
    /// May return multiple actions for some sequences
    ☉ rite parse(&Δ self, byte: u8) -> Option<TerminalAction>!;

    /// Reset parser to ground state (on CAN/SUB or protocol error)
    ☉ rite reset(&Δ self);

    // =========================================================================
    // Internal Methods
    // =========================================================================

    /// Transition to a new state, clearing buffers as needed
    ☉ rite transition(&Δ self, state: ParserState);

    /// Accumulate digit into current parameter
    ☉ rite accumulate_param(&Δ self, digit: u8);

    /// Finish current parameter, push to params vec
    ☉ rite finish_param(&Δ self);

    /// Collect intermediate byte
    ☉ rite collect_intermediate(&Δ self, byte: u8);

    /// Handle UTF-8 lead byte, return expected continuation count
    ☉ rite utf8_start(&Δ self, byte: u8) -> u8!;

    /// Handle UTF-8 continuation byte, return decoded char if complete
    ☉ rite utf8_continue(&Δ self, byte: u8) -> Option<char>!;

    /// Decode completed UTF-8 buffer to char
    ☉ rite utf8_decode(&self) -> Option<char>!;
}
```

#### Supported Sequences

| Sequence | Description | Priority |
|----------|-------------|----------|
| **C0 Controls** | | |
| `0x00` NUL | Ignored | Required |
| `0x07` BEL | Bell | Required |
| `0x08` BS | Backspace | Required |
| `0x09` HT | Horizontal tab | Required |
| `0x0A` LF | Line feed | Required |
| `0x0B` VT | Vertical tab (= LF) | Required |
| `0x0C` FF | Form feed (= LF) | Required |
| `0x0D` CR | Carriage return | Required |
| `0x0E` SO | Shift Out (G1 charset) | Required |
| `0x0F` SI | Shift In (G0 charset) | Required |
| `0x18` CAN | Cancel sequence | Required |
| `0x1A` SUB | Cancel, show error char | Required |
| `0x1B` ESC | Escape | Required |
| **CSI Sequences** | | |
| `CSI Ps @` | Insert characters (ICH) | Required |
| `CSI Ps A` | Cursor up (CUU) | Required |
| `CSI Ps B` | Cursor down (CUD) | Required |
| `CSI Ps C` | Cursor forward (CUF) | Required |
| `CSI Ps D` | Cursor back (CUB) | Required |
| `CSI Ps E` | Cursor next line (CNL) | Required |
| `CSI Ps F` | Cursor preceding line (CPL) | Required |
| `CSI Ps G` | Cursor character absolute (CHA) | Required |
| `CSI Ps ; Ps H` | Cursor position (CUP) | Required |
| `CSI Ps J` | Erase display (ED) | Required |
| `CSI Ps K` | Erase line (EL) | Required |
| `CSI Ps L` | Insert lines (IL) | Required |
| `CSI Ps M` | Delete lines (DL) | Required |
| `CSI Ps P` | Delete characters (DCH) | Required |
| `CSI Ps S` | Scroll up (SU) | Required |
| `CSI Ps T` | Scroll down (SD) | Required |
| `CSI Ps X` | Erase characters (ECH) | Required |
| `CSI Ps b` | Repeat preceding char (REP) | Required |
| `CSI Ps c` | Device attributes (DA1) | Required |
| `CSI > Ps c` | Secondary device attrs (DA2) | Required |
| `CSI Ps d` | Line position absolute (VPA) | Required |
| `CSI Ps ; Ps f` | Cursor position (HVP) | Required |
| `CSI Ps h` | Set mode (SM) | Required |
| `CSI Ps l` | Reset mode (RM) | Required |
| `CSI Ps m` | SGR (colors/styles) | Required |
| `CSI Ps n` | Device status report (DSR) | Required |
| `CSI Ps ; Ps r` | Set scroll region (DECSTBM) | Required |
| `CSI s` | Save cursor (ANSI.SYS) | Required |
| `CSI Ps t` | Window manipulation | Important |
| `CSI u` | Restore cursor (ANSI.SYS) | Required |
| `CSI ? Ps h` | DEC private mode set | Required |
| `CSI ? Ps l` | DEC private mode reset | Required |
| `CSI Ps SP q` | Cursor shape (DECSCUSR) | Required |
| `CSI ? Ps c` | Tertiary device attrs (DA3) | Optional |
| **ESC Sequences** | | |
| `ESC 7` | Save cursor (DECSC) | Required |
| `ESC 8` | Restore cursor (DECRC) | Required |
| `ESC D` | Index (IND) | Required |
| `ESC E` | Next line (NEL) | Required |
| `ESC H` | Tab set (HTS) | Required |
| `ESC M` | Reverse index (RI) | Required |
| `ESC =` | Application keypad (DECKPAM) | Required |
| `ESC >` | Normal keypad (DECKPNM) | Required |
| `ESC c` | Full reset (RIS) | Required |
| `ESC ( C` | Designate G0 charset | Required |
| `ESC ) C` | Designate G1 charset | Required |
| `ESC # 8` | DEC screen alignment test | Optional |
| **OSC Sequences** | | |
| `OSC 0 ; Pt ST` | Set icon name and title | Required |
| `OSC 1 ; Pt ST` | Set icon name | Important |
| `OSC 2 ; Pt ST` | Set window title | Required |
| `OSC 4 ; c ; spec ST` | Set color palette | Important |
| `OSC 7 ; Pt ST` | Set working directory | Important |
| `OSC 8 ; params ; uri ST` | Hyperlink | Important |
| `OSC 10 ; Pt ST` | Set/query foreground | Important |
| `OSC 11 ; Pt ST` | Set/query background | Important |
| `OSC 12 ; Pt ST` | Set/query cursor color | Optional |
| `OSC 52 ; Pc ; Pd ST` | Clipboard access | Optional |
| `OSC 104 ; c ST` | Reset color palette | Optional |
| `OSC 133 ; A/B/C/D ST` | Shell integration | **Required** |

---

### 3. Terminal Grid

Data structures for the terminal screen buffer.

```sigil
// terminal/src/grid.sigil

/// RGB color
☉ Σ Color {
    ☉ r: u8,
    ☉ g: u8,
    ☉ b: u8,
}

⊢ Color {
    /// Standard ANSI colors (0-7)
    ☉ rite indexed(index: u8) -> Self!;

    /// Bright ANSI colors (8-15)
    ☉ rite indexed_bright(index: u8) -> Self!;

    /// 256-color palette
    ☉ rite palette(index: u8) -> Self!;

    /// True color RGB
    ☉ rite rgb(r: u8, g: u8, b: u8) -> Self!;

    /// Convert to CSS color string
    ☉ rite to_css(&self) -> String!;

    /// Default foreground
    ☉ rite default_fg() -> Self!;

    /// Default background
    ☉ rite default_bg() -> Self!;
}

/// Cell text attributes
☉ Σ CellAttrs {
    ☉ fg: Color,
    ☉ bg: Color,
    ☉ bold: bool,
    ☉ dim: bool,
    ☉ italic: bool,
    ☉ underline: UnderlineStyle,
    ☉ strikethrough: bool,
    ☉ inverse: bool,
    ☉ hidden: bool,
    ☉ blink: bool,
}

☉ ᛈ UnderlineStyle {
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

⊢ CellAttrs {
    ☉ rite default() -> Self!;
    ☉ rite reset(&Δ self);
}

/// A single terminal cell
☉ Σ Cell {
    /// The character displayed (empty = space)
    ☉ ch: char,
    /// Display attributes
    ☉ attrs: CellAttrs,
    /// Character width (1 for normal, 2 for wide CJK/emoji)
    ☉ width: u8,
    /// Continuation of wide character from previous cell
    ☉ is_wide_continuation: bool,
}

⊢ Cell {
    ☉ rite default() -> Self! {
        Cell {
            ch: ' ',
            attrs: CellAttrs·default(),
            width: 1,
            is_wide_continuation: false,
        }
    }

    ☉ rite with_char(ch: char, attrs: CellAttrs) -> Self!;
}

/// A single line (row) in the terminal
☉ Σ Line {
    /// Cells in this line
    ☉ cells: Vec<Cell>,
    /// Line wrapped from previous line
    ☉ wrapped: bool,
    /// Line needs re-rendering
    ☉ dirty: bool,
}

⊢ Line {
    ☉ rite new(cols: u16) -> Self!;
    ☉ rite clear(&Δ self);
    ☉ rite clear_range(&Δ self, start: u16, end: u16);
    ☉ rite mark_dirty(&Δ self);
    ☉ rite mark_clean(&Δ self);
}

/// Terminal grid with scrollback
☉ Σ Grid {
    /// Visible screen lines
    ☉ lines: Vec<Line>,
    /// Scrollback buffer (lines scrolled off top)
    ☉ scrollback: Vec<Line>,
    /// Grid dimensions
    ☉ rows: u16,
    ☉ cols: u16,
    /// Scroll region boundaries
    ☉ scroll_top: u16,
    ☉ scroll_bottom: u16,
    /// Maximum scrollback lines
    ☉ max_scrollback: usize,
}

⊢ Grid {
    ☉ rite new(rows: u16, cols: u16) -> Self!;

    /// Access a cell
    ☉ rite cell(&self, row: u16, col: u16) -> &Cell!;
    ☉ rite cell_mut(&Δ self, row: u16, col: u16) -> &Δ Cell!;

    /// Scroll operations
    ☉ rite scroll_up(&Δ self, count: u16);
    ☉ rite scroll_down(&Δ self, count: u16);
    ☉ rite scroll_up_in_region(&Δ self, count: u16, top: u16, bottom: u16);
    ☉ rite scroll_down_in_region(&Δ self, count: u16, top: u16, bottom: u16);

    /// Line operations
    ☉ rite insert_lines(&Δ self, row: u16, count: u16);
    ☉ rite delete_lines(&Δ self, row: u16, count: u16);

    /// Erase operations
    ☉ rite erase_display(&Δ self, mode: EraseMode);
    ☉ rite erase_line(&Δ self, row: u16, mode: EraseMode);

    /// Resize the grid
    ☉ rite resize(&Δ self, rows: u16, cols: u16);

    /// Get total lines including scrollback
    ☉ rite total_lines(&self) -> usize!;
}

☉ ᛈ EraseMode {
    /// Erase from cursor to end
    ToEnd,
    /// Erase from start to cursor
    ToStart,
    /// Erase entire line/display
    All,
    /// Erase scrollback (display only)
    Scrollback,
}
```

---

### 4. Character Sets

Support for DEC Special Graphics (line drawing) and other character sets.

```sigil
// terminal/src/charset.sigil

/// Character set designation
☉ ᛈ Charset {
    /// ASCII (default)
    Ascii,
    /// DEC Special Graphics (line drawing: ┌ ─ ┐ │ etc.)
    DecSpecialGraphics,
    /// UK National (# → £)
    Uk,
    /// DEC Supplemental
    DecSupplemental,
}

/// Character set state
☉ Σ CharsetState {
    /// G0 charset designation
    ☉ g0: Charset,
    /// G1 charset designation
    ☉ g1: Charset,
    /// Active charset (0 = G0, 1 = G1)
    ☉ active: u8,
}

⊢ CharsetState {
    ☉ rite new() -> Self! {
        CharsetState {
            g0: Charset·Ascii,
            g1: Charset·Ascii,
            active: 0,
        }
    }

    /// Get the currently active charset
    ☉ rite current(&self) -> Charset! {
        ⎇ self.active = 0 { self.g0 } ⎉ { self.g1 }
    }

    /// Shift Out - activate G1
    ☉ rite shift_out(&Δ self) {
        self.active = 1;
    }

    /// Shift In - activate G0
    ☉ rite shift_in(&Δ self) {
        self.active = 0;
    }

    /// Designate G0 charset
    ☉ rite designate_g0(&Δ self, charset: Charset) {
        self.g0 = charset;
    }

    /// Designate G1 charset
    ☉ rite designate_g1(&Δ self, charset: Charset) {
        self.g1 = charset;
    }

    /// Translate character through current charset
    ☉ rite translate(&self, ch: char) -> char! {
        ⌥ self.current() {
            Charset·Ascii => ch,
            Charset·DecSpecialGraphics => translate_dec_graphics(ch),
            Charset·Uk => ⎇ ch = '#' { '£' } ⎉ { ch },
            Charset·DecSupplemental => ch, // TODO: full mapping
        }
    }
}

/// Translate ASCII to DEC Special Graphics
/// Used for box drawing when ESC(0 is active
☉ rite translate_dec_graphics(ch: char) -> char! {
    ⌥ ch {
        'j' => '┘',  // Lower-right corner
        'k' => '┐',  // Upper-right corner
        'l' => '┌',  // Upper-left corner
        'm' => '└',  // Lower-left corner
        'n' => '┼',  // Crossing lines
        'q' => '─',  // Horizontal line
        't' => '├',  // Left tee
        'u' => '┤',  // Right tee
        'v' => '┴',  // Bottom tee
        'w' => '┬',  // Top tee
        'x' => '│',  // Vertical line
        'a' => '▒',  // Checkerboard
        'f' => '°',  // Degree symbol
        'g' => '±',  // Plus/minus
        'o' => '⎺',  // Scan line 1
        'p' => '⎻',  // Scan line 3
        'r' => '⎼',  // Scan line 7
        's' => '⎽',  // Scan line 9
        '`' => '◆',  // Diamond
        '~' => '·',  // Middle dot
        ',' => '←',  // Arrow left
        '+' => '→',  // Arrow right
        '.' => '↓',  // Arrow down
        '-' => '↑',  // Arrow up
        'h' => '▮',  // Board (NL)
        'i' => '␋',  // Lantern (VT)
        '0' => '█',  // Solid block
        'y' => '≤',  // Less than or equal
        'z' => '≥',  // Greater than or equal
        '{' => 'π',  // Pi
        '|' => '≠',  // Not equal
        '}' => '£',  // Pound sterling
        _ => ch,
    }
}
```

---

### 5. Cursor State

```sigil
// terminal/src/cursor.sigil

/// Cursor shape (DECSCUSR values)
☉ ᛈ CursorShape {
    /// Default (usually block)
    Default,
    /// Blinking block
    BlinkingBlock,
    /// Steady block
    SteadyBlock,
    /// Blinking underline
    BlinkingUnderline,
    /// Steady underline
    SteadyUnderline,
    /// Blinking bar (I-beam)
    BlinkingBar,
    /// Steady bar (I-beam)
    SteadyBar,
}

⊢ CursorShape {
    /// Parse from DECSCUSR parameter
    ☉ rite from_decscusr(ps: u16) -> Self! {
        ⌥ ps {
            0 => CursorShape·Default,
            1 => CursorShape·BlinkingBlock,
            2 => CursorShape·SteadyBlock,
            3 => CursorShape·BlinkingUnderline,
            4 => CursorShape·SteadyUnderline,
            5 => CursorShape·BlinkingBar,
            6 => CursorShape·SteadyBar,
            _ => CursorShape·Default,
        }
    }

    /// Check if this shape blinks
    ☉ rite is_blinking(&self) -> bool! {
        matches!(self,
            CursorShape·BlinkingBlock |
            CursorShape·BlinkingUnderline |
            CursorShape·BlinkingBar
        )
    }
}

☉ Σ Cursor {
    /// Row position (0-indexed)
    ☉ row: u16,
    /// Column position (0-indexed)
    ☉ col: u16,
    /// Cursor visibility (DECTCEM)
    ☉ visible: bool,
    /// Cursor shape (includes blink state)
    ☉ shape: CursorShape,
    /// Saved cursor state (for DECSC/DECRC)
    ☉ saved: Option<SavedCursor>,
}

☉ Σ SavedCursor {
    ☉ row: u16,
    ☉ col: u16,
    ☉ attrs: CellAttrs,
    ☉ origin_mode: bool,
    ☉ wrap_mode: bool,
    ☉ charset_state: CharsetState,
}

⊢ Cursor {
    ☉ rite new() -> Self!;
    ☉ rite save(&Δ self, attrs: &CellAttrs, origin_mode: bool, wrap_mode: bool, charset: &CharsetState);
    ☉ rite restore(&Δ self) -> Option<SavedCursor>!;
    ☉ rite move_to(&Δ self, row: u16, col: u16, grid: &Grid);
    ☉ rite move_up(&Δ self, count: u16);
    ☉ rite move_down(&Δ self, count: u16, grid: &Grid);
    ☉ rite move_forward(&Δ self, count: u16, grid: &Grid);
    ☉ rite move_back(&Δ self, count: u16);
    ☉ rite carriage_return(&Δ self);
}
```

---

### 5. Terminal Interpreter

Translates parser actions into grid mutations.

```sigil
// terminal/src/interpreter.sigil

/// Terminal modes (DEC private modes)
☉ Σ TerminalModes {
    // === DEC Private Modes ===
    /// Mode 1 - DECCKM: Application cursor keys
    ☉ application_cursor: bool,
    /// Mode 3 - DECCOLM: 132 column mode (vs 80)
    ☉ column_132: bool,
    /// Mode 5 - DECSCNM: Reverse video (swap fg/bg globally)
    ☉ reverse_video: bool,
    /// Mode 6 - DECOM: Origin mode (cursor relative to scroll region)
    ☉ origin_mode: bool,
    /// Mode 7 - DECAWM: Auto-wrap mode
    ☉ auto_wrap: bool,
    /// Mode 12 - ATT610: Cursor blink
    ☉ cursor_blink: bool,
    /// Mode 25 - DECTCEM: Cursor visible
    ☉ cursor_visible: bool,

    // === Keypad Modes ===
    /// Application keypad mode (ESC = / ESC >)
    ☉ application_keypad: bool,

    // === Screen Buffer ===
    /// Alternate screen buffer active (modes 47, 1047, 1049)
    ☉ alternate_screen: bool,

    // === Mouse Tracking ===
    /// Mouse tracking mode
    ☉ mouse_tracking: MouseMode,
    /// Mouse encoding format
    ☉ mouse_encoding: MouseEncoding,

    // === Other ===
    /// Mode 1004: Focus tracking
    ☉ focus_tracking: bool,
    /// Mode 2004: Bracketed paste mode
    ☉ bracketed_paste: bool,
}

⊢ TerminalModes {
    ☉ rite default() -> Self! {
        TerminalModes {
            application_cursor: false,
            column_132: false,
            reverse_video: false,
            origin_mode: false,
            auto_wrap: true,  // Default ON
            cursor_blink: true,
            cursor_visible: true,
            application_keypad: false,
            alternate_screen: false,
            mouse_tracking: MouseMode·None,
            mouse_encoding: MouseEncoding·X10,
            focus_tracking: false,
            bracketed_paste: false,
        }
    }
}

/// Mouse tracking mode
☉ ᛈ MouseMode {
    /// No mouse tracking
    None,
    /// Mode 9: X10 compatibility (button press only)
    X10,
    /// Mode 1000: Normal tracking (press/release)
    Normal,
    /// Mode 1002: Button-event tracking (press/release/motion while pressed)
    ButtonMotion,
    /// Mode 1003: Any-event tracking (all motion)
    AnyMotion,
}

/// Mouse coordinate encoding format
☉ ᛈ MouseEncoding {
    /// Default X10-style (coordinates + 32, max 223)
    X10,
    /// Mode 1005: UTF-8 encoding (coordinates as UTF-8)
    Utf8,
    /// Mode 1006: SGR encoding (CSI < ... M/m) - recommended
    Sgr,
    /// Mode 1015: URXVT encoding (CSI ... M)
    Urxvt,
}

☉ ᛈ TerminalMsg {
    /// Data from PTY
    PtyData { bytes: Vec<u8> },
    /// PTY exited
    PtyExit { code: i32 },
    /// Resize request
    Resize { rows: u16, cols: u16 },
    /// User input (key event)
    KeyInput { event: KeyEvent },
    /// Mouse event
    MouseInput { event: MouseEvent },
    /// Paste text
    Paste { text: String },
}

☉ actor Terminal {
    /// The primary screen grid
    state grid: Grid!,
    /// Alternate screen grid (for vim, etc.)
    state alt_grid: Option<Grid>!,
    /// Cursor state
    state cursor: Cursor!,
    /// Current text attributes
    state attrs: CellAttrs!,
    /// Character set state (G0/G1)
    state charset: CharsetState!,
    /// Terminal modes
    state modes: TerminalModes!,
    /// Tab stops (column positions)
    state tab_stops: Vec<u16>!,
    /// Escape sequence parser
    state parser: Parser!,
    /// PTY handle
    state pty: Pty!,
    /// Window title
    state title: String!,
    /// Icon name (may differ from title)
    state icon_name: String!,

    /// Create a new terminal
    ☉ rite spawn(config: TerminalConfig) -> Result<Self, Error>?;

    on PtyData { bytes } {
        ∀ byte ∈ bytes {
            ⌐ action = self.parser.parse(byte)? {
                self.execute(action);
            }
        }
    }

    on PtyExit { code } {
        // Handle shell exit
    }

    on Resize { rows, cols } {
        self.grid.resize(rows, cols);
        ⌐ alt = &mut self.alt_grid? {
            alt.resize(rows, cols);
        }
        self.pty.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
    }

    on KeyInput { event } {
        ≔ seq = self.encode_key(&event);
        self.pty.write(&seq);
    }

    on Paste { text } {
        ⎇ self.modes.bracketed_paste {
            self.pty.write(b"\x1b[200~");
            self.pty.write(text.as_bytes());
            self.pty.write(b"\x1b[201~");
        } ⎉ {
            self.pty.write(text.as_bytes());
        }
    }

    // =========================================================================
    // Action Execution
    // =========================================================================

    ☉ rite execute(&Δ self, action: TerminalAction);

    // C0 control handlers
    ☉ rite bell(&self);
    ☉ rite backspace(&Δ self);
    ☉ rite horizontal_tab(&Δ self);
    ☉ rite line_feed(&Δ self);
    ☉ rite carriage_return(&Δ self);

    // CSI handlers
    ☉ rite handle_csi(&Δ self, params: &[u16], inter: &[u8], final_byte: u8);
    ☉ rite cursor_up(&Δ self, count: u16);                    // CSI A
    ☉ rite cursor_down(&Δ self, count: u16);                  // CSI B
    ☉ rite cursor_forward(&Δ self, count: u16);               // CSI C
    ☉ rite cursor_back(&Δ self, count: u16);                  // CSI D
    ☉ rite cursor_next_line(&Δ self, count: u16);             // CSI E
    ☉ rite cursor_prev_line(&Δ self, count: u16);             // CSI F
    ☉ rite cursor_character_absolute(&Δ self, col: u16);      // CSI G
    ☉ rite cursor_position(&Δ self, row: u16, col: u16);      // CSI H
    ☉ rite erase_display(&Δ self, mode: EraseMode);           // CSI J
    ☉ rite erase_line(&Δ self, mode: EraseMode);              // CSI K
    ☉ rite insert_lines(&Δ self, count: u16);                 // CSI L
    ☉ rite delete_lines(&Δ self, count: u16);                 // CSI M
    ☉ rite delete_chars(&Δ self, count: u16);                 // CSI P
    ☉ rite scroll_up(&Δ self, count: u16);                    // CSI S
    ☉ rite scroll_down(&Δ self, count: u16);                  // CSI T
    ☉ rite erase_chars(&Δ self, count: u16);                  // CSI X
    ☉ rite insert_chars(&Δ self, count: u16);                 // CSI @
    ☉ rite repeat_char(&Δ self, count: u16);                  // CSI b
    ☉ rite device_attributes(&Δ self, params: &[u16]);        // CSI c
    ☉ rite line_position_absolute(&Δ self, row: u16);         // CSI d
    ☉ rite set_scroll_region(&Δ self, top: u16, bottom: u16); // CSI r
    ☉ rite save_cursor_ansi(&Δ self);                         // CSI s
    ☉ rite restore_cursor_ansi(&Δ self);                      // CSI u
    ☉ rite set_graphics_rendition(&Δ self, params: &[u16]);   // CSI m
    ☉ rite device_status_report(&Δ self, params: &[u16]);     // CSI n
    ☉ rite set_mode(&Δ self, mode: u16, private: bool);       // CSI h / CSI ? h
    ☉ rite reset_mode(&Δ self, mode: u16, private: bool);     // CSI l / CSI ? l
    ☉ rite window_manipulation(&Δ self, params: &[u16]);      // CSI t

    // ESC handlers
    ☉ rite handle_esc(&Δ self, inter: &[u8], final_byte: u8);
    ☉ rite save_cursor(&Δ self);                              // ESC 7
    ☉ rite restore_cursor(&Δ self);                           // ESC 8
    ☉ rite index(&Δ self);                                    // ESC D
    ☉ rite next_line(&Δ self);                                // ESC E
    ☉ rite tab_set(&Δ self);                                  // ESC H
    ☉ rite reverse_index(&Δ self);                            // ESC M
    ☉ rite application_keypad(&Δ self);                       // ESC =
    ☉ rite normal_keypad(&Δ self);                            // ESC >
    ☉ rite full_reset(&Δ self);                               // ESC c
    ☉ rite designate_charset(&Δ self, slot: u8, charset: Charset); // ESC ( / ESC )

    // OSC handlers
    ☉ rite handle_osc(&Δ self, params: &[Vec<u8>]);
    ☉ rite set_title(&Δ self, title: &str);
    ☉ rite set_icon_name(&Δ self, name: &str);
    ☉ rite set_color_palette(&Δ self, index: u8, color: Color);
    ☉ rite query_color(&self, index: u8) -> Option<Color>!;

    // Screen buffer switching
    ☉ rite switch_to_alternate(&Δ self);
    ☉ rite switch_to_primary(&Δ self);

    // =========================================================================
    // Queries
    // =========================================================================

    ☉ rite grid(&self) -> &Grid!;
    ☉ rite cursor(&self) -> &Cursor!;
    ☉ rite title(&self) -> &str!;
    ☉ rite is_alternate_screen(&self) -> bool!;
}
```

---

### 6. Scry Component

Renders the terminal to VNodes.

```sigil
// scry/src/component.sigil

☉ Σ ScryProps {
    /// Shell to run (None = default shell)
    ☉ shell: Option<String>,
    /// Initial rows
    ☉ rows: u16,
    /// Initial columns
    ☉ cols: u16,
    /// Font family
    ☉ font_family: String,
    /// Font size in pixels
    ☉ font_size: u16,
    /// Color scheme
    ☉ theme: ScryTheme,
    /// Scrollback limit
    ☉ scrollback_limit: usize,
    /// On title change callback
    ☉ on_title_change: Option<fn(String)>,
    /// On exit callback
    ☉ on_exit: Option<fn(i32)>,
}

☉ Σ ScryTheme {
    ☉ foreground: Color,
    ☉ background: Color,
    ☉ cursor: Color,
    ☉ selection: Color,
    /// ANSI colors 0-15
    ☉ ansi: [Color; 16],
}

⊢ ScryTheme {
    ☉ rite dark() -> Self!;
    ☉ rite light() -> Self!;
}

☉ actor ScryView {
    state scry: Scry!,
    state selection: Option<Selection>!,
    state scroll_offset: i32! = 0,
    state focused: bool! = false,
    state theme: ScryTheme!,
    state font_family: String!,
    state font_size: u16!,
    state cell_width: f32!,
    state cell_height: f32!,

    // Callbacks
    state on_title_change: Option<fn(String)>!,
    state on_exit: Option<fn(i32)>!,

    /// Create scry view
    ☉ rite spawn(props: ScryProps) -> Result<Self, Error>?;

    /// Main render function
    ☉ rite view(&self) -> VNode!;

    /// Render a single line
    ☉ rite render_line(&self, row: u16, line: &Line) -> VNode!;

    /// Build optimized spans (merge cells with same attributes)
    ☉ rite build_spans(&self, line: &Line, row: u16) -> Vec<VNode>!;

    /// Render cursor overlay
    ☉ rite render_cursor(&self) -> VNode!;

    /// Render selection overlay
    ☉ rite render_selection(&self) -> Option<VNode>!;

    /// Convert cell attributes to CSS
    ☉ rite attrs_to_style(&self, attrs: &CellAttrs) -> String!;

    /// Handle keyboard input
    ☉ rite handle_key(&self, event: KeyEvent);

    /// Handle mouse events
    ☉ rite handle_mouse(&Δ self, event: MouseEvent);

    /// Handle scroll
    ☉ rite handle_scroll(&Δ self, delta: i32);

    /// Copy selection to clipboard
    ☉ rite copy_selection(&self) -> Option<String>!;

    /// Paste from clipboard
    ☉ rite paste(&self, text: &str);

    /// Focus the terminal
    ☉ rite focus(&Δ self);

    /// Blur the terminal
    ☉ rite blur(&Δ self);
}
```

---

### 7. Selection Handling

```sigil
// terminal/src/selection.sigil

☉ Σ SelectionPoint {
    ☉ row: i32,  // Can be negative (scrollback)
    ☉ col: u16,
}

☉ ᛈ SelectionType {
    /// Character-by-character selection
    Normal,
    /// Select whole words
    Word,
    /// Select whole lines
    Line,
    /// Rectangular block selection
    Block,
}

☉ Σ Selection {
    ☉ start: SelectionPoint,
    ☉ end: SelectionPoint,
    ☉ selection_type: SelectionType,
}

⊢ Selection {
    /// Start a new selection
    ☉ rite new(start: SelectionPoint) -> Self!;

    /// Update selection endpoint
    ☉ rite update(&Δ self, point: SelectionPoint);

    /// Get normalized (start <= end) bounds
    ☉ rite normalize(&self) -> (SelectionPoint, SelectionPoint)!;

    /// Check if a cell is within selection
    ☉ rite contains(&self, row: i32, col: u16) -> bool!;

    /// Extract selected text from grid
    /// Handles wrapped lines correctly:
    /// - If line.wrapped == true, no newline inserted
    /// - Trims trailing whitespace from each line
    /// - For block selection, extracts rectangular region
    ☉ rite extract_text(&self, grid: &Grid) -> String!;

    /// Expand selection to word boundaries
    ☉ rite expand_to_word(&Δ self, grid: &Grid);

    /// Expand selection to line boundaries
    ☉ rite expand_to_line(&Δ self, grid: &Grid);

    /// Check if selection is empty (start == end)
    ☉ rite is_empty(&self) -> bool!;

    /// Clear the selection
    ☉ rite clear(&Δ self);
}
```

---

### 8. Scrollback Search

```sigil
// terminal/src/search.sigil

/// Search direction
☉ ᛈ SearchDirection {
    Forward,
    Backward,
}

/// A search match in the terminal
☉ Σ SearchMatch {
    /// Line index (can be negative for scrollback)
    ☉ line: i32,
    /// Start column
    ☉ start_col: u16,
    /// End column (exclusive)
    ☉ end_col: u16,
}

/// Search state
☉ Σ SearchState {
    /// Current search pattern (regex)
    ☉ pattern: Option<String>,
    /// Compiled regex
    ☉ regex: Option<Regex>,
    /// All matches in scrollback + visible
    ☉ matches: Vec<SearchMatch>,
    /// Currently focused match index
    ☉ current_match: Option<usize>,
    /// Case sensitivity
    ☉ case_sensitive: bool,
    /// Use regex or literal
    ☉ use_regex: bool,
}

⊢ SearchState {
    ☉ rite new() -> Self!;

    /// Start a new search, returns match count
    ☉ rite search(&Δ self, pattern: &str, grid: &Grid) -> usize!;

    /// Clear search state
    ☉ rite clear(&Δ self);

    /// Jump to next match
    ☉ rite next_match(&Δ self) -> Option<&SearchMatch>!;

    /// Jump to previous match
    ☉ rite prev_match(&Δ self) -> Option<&SearchMatch>!;

    /// Jump to specific match by index
    ☉ rite goto_match(&Δ self, index: usize) -> Option<&SearchMatch>!;

    /// Get current match
    ☉ rite current(&self) -> Option<&SearchMatch>!;

    /// Get all matches
    ☉ rite matches(&self) -> &[SearchMatch]!;

    /// Check if a cell is part of any match (for highlighting)
    ☉ rite is_match(&self, line: i32, col: u16) -> bool!;

    /// Check if a cell is the current/focused match
    ☉ rite is_current_match(&self, line: i32, col: u16) -> bool!;

    /// Update search when grid content changes
    ☉ rite refresh(&Δ self, grid: &Grid);
}

/// Incremental search (search-as-you-type)
☉ Σ IncrementalSearch {
    ☉ state: SearchState,
    ☉ input_buffer: String,
    ☉ original_scroll_offset: i32,
}

⊢ IncrementalSearch {
    /// Start incremental search, save current scroll position
    ☉ rite start(&Δ self, scroll_offset: i32);

    /// Add character to search pattern
    ☉ rite push_char(&Δ self, ch: char, grid: &Grid);

    /// Remove character from search pattern
    ☉ rite pop_char(&Δ self, grid: &Grid);

    /// Confirm search and stay at current match
    ☉ rite confirm(&Δ self) -> Option<SearchMatch>!;

    /// Cancel search and return to original scroll position
    ☉ rite cancel(&Δ self) -> i32!;
}
```

---

### 9. Key Encoding

```sigil
// terminal/src/keys.sigil

☉ Σ KeyEvent {
    ☉ key: Key,
    ☉ modifiers: Modifiers,
}

☉ Σ Modifiers {
    ☉ ctrl: bool,
    ☉ alt: bool,
    ☉ shift: bool,
    ☉ meta: bool,
}

☉ ᛈ Key {
    /// Printable character
    Char(char),

    // === Control Keys ===
    Enter,
    Backspace,
    Tab,
    Escape,

    // === Cursor Keys ===
    Up,
    Down,
    Left,
    Right,

    // === Navigation Keys ===
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,

    // === Function Keys ===
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    F13, F14, F15, F16, F17, F18, F19, F20,

    // === Keypad Keys (for application mode) ===
    KpEnter,
    KpPlus,
    KpMinus,
    KpMultiply,
    KpDivide,
    KpDecimal,
    Kp0, Kp1, Kp2, Kp3, Kp4, Kp5, Kp6, Kp7, Kp8, Kp9,
}

/// Encode a key event as escape sequence for PTY
☉ rite encode_key(event: &KeyEvent, modes: &TerminalModes) -> Vec<u8>!;

/// Encode function key (F1-F20)
☉ rite encode_function_key(num: u8, modifiers: &Modifiers) -> Vec<u8>!;

/// Encode cursor key (with DECCKM handling)
☉ rite encode_cursor_key(key: &Key, modes: &TerminalModes, modifiers: &Modifiers) -> Vec<u8>!;

/// Encode keypad key (with application keypad mode handling)
☉ rite encode_keypad_key(key: &Key, modes: &TerminalModes) -> Vec<u8>!;

/// Encode Ctrl+key combination
☉ rite encode_ctrl_key(ch: char) -> Option<u8>!;
```

---

### 10. Mouse Events

```sigil
// terminal/src/mouse.sigil

/// Mouse button
☉ ᛈ MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
    /// No button (for motion events)
    None,
    /// Additional buttons (button 4+)
    Extra(u8),
}

/// Mouse event type
☉ ᛈ MouseEventKind {
    /// Button pressed
    Press,
    /// Button released
    Release,
    /// Mouse moved (no button)
    Move,
    /// Mouse dragged (button held)
    Drag,
}

/// A mouse event
☉ Σ MouseEvent {
    /// Event type
    ☉ kind: MouseEventKind,
    /// Button involved (if any)
    ☉ button: MouseButton,
    /// Row (0-indexed, can be in scrollback)
    ☉ row: i32,
    /// Column (0-indexed)
    ☉ col: u16,
    /// Modifier keys held
    ☉ modifiers: Modifiers,
}

⊢ MouseEvent {
    /// Create a press event
    ☉ rite press(button: MouseButton, row: i32, col: u16, modifiers: Modifiers) -> Self!;

    /// Create a release event
    ☉ rite release(button: MouseButton, row: i32, col: u16, modifiers: Modifiers) -> Self!;

    /// Create a move event
    ☉ rite move_to(row: i32, col: u16, modifiers: Modifiers) -> Self!;

    /// Create a drag event
    ☉ rite drag(button: MouseButton, row: i32, col: u16, modifiers: Modifiers) -> Self!;
}

/// Encode mouse event for PTY based on current mode
☉ rite encode_mouse(
    event: &MouseEvent,
    mode: MouseMode,
    encoding: MouseEncoding,
) -> Option<Vec<u8>>! {
    // Only encode if mode allows this event type
    ⌥ mode {
        MouseMode·None => ↩ None,
        MouseMode·X10 => {
            // X10: only button press
            ⎇ event.kind ≠ MouseEventKind·Press { ↩ None; }
        }
        MouseMode·Normal => {
            // Normal: press and release
            ⎇ matches!(event.kind, MouseEventKind·Move) { ↩ None; }
        }
        MouseMode·ButtonMotion => {
            // Button motion: press, release, drag
            ⎇ matches!(event.kind, MouseEventKind·Move) { ↩ None; }
        }
        MouseMode·AnyMotion => {
            // Any motion: all events
        }
    }

    ⌥ encoding {
        MouseEncoding·X10 => encode_mouse_x10(event),
        MouseEncoding·Utf8 => encode_mouse_utf8(event),
        MouseEncoding·Sgr => encode_mouse_sgr(event),
        MouseEncoding·Urxvt => encode_mouse_urxvt(event),
    }
}

/// X10 encoding: CSI M Cb Cx Cy (coordinates + 32, max 223)
☉ rite encode_mouse_x10(event: &MouseEvent) -> Option<Vec<u8>>!;

/// UTF-8 encoding: like X10 but coordinates as UTF-8
☉ rite encode_mouse_utf8(event: &MouseEvent) -> Option<Vec<u8>>!;

/// SGR encoding: CSI < Cb ; Cx ; Cy M/m (recommended, no limits)
☉ rite encode_mouse_sgr(event: &MouseEvent) -> Option<Vec<u8>>!;

/// URXVT encoding: CSI Cb ; Cx ; Cy M
☉ rite encode_mouse_urxvt(event: &MouseEvent) -> Option<Vec<u8>>!;
```

---

## SGR (Select Graphic Rendition) Support

Full support for text styling via CSI m sequences:

| Code | Effect |
|------|--------|
| 0 | Reset all attributes |
| 1 | Bold |
| 2 | Dim |
| 3 | Italic |
| 4 | Underline |
| 5 | Blink (slow) |
| 7 | Inverse |
| 8 | Hidden |
| 9 | Strikethrough |
| 21 | Double underline |
| 22 | Normal intensity |
| 23 | Not italic |
| 24 | Not underlined |
| 25 | Not blinking |
| 27 | Not inverse |
| 28 | Not hidden |
| 29 | Not strikethrough |
| 30-37 | Foreground color (ANSI) |
| 38;5;n | Foreground 256-color |
| 38;2;r;g;b | Foreground RGB |
| 39 | Default foreground |
| 40-47 | Background color (ANSI) |
| 48;5;n | Background 256-color |
| 48;2;r;g;b | Background RGB |
| 49 | Default background |
| 90-97 | Bright foreground |
| 100-107 | Bright background |

---

## DEC Private Modes

| Mode | Name | Description | Default |
|------|------|-------------|---------|
| 1 | DECCKM | Application cursor keys | Off |
| 3 | DECCOLM | 132 column mode (vs 80) | Off |
| 5 | DECSCNM | Reverse video (swap fg/bg) | Off |
| 6 | DECOM | Origin mode (relative to scroll region) | Off |
| 7 | DECAWM | Auto-wrap mode | **On** |
| 9 | X10 Mouse | X10 mouse compatibility | Off |
| 12 | ATT610 | Cursor blink | On |
| 25 | DECTCEM | Cursor visible | **On** |
| 47 | - | Alternate screen buffer (old) | Off |
| 1000 | - | Normal mouse tracking (press/release) | Off |
| 1002 | - | Button-event mouse tracking | Off |
| 1003 | - | Any-event mouse tracking | Off |
| 1004 | - | Focus tracking (send focus in/out) | Off |
| 1005 | - | UTF-8 mouse encoding | Off |
| 1006 | - | SGR mouse encoding (recommended) | Off |
| 1015 | - | URXVT mouse encoding | Off |
| 1047 | - | Alternate screen buffer | Off |
| 1048 | - | Save/restore cursor | Off |
| 1049 | - | Alternate screen + save cursor | Off |
| 2004 | - | Bracketed paste mode | Off |

**Notes:**
- Mode 1049 is the most common: it saves cursor, switches to alt screen, and clears it
- SGR mouse (1006) is preferred as it has no coordinate limits
- DECAWM (7) defaults ON per DEC standard

---

## Performance Considerations

### Rendering Optimization

1. **Span Merging**: Adjacent cells with identical attributes render as single `<span>`
2. **Dirty Tracking**: Only re-render lines that changed
3. **Virtual Scrolling**: Only render visible portion of scrollback
4. **Batch Updates**: Coalesce rapid PTY data into single render pass

### Memory

1. **Scrollback Limit**: Default 10,000 lines, configurable
2. **Line Compaction**: Empty trailing cells not stored
3. **Attribute Interning**: Common attribute combinations shared

---

## Testing Strategy

### Unit Tests

- Parser state machine transitions
- CSI sequence parameter parsing
- SGR color parsing (ANSI, 256, RGB)
- Grid operations (scroll, erase, resize)
- Key encoding for various modes

### Integration Tests

- Round-trip: input → PTY → output → grid
- Mode switching (alternate screen)
- Scrollback behavior
- Selection across wrapped lines

### Compatibility Tests

- Run `vttest` terminal test suite
- Test common applications: bash, vim, htop, tmux
- Unicode handling (wide chars, combining marks)

---

## Shell Integration (OSC 133)

Shell integration allows the terminal to understand command structure:

```
┌─────────────────────────────────────────────────────────────┐
│ $ ls -la                                                    │
│ ↑                                                           │
│ OSC 133;A (prompt start)                                    │
│         ↑                                                   │
│         OSC 133;B (command start - user types here)         │
│                                                             │
│ total 42                                                    │
│ ↑                                                           │
│ OSC 133;C (command executed - output begins)                │
│ drwxr-xr-x  5 user user  4096 Feb 17 10:00 .                │
│ ...                                                         │
│                                                             │
│ OSC 133;D;0 (command finished, exit code 0)                 │
└─────────────────────────────────────────────────────────────┘
```

### Implementation

```sigil
// terminal/src/shell_integration.sigil

/// A command region in the terminal
☉ Σ CommandRegion {
    /// Line where prompt starts
    ☉ prompt_start: i32,
    /// Line where command input starts
    ☉ command_start: i32,
    /// Line where output starts
    ☉ output_start: Option<i32>,
    /// Line where command finished
    ☉ output_end: Option<i32>,
    /// Exit code (if finished)
    ☉ exit_code: Option<i32>,
    /// The command text (if captured)
    ☉ command: Option<String>,
}

☉ Σ ShellIntegrationState {
    /// All command regions (most recent last)
    ☉ regions: Vec<CommandRegion>,
    /// Currently active region (being built)
    ☉ current: Option<CommandRegion>,
    /// Current working directory (from OSC 7)
    ☉ cwd: Option<String>,
}

⊢ ShellIntegrationState {
    /// Handle OSC 133 sequence
    ☉ rite handle_osc_133(&Δ self, param: char, arg: Option<&str>, current_line: i32);

    /// Handle OSC 7 (working directory)
    ☉ rite handle_osc_7(&Δ self, uri: &str);

    /// Get command region at line
    ☉ rite region_at(&self, line: i32) -> Option<&CommandRegion>!;

    /// Get previous command region
    ☉ rite prev_command(&self) -> Option<&CommandRegion>!;

    /// Get next command region
    ☉ rite next_command(&self) -> Option<&CommandRegion>!;

    /// Jump to prompt N commands back
    ☉ rite jump_to_prompt(&self, offset: i32) -> Option<i32>!;
}
```

### Shell Configuration

Shells must be configured to emit these sequences. Example for bash/zsh:

```bash
# Add to .bashrc / .zshrc
PS1='\[\e]133;A\a\]'$PS1'\[\e]133;B\a\]'
precmd() { echo -ne '\e]133;D;'$?'\a' }
preexec() { echo -ne '\e]133;C\a' }
```

---

## Synchronized Output

Prevents tearing during fast output by batching updates:

```sigil
// In Terminal actor

☉ Σ SyncState {
    /// Currently in synchronized update
    ☉ active: bool,
    /// Pending render
    ☉ pending_render: bool,
}

⊢ Terminal {
    /// Handle DCS for synchronized updates
    ☉ rite handle_sync_update(&Δ self, mode: u8) {
        ⌥ mode {
            1 => {
                // Begin synchronized update - suppress renders
                self.sync.active = true;
            }
            2 => {
                // End synchronized update - trigger render
                self.sync.active = false;
                ⎇ self.sync.pending_render {
                    self.trigger_render();
                    self.sync.pending_render = false;
                }
            }
            _ => {}
        }
    }

    /// Check if render should be suppressed
    ☉ rite should_render(&self) -> bool! {
        ¬self.sync.active
    }
}
```

**Sequences:**
- `DCS = 1 s ST` or `ESC P = 1 s ESC \` — Begin synchronized update
- `DCS = 2 s ST` or `ESC P = 2 s ESC \` — End synchronized update

Used by: tmux, vim, other TUI applications for flicker-free updates.

---

## Future Considerations

- **Sixel Graphics**: Image display in terminal
- **Kitty Graphics Protocol**: Modern image protocol
- **Ligatures**: If font supports (coordinate with Athame)
- **Reflow on Resize**: Unwrap/rewrap lines when terminal width changes (complex)

---

## Estimated Scope

| Module | Lines of Sigil | Complexity | Notes |
|--------|----------------|------------|-------|
| `pty.sigil` | ~150 | Medium | Platform FFI via qliphoth-sys |
| `parser.sigil` | ~500 | Medium | VT500 state machine + UTF-8 |
| `grid.sigil` | ~250 | Low | Data structures |
| `cell.sigil` | ~100 | Low | Cell/attribute types |
| `cursor.sigil` | ~180 | Low | Cursor state + DECSCUSR |
| `charset.sigil` | ~120 | Low | DEC graphics mapping |
| `modes.sigil` | ~100 | Low | Mode flags |
| `interpreter.sigil` | ~800 | High | CSI/ESC/OSC handlers |
| `component.sigil` | ~450 | Medium | VNode rendering + hyperlinks |
| `selection.sigil` | ~200 | Low | Text selection |
| `search.sigil` | ~250 | Medium | Regex search + incremental |
| `keys.sigil` | ~220 | Medium | Key encoding |
| `mouse.sigil` | ~200 | Medium | Mouse encoding (4 formats) |
| `shell_integration.sigil` | ~180 | Medium | OSC 7/133 command tracking |
| **Total** | **~3,700** | | |

This is a feature-complete terminal emulator with:
- Full shell integration (OSC 133)
- Hyperlinks (OSC 8)
- Synchronized output
- Incremental scrollback search
- All cursor shapes (DECSCUSR)
- Full mouse support with SGR encoding

Still ~4x smaller than `alacritty_terminal` (~15,000 lines) because we're not implementing:
- Sixel/Kitty graphics
- Full VT520 compliance
- Reflow on resize (future)
- Legacy hardware terminal modes

---

## References

- [VT100 User Guide](https://vt100.net/docs/vt100-ug/)
- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
- [Paul Williams' Parser](https://vt100.net/emu/dec_ansi_parser)
- [Alacritty VTE](https://github.com/alacritty/vte)
- [Kitty Terminal](https://sw.kovidgoyal.net/kitty/)
