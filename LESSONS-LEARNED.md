# Lessons Learned

This file captures organizational memory for the Qliphoth project.
See `~/dev2/workspace/docs/methodologies/PROJECT-WELLNESS.md` for methodology.

---

## 2026-02-14 - WASM Backend Dependency Resolution

### Context
Attempting to compile qliphoth-router to WASM. The router depends on qliphoth
core, which should be compiled first.

### What Happened
Dependency resolution failed due to:
1. File naming: `sigil.toml` vs `Sigil.toml` (case sensitivity)
2. Section naming: `[package]` vs `[project]` in manifest
3. Parser expecting commas between struct fields (Sigil uses comma-less syntax)

### Root Cause
The Sigil compiler was built assuming Rust conventions (lowercase filenames,
`[package]` sections, comma-separated fields) rather than native Sigil
conventions.

### Lesson
Sigil has its own conventions that differ from Rust:
- Manifest can be `Sigil.toml` or `sigil.toml`
- Sections can be `[project]` or `[package]`
- Struct fields don't require commas
- `state` is a contextual keyword (only in actors)

### Prevention
When adding new parser/compiler features, check against actual Sigil codebases
(qliphoth, sigil-lang itself) not just Rust assumptions.

---

## 2026-02-14 - Actor Syntax Requirements

### Context
Parsing qliphoth core's `AppRuntime` actor definition.

### What Happened
Parser failed on:
1. `state field: Type = default` syntax (state as keyword prefix)
2. `on Message { }` without parentheses for no-param handlers
3. Methods (`rite`) inside actor bodies

### Root Cause
Actor syntax evolved independently of the parser implementation. The parser
had minimal actor support that didn't match real usage.

### Lesson
Actor definitions can contain:
- State fields with optional `state` prefix
- Message handlers with optional parameter parens
- Method definitions (same as impl blocks)
- Doc comments before any member

### Prevention
When implementing a language feature, check existing codebases for actual
usage patterns before finalizing the parser.

---

## 2026-02-14 - Nested Module Resolution

### Context
Attempting to compile qliphoth core which uses directory-style modules (`core/mod.sigil`)
with sibling modules (`core/vdom.sigil`, `core/events.sigil`).

### What Happened
Module resolution failed with:
```
module 'vdom' not found, tried: ./src/vdom.sigil, ./src/vdom/mod.sigil
```

The compiler was looking for `vdom` at the crate root instead of inside `core/`.

### Root Cause
When entering a directory-based module (`foo/mod.sigil`), the compiler's `source_dir`
wasn't being updated to point to that directory. Sibling module lookups used the
parent directory instead of the current module's directory.

### Fix
In `wasm/statements.rs`, all functions that process file-based modules now:
1. Load the module file using current `source_dir`
2. Update `source_dir` to the module's directory (for `foo/mod.sigil`, use `foo/`)
3. Process child items with updated `source_dir`
4. Restore previous `source_dir` when done

Key functions fixed: `collect_use_declarations`, `collect_all_type_defs`,
`prescan_all_functions`, `load_and_collect_module_sigs`, `compile_module`.

### Lesson
Directory-based modules establish a new resolution context. When `core/mod.sigil`
declares `☉ scroll vdom;`, it should look for `core/vdom.sigil` not `src/vdom.sigil`.

### Prevention
Module resolution logic must track and update the "current directory" context as
it descends into nested modules.

---

## 2026-02-14 - DSL View Syntax Discovery

### Context
Attempting to compile qliphoth core with all parser fixes applied.

### What Happened
Compilation failed on test code containing:
```sigil
div { "Hello, World!" }
```

The parser interprets `div { ... }` as a struct literal, expecting field names.

### Root Cause
Qliphoth uses a DSL for view templates similar to JSX, but the parser has no
awareness of this pattern. There's no way to distinguish:
- `MyStruct { field: value }` (struct literal)
- `div { "content" }` (view element)

### Lesson
View DSL syntax requires:
1. Element detection (recognizing `div`, `span`, etc. as elements)
2. Content parsing (children can be text, expressions, nested elements)
3. Attribute syntax (props on elements)

This is a substantial feature, not a simple parser fix. After critical review,
DSL was deferred as it primarily benefits human ergonomics over agent ergonomics.
The DSL test in `core/mod.sigil` was commented out with a TODO reference.

### Prevention
Before marking a package "compiles to WASM," verify all source files parse
correctly, including test modules.

---

## 2026-02-14 - Advanced Type Syntax Gaps

### Context
With module resolution fixed, compilation progresses into deeper modules like
`hooks/mod.sigil` which use advanced type syntax.

### What Happened
Parser fails on function signatures like:
```sigil
rite use_effect<D: PartialEq + Clone + 'static>(
    effect: rite() -> Option<rite()>?,
    deps: Vec<D>
)!
```

### Root Cause
Several advanced type syntax features aren't fully supported:
1. Function types as parameters: `rite() -> T`
2. Trait bounds with lifetimes: `'static`
3. Multiple trait bounds: `PartialEq + Clone`

### Lesson
The qliphoth framework uses Rust-inspired advanced generics that require
additional parser work. Current WASM compilation is limited to simpler
packages like `qliphoth-sys`.

### Current Status
- `qliphoth-sys`: Compiles to WASM (20.7 KB)
- `qliphoth core`: Blocked on hooks advanced syntax
- `qliphoth-router`: Blocked on hooks advanced syntax

### Resolution
Rather than implementing these features, the entire architecture was redesigned.
See "Agent-Centric Qliphoth Redesign" below.

---

## 2026-02-14 - Agent-Centric Qliphoth Redesign

### Context
Attempting to add advanced type syntax to support qliphoth's React-style hooks.

### What Happened
Critical question asked: "Would these features be meaningful to agents?"

Analysis revealed:
- **Function types**: Needed for callbacks, but actors use messages, not callbacks
- **Lifetimes**: Rust borrow checker concept, WASM doesn't need this
- **Trait bounds**: Needed for complex generics in hooks, but hooks are being removed

### Root Cause
Qliphoth was cargo-culting React patterns designed for JavaScript's limitations:
- Hooks exist because early React had no classes → Sigil has actors
- Callbacks exist because JS is callback-native → Sigil has messages
- JSX/DSL exists to reduce typing → Agents generate code, don't type
- Context exists to avoid prop drilling → Actors receive deps via constructor

### Resolution
Complete architectural redesign around agent-centric principles:

1. **Components are actors** with state fields, message handlers, and view method
2. **Events dispatch messages**, not callbacks
3. **Views use builder pattern**, not DSL syntax
4. **Evidentiality markers** replace loading boolean states
5. **Delete hooks/ entirely** - they were JavaScript workarounds

### Lesson
When hitting complexity, ask: "Is this solving our problem or cargo-culting
solutions to someone else's problem?" React solves JavaScript problems.
Sigil doesn't have JavaScript's problems.

### Spec
See `docs/specs/AGENT-CENTRIC-QLIPHOTH.md` for the complete redesigned architecture.

---

## 2026-02-14 - WASM Compilation Success

### Context
Implementing the agent-centric redesign and achieving WASM compilation.

### What Happened
Successfully compiled qliphoth to WASM (38.9 KB) after extensive refactoring to
avoid unsupported features.

### Unsupported Features Discovered
The WASM backend does not support:
1. Function references/closures: `|τ{x => ...}`, `.map(func)`, `retain(|x| ...)`
2. `unsafe` blocks
3. `matches!` macro
4. `.as_str()`, `.copied()`, `.extend()` methods
5. Function types as parameters: `⊢ Fn() + 'static`
6. `Box<dyn Fn()>` trait objects

### Resolution
All these features were replaced with:
1. Explicit `∀` for loops instead of iterator chains with closures
2. Stub implementations for FFI (runtime provides real bindings)
3. Explicit comparisons instead of `matches!`
4. Match expressions instead of `.copied()`
5. Message IDs (u64) instead of callback functions
6. Simple data types instead of trait objects

### Lesson
The agent-centric design naturally avoids most unsupported features. The paradigm
shift from callbacks to message IDs wasn't just philosophical—it made the code
compilable.

### Files Changed
- `hooks/` directory: DELETED
- `core/vdom.sigil`: Rewrote with builder pattern (~540 lines)
- `core/mod.sigil`: Simplified for actor components (~230 lines)
- `core/events.sigil`: Message ID handlers instead of callbacks
- `dom/mod.sigil`: Removed callback-based helpers
- `platform/mod.sigil`: Stubbed FFI functions
- `lib.sigil`: Updated exports

### Prevention
When writing WASM-compilable Sigil:
- Use explicit for loops, not iterator chains with closures
- Use message IDs, not callback functions
- Use match expressions, not `.copied()` or `.map(fn)`
- Stub FFI functions for compilation; runtime provides real implementations

---
