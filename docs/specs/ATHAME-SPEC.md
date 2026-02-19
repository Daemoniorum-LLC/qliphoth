# Athame - Sigil Code Editor Specification

> *"A ritual blade for inscribing sigils"*

**Version:** 1.0.0
**Date:** 2026-01-31
**Status:** Draft (extracted from implementation)
**Component:** `qliphoth/components/athame/`

---

## 1. Overview

Athame is a code editor component for Sigil, designed for embedding in web applications (via Qliphoth) and the sigil-lang.com playground. It provides syntax highlighting, basic editing operations, and undo/redo history.

### 1.1 Design Goals

1. **Native Sigil Implementation** — Written in Sigil, compiles to WASM
2. **Syntax-Aware** — Tokenizer understands Sigil's unique syntax (morphemes, evidentiality, native symbols)
3. **Embeddable** — Works as a Qliphoth component or standalone
4. **Lightweight** — Minimal dependencies, fast load times

### 1.2 Architecture

```
athame/
├── src/
│   ├── mod.sigil        # Module exports
│   ├── tokenizer.sigil  # Lexer for syntax highlighting
│   ├── state.sigil      # Editor state management
│   ├── history.sigil    # Undo/redo stack
│   ├── viewport.sigil   # Visible line range calculation
│   ├── highlight.sigil  # Syntax highlighting spans
│   ├── editor.sigil     # Core editor logic
│   ├── component.sigil  # Qliphoth component wrapper
│   └── playground.sigil # Playground-specific integration
└── tests/
    ├── tokenizer_test.sigil
    ├── state_test.sigil
    ├── history_test.sigil
    └── rendering_test.sigil
```

---

## 2. Tokenizer Specification

The tokenizer lexes Sigil source code into tokens for syntax highlighting.

### 2.1 Token Types

```sigil
enum TokenKind {
    Keyword,           // Language keywords
    Identifier,        // Variable/function names
    Type,              // Type names (capitalized identifiers)
    String,            // String literals
    Number,            // Numeric literals
    Comment,           // Line and block comments
    Operator,          // Arithmetic, comparison, logical operators
    Morpheme,          // Greek letter pipeline operators
    NativeSymbol,      // Sigil-specific symbols (≔, ⎇, ⟳, etc.)
    EvidenceKnown,     // ! marker after identifier
    EvidenceUncertain, // ? marker after identifier
    EvidenceReported,  // ~ marker after identifier
    EvidenceParadox,   // ‽ interrobang
    Punctuation,       // Brackets, commas, semicolons
    Whitespace,        // Spaces, tabs
    Newline,           // Line breaks
    Unknown,           // Unrecognized characters
}
```

### 2.2 Keywords

The tokenizer recognizes both legacy (Rust-compatible) and native Sigil keywords:

**Legacy Keywords (aliases):**
```
fn, let, mut, if, else, match, return, for, while, in,
struct, enum, trait, impl, use, pub, async, await,
true, false, self, Self, super, loop, break, continue,
const, where
```

**Native Sigil Keywords:**
```
rite, sigil, aspect, vary, yea, nay, each, of,
forever, this, This, above, invoke, scroll, tome
```

### 2.3 Morpheme Characters

Greek letters used as semantic pipeline operators:

| Character | Unicode | Name | Operation |
|-----------|---------|------|-----------|
| τ | U+03C4 | tau | Transform (map) |
| φ | U+03C6 | phi | Filter |
| σ | U+03C3 | sigma | Sort |
| ρ | U+03C1 | rho | Reduce |
| Σ | U+03A3 | Sigma | Sum |
| Π | U+03A0 | Pi | Product |
| α | U+03B1 | alpha | First |
| ω | U+03C9 | omega | Last |
| μ | U+03BC | mu | Merge |
| λ | U+03BB | lambda | Lambda/function |

**Uppercase variants also recognized:** Τ, Φ, Ρ, Α, Ω, Μ, Λ, Θ

### 2.4 Native Symbols

Sigil-specific Unicode symbols for control flow and structure:

| Symbol | Unicode | Meaning |
|--------|---------|---------|
| ≔ | U+2254 | Assignment (let) |
| Δ | U+0394 | Mutable modifier |
| ⎇ | U+2387 | If |
| ⎉ | U+2389 | Else |
| ⌥ | U+2325 | Match |
| ⟳ | U+27F3 | While loop |
| ∞ | U+221E | Forever loop |
| ∀ | U+2200 | For all |
| ∈ | U+2208 | In (membership) |
| ∧ | U+2227 | Logical AND |
| ∨ | U+2228 | Logical OR |
| ¬ | U+00AC | Logical NOT |
| ☉ | U+2609 | Public export |
| ᛈ | U+16C8 | Enum declaration |
| · | U+00B7 | Module separator |
| → | U+2192 | Return type arrow |

### 2.5 Evidentiality Markers

Context-sensitive tokenization for evidentiality:

- `!` after identifier/type → `EvidenceKnown`
- `?` after identifier/type → `EvidenceUncertain`
- `~` after identifier/type → `EvidenceReported`
- `‽` (U+203D) anywhere → `EvidenceParadox`
- `!`, `?`, `~` in other contexts → `Operator`

### 2.6 Comments

- **Line comments:** `// ...` until newline
- **Block comments:** `/* ... */` with nesting support

### 2.7 Two-Character Operators

```
==, !=, <=, >=, &&, ||, ->, =>,
+=, -=, *=, /=, ::
```

---

## 3. Editor State

### 3.1 State Structure

```sigil
sigil EditorState {
    content: String,           // Full document text
    cursor_pos: i64,           // Cursor position (character offset)
    selection: OptionSelection, // Optional selection range
    scroll_top: i64,           // Scroll position (pixels)
    line_height: i64,          // Line height for calculations
    viewport_height: i64,      // Visible area height
}
```

### 3.2 Selection

```sigil
sigil Selection {
    start: i64,  // Selection start offset
    end: i64,    // Selection end offset
}
```

### 3.3 Signals

Reactive state updates via signals:

```sigil
sigil Signal<T> {
    value: T,
    subscribers: Vec<fn(T)>,
}
```

---

## 4. History (Undo/Redo)

### 4.1 History Entry

```sigil
sigil HistoryEntry {
    content: String,     // Document state
    cursor_pos: i64,     // Cursor position at this state
    timestamp: i64,      // When this entry was created
}
```

### 4.2 History Stack

```sigil
sigil HistoryStack {
    entries: Vec<HistoryEntry>,
    current_index: i64,
    max_entries: i64,    // Default: 100
}
```

### 4.3 Operations

| Operation | Behavior |
|-----------|----------|
| `push(entry)` | Add new entry, truncate redo stack |
| `undo()` | Move to previous entry if available |
| `redo()` | Move to next entry if available |
| `can_undo()` | Returns true if undo available |
| `can_redo()` | Returns true if redo available |

---

## 5. Editor API

### 5.1 Constructor

```sigil
athame_new() → Athame
athame_with_content(content: String) → Athame
```

### 5.2 Content Operations

```sigil
athame_get_content(editor: &Athame) → String
athame_set_content(editor: &mut Athame, content: String)
athame_insert(editor: &mut Athame, text: String)
athame_delete_backward(editor: &mut Athame)
athame_delete_forward(editor: &mut Athame)
athame_newline(editor: &mut Athame)
athame_tab(editor: &mut Athame)  // Inserts 4 spaces
```

### 5.3 Cursor Movement

```sigil
athame_move_left(editor: &mut Athame)
athame_move_right(editor: &mut Athame)
athame_move_home(editor: &mut Athame)  // Start of line
athame_move_end(editor: &mut Athame)   // End of line
```

### 5.4 History

```sigil
athame_undo(editor: &mut Athame) → bool
athame_redo(editor: &mut Athame) → bool
athame_can_undo(editor: &Athame) → bool
athame_can_redo(editor: &Athame) → bool
```

### 5.5 Viewport

```sigil
athame_scroll_to(editor: &mut Athame, position: i64)
athame_scroll_by(editor: &mut Athame, delta: i64)
athame_ensure_cursor_visible(editor: &mut Athame)
athame_get_visible_range(editor: &Athame) → LineRange
athame_get_render_range(editor: &Athame) → LineRange  // With overscan
athame_get_cursor_line(editor: &Athame) → i64
```

### 5.6 Syntax Features

```sigil
athame_find_matching_bracket(editor: &Athame) → OptionBracketPair
athame_get_line_highlights(editor: &Athame, line: i64) → Vec<HighlightSpan>
```

### 5.7 Keyboard Handling

```sigil
athame_handle_key(editor: &mut Athame, event: KeyEvent) → bool
```

---

## 6. Syntax Highlighting CSS Classes

For rendering highlighted tokens, use these CSS class mappings:

| TokenKind | CSS Class |
|-----------|-----------|
| Keyword | `ath-keyword` |
| Type | `ath-type` |
| String | `ath-string` |
| Number | `ath-number` |
| Comment | `ath-comment` |
| Operator | `ath-operator` |
| Morpheme | `ath-morpheme` |
| NativeSymbol | `ath-native` |
| EvidenceKnown | `ath-evidence-known` |
| EvidenceUncertain | `ath-evidence-uncertain` |
| EvidenceReported | `ath-evidence-reported` |
| EvidenceParadox | `ath-evidence-paradox` |
| Identifier | (no class, default text) |
| Punctuation | (no class, default text) |
| Whitespace | (preserved as-is) |
| Newline | (preserved as-is) |
| Unknown | (no class, default text) |

---

## 7. Qliphoth Component

### 7.1 Component Props

```sigil
sigil AthameProps {
    initial_content: String,
    on_change: OptionCallback,  // fn(String)
    read_only: bool,
    line_numbers: bool,
    highlight_active_line: bool,
}
```

### 7.2 Component Usage

```sigil
use athame::component::AthameEditor;

fn CodeEditor() -> Element {
    AthameEditor {
        initial_content: "// Hello, Sigil!",
        on_change: Some(|content| save_draft(content)),
        read_only: false,
        line_numbers: true,
        highlight_active_line: true,
    }
}
```

---

## 8. Testing Requirements

### 8.1 Tokenizer Tests

- All keywords tokenize as `Keyword`
- Capitalized identifiers tokenize as `Type`
- All morpheme characters tokenize as `Morpheme`
- All native symbols tokenize as `NativeSymbol`
- Evidentiality markers context-sensitive
- Nested block comments handled correctly
- Escape sequences in strings preserved

### 8.2 Editor Tests

- Insert/delete operations update content correctly
- Cursor movement respects line boundaries
- Undo/redo restores previous states
- Tab inserts 4 spaces (not tab character)

### 8.3 Integration Tests

- Qliphoth component renders correctly
- Keyboard events handled properly
- Scroll sync between textarea and highlight layer

---

## 9. Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2026-01-31 | Claude Opus 4.5 | Initial spec extracted from implementation |

---

*Athame: From the ceremonial blade used to direct energy in ritual magic.
In Sigil, it directs the flow of code.*
